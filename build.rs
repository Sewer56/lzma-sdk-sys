#![feature(core_intrinsics)]

use core::time::Duration;
use std::{collections::HashMap, env, fs, path::{Path, PathBuf}, process::Command, thread::sleep};
#[allow(deprecated)] // doing the suggestion
use bindgen::CargoCallbacks;
use regex::Regex;


#[derive(Clone)]
struct Define {
    value: Option<String>,
    comment: String,
    default: bool,
    category: &'static str,
}

#[allow(dead_code)]
struct PlatformInfo {
    is_x64: bool,
    is_x86: bool,
    is_arm64: bool,
    is_windows: bool,
    is_macos: bool,
    is_clang: bool,
    is_gnu: bool,
    is_msvc: bool,
}

impl PlatformInfo {
    fn new(compiler: &cc::Tool) -> Self {
        Self {
            is_x64: env::var("CARGO_CFG_TARGET_ARCH") == Ok("x86_64".into()),
            is_x86: env::var("CARGO_CFG_TARGET_ARCH") == Ok("x86".into()),
            is_arm64: env::var("CARGO_CFG_TARGET_ARCH") == Ok("aarch64".into()),
            is_windows: env::var("CARGO_CFG_TARGET_OS") == Ok("windows".into()),
            is_macos: env::var("CARGO_CFG_TARGET_OS") == Ok("macos".into()),
            is_clang: compiler.is_like_clang(),
            is_gnu: compiler.is_like_gnu(),
            is_msvc: compiler.is_like_msvc(),
        }
    }
}

fn get_defines(info: &PlatformInfo) -> HashMap<&'static str, Define> {
    let mut defines = HashMap::new();

    // Threading Configuration
    // ----------------------
    // Z7_ST is controlled by the 'st' feature flag - multithreaded by default
    if env::var("CARGO_FEATURE_ST").is_ok() {
        defines.insert("Z7_ST", Define {
            value: None,
            comment: "Single-threaded mode".into(),
            default: false,
            category: "Threading",
        });
    }

    // Core/Required Defines (always enabled)
    // -------------------------------------
    defines.insert("_REENTRANT", Define {
        value: None,
        comment: "Thread-safe libc".into(),
        default: true,
        category: "Core",
    });
    defines.insert("_FILE_OFFSET_BITS", Define {
        value: Some("64".into()),
        comment: "Large file support".into(),
        default: true,
        category: "Core",
    });
    defines.insert("_LARGEFILE_SOURCE", Define {
        value: None,
        comment: "Large file support".into(),
        default: true,
        category: "Core",
    });
    if env::var("CARGO_FEATURE_EXTERNAL_CODECS").is_ok() {
        defines.insert("Z7_EXTERNAL_CODECS", Define {
            value: None,
            comment: "Support external codecs".into(),
            default: true,
            category: "Core",
        });
    }

    // Unicode Support (always enabled)
    // ------------------------------
    defines.insert("UNICODE", Define {
        value: None,
        comment: "Unicode support".into(),
        default: true,
        category: "Unicode",
    });
    defines.insert("_UNICODE", Define {
        value: None,
        comment: "Unicode support (Windows)".into(),
        default: true,
        category: "Unicode",
    });

    // Optional Features (controlled by Cargo features)
    // ---------------------------------------------
    if env::var("CARGO_FEATURE_LARGE_PAGES").is_ok() {
        defines.insert("Z7_LARGE_PAGES", Define {
            value: None,
            comment: "Large pages support".into(),
            default: false,
            category: "Performance",
        });
    }

    if env::var("CARGO_FEATURE_LONG_PATHS").is_ok() {
        defines.insert("Z7_LONG_PATH", Define {
            value: None,
            comment: "Long path support".into(),
            default: false,
            category: "FileSystem",
        });
    }

    // Use Hand Written Assembly Routines for Performance
    // --------------------------------------------------
    // This matches the settings in the makefiles:
    // var_clang_x64.mak: USE_ASM=1 USE_CLANG=1
    // var_clang_x86.mak: USE_ASM=1 USE_CLANG=1
    // var_clang_arm64.mak: USE_ASM=1 USE_CLANG=1
    // var_clang.mak (other platforms): USE_ASM= (undefined) USE_CLANG=1
    // etc.
    
    // For Rust, we're powered by LLVM, so clang.
    // Only exception is Apple macOS x64, that doesn't use USE_ASM.
    let is_x64 = info.is_x64;
    let is_x86 = info.is_x86;
    let is_arm64 = info.is_arm64;
    let is_macos = info.is_macos;
    
    if info.is_clang {
        defines.insert("USE_CLANG", Define {
            value: Some("1".to_owned()),
            comment: "Whether current compiler is Clang".into(),
            default: true,
            category: "Build",
        });
    }

    if is_x64 || is_x86 || is_arm64 {
        // All x86/x64/arm64 except Apple x64 
        if !(is_macos && is_x64) {
            defines.insert("USE_ASM", Define {
                value: Some("1".to_owned()),
                comment: "Enable assembly optimizations".into(),
                default: true,
                category: "Performance",
            });
        } 
    }

    if is_x64 {
        defines.insert("IS_X64", Define {
            value: Some("1".to_owned()),
            comment: "x64 platform".into(),
            default: true,
            category: "Architecture",
        });
    } else if is_x86 {
        defines.insert("IS_X86", Define {
            value: Some("1".to_owned()),
            comment: "x86 platform".into(),
            default: true,
            category: "Architecture",
        });
    } else if is_arm64 {
        defines.insert("IS_ARM64", Define {
            value: Some("1".to_owned()),
            comment: "ARM64 platform".into(),
            default: true,
            category: "Architecture",
        });
        defines.insert("ASM_FLAGS", Define {
            value: Some("-Wno-unused-macros".to_owned()),
            comment: "Flags related to Hand Written Assembly".into(),
            default: true,
            category: "Architecture",
        });
    }

    defines
}

fn get_source_files_from_includes(wrapper_path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(wrapper_path)?;
    let include_re = Regex::new(r#"#include\s+"7z/C/([^"]+)\.h""#)?;
    let mut sources = Vec::new();
    
    for cap in include_re.captures_iter(&content) {
        let header = cap.get(1).unwrap().as_str();
        let source = format!("7z/C/{}.c", header);
        if Path::new(&source).exists() {
            sources.push(source);
        }
    }
    
    Ok(sources)
}

fn prefer_clang(build: &mut cc::Build) {
    // We prefer clang, because that way it's all LLVM through and through,
    // which helps with performance.
    if Command::new("clang").arg("--version").output().is_ok() {
        build.compiler("clang");
    } else {
        println!("cargo:warning=Clang not found, falling back to gcc");
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Disable rust-analyzer before uncommenting.
    // Windows devs may need a different solution, but this works for Linux & macOS
    // Also uncomment [profile.dev.build-override] in Cargo.toml

    // let url = format!("vscode://vadimcn.vscode-lldb/launch/config?{{'request':'attach','pid':{}}}", std::process::id());
    // Command::new("code").arg("--open-url").arg(url).output().unwrap();
    // sleep(Duration::from_secs(1)); // Wait for debugger to attach

    let mut build = cc::Build::new();
    prefer_clang(&mut build);
    let source_files = get_source_files_from_includes("wrapper.h")?;
    let platform_info = PlatformInfo::new(&build.get_compiler());
    let defines = get_defines(&platform_info);

    // Base compilation flags 
    build
        .files(source_files)
        .include("7z/C");

    // Apply defines to cc::Build
    for (name, define) in &defines {
        build.define(name, define.value.as_deref());
    }

    // Compile the library
    build.compile("7zip");

    // Setup bindgen
    let mut bindgen = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg("-I7z/C")
        .allowlist_recursively(true)
        .derive_debug(true)
        .derive_default(true)
        .derive_eq(true)
        .derive_hash(true)
        .derive_ord(true)
        .impl_debug(true)
        .impl_partialeq(true)
        .size_t_is_usize(true)
        .layout_tests(true)
        .generate_comments(true)
        .explicit_padding(true)
        .wrap_unsafe_ops(true)
        .wrap_static_fns(true)
        .parse_callbacks(Box::new(CargoCallbacks::new()))
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .bitfield_enum(".*_FLAGS")
        .rustified_enum(".*");

    // Apply defines to bindgen
    for (name, define) in &defines {
        let arg = if let Some(value) = &define.value {
            format!("-D{}={}", name, value)
        } else {
            format!("-D{}", name)
        };
        bindgen = bindgen.clang_arg(&arg);
    }

    // Generate bindings
    let bindings = bindgen.generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR")?);
    bindings.write_to_file(out_path.join("bindings.rs"))?;

    // Print build configuration
    #[cfg(feature = "debug-build")]
    {
        println!("cargo:warning=7-Zip Build Configuration:");
        println!("cargo:warning=========================");
        
        // Get all unique categories
        let mut categories: Vec<_> = defines.values()
            .map(|d| d.category)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        categories.sort();
    
        // Print defines by category
        for category in categories {
            println!("cargo:warning=");
            println!("cargo:warning={} Defines:", category);
            println!("cargo:warning={}", "-".repeat(category.len() + 8));
            
            let category_defines: Vec<_> = defines.iter()
                .filter(|(_, d)| d.category == category)
                .collect();
                
            for (name, define) in category_defines {
                let status = if define.default { "default" } else { "optional" };
                let value_str = define.value.as_ref()
                    .map(|v| format!("={}", v))
                    .unwrap_or_default();
                println!(
                    "cargo:warning={}{} [{}] - {} ({})",
                    name,
                    value_str,
                    status,
                    define.comment,
                    if defines.contains_key(name) { "enabled" } else { "disabled" }
                );
            }
        }
    
        // Print platform info
        println!("cargo:warning=");
        println!("cargo:warning=Platform Configuration:");
        println!("cargo:warning======================");
        println!("cargo:warning=Target Architecture: {}", 
            if platform_info.is_x64 { "x86_64" }
            else if platform_info.is_x86 { "x86" }
            else if platform_info.is_arm64 { "arm64" }
            else { "unknown" }
        );
        println!("cargo:warning=Target OS: {}", 
            if platform_info.is_windows { "Windows" }
            else if platform_info.is_macos { "macOS" }
            else { "Unix/Linux" }
        );
    }

    Ok(())
}