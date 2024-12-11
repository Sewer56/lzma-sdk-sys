use std::{collections::HashMap, env, fs, path::{Path, PathBuf}, process::Command};
#[allow(deprecated)] // doing the suggestion
use bindgen::CargoCallbacks;
use regex::Regex;

#[cfg(feature = "debug-build-script")]
use core::time::Duration;
#[cfg(feature = "debug-build-script")]
use std::thread::sleep;

#[derive(Clone)]
#[allow(dead_code)]
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

    fn supports_decompression_acceleration(&self) -> bool {
        let not_apple_x64 = !(self.is_macos && self.is_x64);
        (self.is_x64 || self.is_arm64) && not_apple_x64
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

    // Those prefixed with MAKEFILE are the makefile variables.
    // Not used in compilation, but used to keep accuracy with upstream when verifying.
    if info.is_clang {
        defines.insert("MAKEFILE_USE_CLANG", Define {
            value: Some("1".to_owned()),
            comment: "Whether current compiler is Clang".into(),
            default: true,
            category: "Build",
        });
    }

    if (is_x64 || is_x86 || is_arm64) && env::var("CARGO_FEATURE_ENABLE_ASM").is_ok() {
        // All x86/x64/arm64 except Apple x64 
        if !(is_macos && is_x64) {
            defines.insert("MAKEFILE_USE_ASM", Define {
                value: Some("1".to_owned()),
                comment: "Enable assembly optimizations".into(),
                default: true,
                category: "Performance",
            });

            /*
                // Original Makefile.

                ifdef USE_LZMA_DEC_ASM
                    ifdef IS_X64
                        $O/LzmaDecOpt.o: ../../../Asm/x86/LzmaDecOpt.asm
                            $(MY_ASM) $(AFLAGS) $<
                    endif

                    ifdef IS_ARM64
                        $O/LzmaDecOpt.o: ../../../Asm/arm64/LzmaDecOpt.S ../../../Asm/arm64/7zAsm.S
                            $(CC) $(CFLAGS) $(ASM_FLAGS) $<
                    endif

                    $O/LzmaDec.o: ../../LzmaDec.c
                        $(CC) $(CFLAGS) -DZ7_LZMA_DEC_OPT $<
                else

                $O/LzmaDec.o: ../../LzmaDec.c
                    $(CC) $(CFLAGS) $<

                endif
            */
            if info.supports_decompression_acceleration() {
                defines.insert("Z7_LZMA_DEC_OPT", Define {
                    value: Some("1".to_owned()),
                    comment: "Enable assembly optimizations".into(),
                    default: true,
                    category: "Performance",
                });
                // Rust Note: We link `LzmaDec.c` via the header `LzmaDec.h` in `wrapper.h`
                // So we need to set this define if enabling the feature.
            }
        } 
    }

    if is_x64 {
        defines.insert("MAKEFILE_IS_X64", Define {
            value: Some("1".to_owned()),
            comment: "x64 platform".into(),
            default: true,
            category: "Architecture",
        });
    } else if is_x86 {
        defines.insert("MAKEFILE_IS_X86", Define {
            value: Some("1".to_owned()),
            comment: "x86 platform".into(),
            default: true,
            category: "Architecture",
        });
    } else if is_arm64 {
        defines.insert("MAKEFILE_IS_ARM64", Define {
            value: Some("1".to_owned()),
            comment: "ARM64 platform".into(),
            default: true,
            category: "Architecture",
        });
        defines.insert("MAKEFILE_ASM_FLAGS", Define {
            value: Some("-Wno-unused-macros".to_owned()),
            comment: "Flags related to Hand Written Assembly".into(),
            default: true,
            category: "Architecture",
        });
    }

    defines
}

/// Extracts source file paths from C/C++ include directives in a wrapper file.
///
/// This function scans a given wrapper file for `#include` directives that reference
/// files in the "7z/C/" directory and builds a list of corresponding source files:
/// 
/// - For `.h` includes: looks for matching `.c` implementation files
/// - For `.c` includes: adds them directly to the source list
/// 
/// All paths are verified to exist before being included in the result.
///
/// # Arguments
/// * `wrapper_path` - Path to the wrapper file to analyze
///
/// # Returns
/// * `Result<Vec<String>>` - A vector of existing source file paths on success
///                          or an error if file reading or regex compilation fails
fn get_source_files_from_includes(wrapper_path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(wrapper_path)?;
    let include_re = Regex::new(r#"#include\s+"7z/C/([^"]+)\.(h|c)""#)?;
    let mut sources = Vec::new();
    
    for cap in include_re.captures_iter(&content) {
        let file_name = cap.get(1).unwrap().as_str();
        let extension = cap.get(2).unwrap().as_str();
        
        if extension == "c" {
            let source = format!("7z/C/{}.c", file_name);
            if Path::new(&source).exists() {
                sources.push(source);
            }
        } else {
            let source = format!("7z/C/{}.c", file_name);
            if Path::new(&source).exists() {
                sources.push(source);
            }
        }
    }

    /*
        TODO: Replace compilation units with assembly files
        when enable-asm feature is used.

        ifdef USE_X86_ASM
        $O/7zCrcOpt.o: ../../../Asm/x86/7zCrcOpt.asm
            $(MY_ASM) $(AFLAGS) $<
        $O/XzCrc64Opt.o: ../../../Asm/x86/XzCrc64Opt.asm
            $(MY_ASM) $(AFLAGS) $<
        $O/AesOpt.o: ../../../Asm/x86/AesOpt.asm
            $(MY_ASM) $(AFLAGS) $<
        $O/Sha1Opt.o: ../../../Asm/x86/Sha1Opt.asm
            $(MY_ASM) $(AFLAGS) $<
        $O/Sha256Opt.o: ../../../Asm/x86/Sha256Opt.asm
            $(MY_ASM) $(AFLAGS) $<
        else
        $O/7zCrcOpt.o: ../../7zCrcOpt.c
            $(CC) $(CFLAGS) $<
        $O/XzCrc64Opt.o: ../../XzCrc64Opt.c
            $(CC) $(CFLAGS) $<
        $O/Sha1Opt.o: ../../Sha1Opt.c
            $(CC) $(CFLAGS) $<
        $O/Sha256Opt.o: ../../Sha256Opt.c
            $(CC) $(CFLAGS) $<
        $O/AesOpt.o: ../../AesOpt.c
            $(CC) $(CFLAGS) $<
        endif
    */
    
    Ok(sources)
}

/// This function would find the first flag in `flags` that is supported
/// and add that to `build`.
#[allow(dead_code)]
fn flag_if_supported_with_fallbacks(build: &mut cc::Build, flags: &[&str]) {
    let option = flags
        .iter()
        .find(|flag| build.is_flag_supported(flag).unwrap_or_default());

    if let Some(flag) = option {
        build.flag(flag);
    }
}

fn prefer_clang(build: &mut cc::Build) {
    // We prefer clang, because that way it's all LLVM through and through,
    // which helps with performance.
    if !env::var("CARGO_FEATURE_PREFER_CLANG").is_ok() {
        return;
    }

    if Command::new("clang").arg("--version").output().is_ok() {
        build.compiler("clang");
    } else {
        println!("cargo:warning=Clang not found, falling back to gcc");
    }

    if env::var("CARGO_FEATURE_FAT_LTO").is_ok() {
        build.flag_if_supported("-flto");
    } else if env::var("CARGO_FEATURE_THIN_LTO").is_ok() {
        flag_if_supported_with_fallbacks(
            build,
            &["-flto=thin", "-flto"],
        );
    }
}

fn add_asm_files(build: &mut cc::Build, build_info: &PlatformInfo) -> Result<(), Box<dyn std::error::Error>> {
    // Only add ASM files if enabled
    if !env::var("CARGO_FEATURE_ENABLE_ASM").is_ok() || !build_info.supports_decompression_acceleration() {
        return Ok(());
    }

    if build_info.is_arm64 {
        // ARM64: Add .S files directly to the build
        build
            .file("7z/Asm/arm64/LzmaDecOpt.S")
            .file("7z/Asm/arm64/7zAsm.S");
    } else if build_info.is_x64 || build_info.is_x86 {
        // Get the right directory for precompiled objects
        let obj_dir = if build_info.is_windows {
            if build_info.is_x64 { "precompiled-asm/x86/win-x64" }
            else { "precompiled-asm/x86/win-x86" }
        } else if build_info.is_macos {
            if build_info.is_x64 { "precompiled-asm/x86/apple-x64" }
            else { 
                println!("cargo:warning='enable-asm' feature is not supported for this macOS architecture");
                panic!("'enable-asm' feature is not supported for this macOS architecture")
            }
        }
        else { // ELF. Unixes. Including Apple, Android, etc.
            if build_info.is_x64 { "precompiled-asm/x86/linux-x64" }
            else { "precompiled-asm/x86/linux-x86" }
        };

        // List all object files
        let mut objects = vec![
            "7zCrcOpt",
            "XzCrc64Opt", 
            "AesOpt",
            "Sha1Opt",
            "Sha256Opt",
        ];

        // LzmaDecOpt is 64-bit only
        if build_info.is_x64 {
            objects.push("LzmaDecOpt");
        }

        // Add each object file to the build
        for obj in objects {
            build.object(format!("{}/{}.o", obj_dir, obj));
        }
    }

    Ok(())
}

fn generate_bindings(defines: &HashMap<&'static str, Define>) -> Result<String, Box<dyn std::error::Error>> {
    // Configure and generate bindings
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
        .layout_tests(false) // Issues on Windows with multithreaded builds
        .generate_comments(true)
        .explicit_padding(true)
        .wrap_unsafe_ops(true)
        .wrap_static_fns(true)
        .use_core()
        .parse_callbacks(Box::new(CargoCallbacks::new()))
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .bitfield_enum(".*_FLAGS")
        .rustified_enum(".*");

    // Apply defines to bindgen
    for (name, define) in defines {
        let arg = if let Some(value) = &define.value {
            format!("-D{}={}", name, value)
        } else {
            format!("-D{}", name)
        };
        bindgen = bindgen.clang_arg(&arg);
    }

    Ok(bindgen.generate()?.to_string())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Disable rust-analyzer before uncommenting.
    // Windows devs may need a different solution, but this works for Linux & macOS
    // Also uncomment [profile.dev.build-override] in Cargo.toml

    #[cfg(feature = "debug-build-script")] {
      let url = format!("vscode://vadimcn.vscode-lldb/launch/config?{{'request':'attach','pid':{}}}", std::process::id());
      Command::new("code").arg("--open-url").arg(url).output().unwrap();
      sleep(Duration::from_secs(1)); // Wait for debugger to attach
    }

    let mut build = cc::Build::new();
    prefer_clang(&mut build);
    let source_files = get_source_files_from_includes("wrapper.h")?;
    let platform_info = PlatformInfo::new(&build.get_compiler());
    let defines = get_defines(&platform_info);

    // Apply defines to cc::Build
    for (name, define) in &defines {
        build.define(name, define.value.as_deref());
    }

    // Base compilation flags 
    build
        .files(source_files)
        .include("7z/C");

    // Link assembly files if enabled
    add_asm_files(&mut build, &platform_info)?;

    // Compile the library
    build.compile("7zip");

    // Generate bindings if the feature is enabled
    if env::var("CARGO_FEATURE_GENERATE_BINDINGS").is_ok() {
        let bindings = generate_bindings(&defines)?;
        let out_dir = PathBuf::from(env::var("OUT_DIR")?);
        
        // Write to OUT_DIR for compilation
        fs::write(out_dir.join("bindings.rs"), &bindings)?;
        
        // Also save to src for version control
        fs::write("src/bindings.rs", &bindings)?;
    }

    // Print build configuration
    #[cfg(feature = "debug-build-logs")]
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