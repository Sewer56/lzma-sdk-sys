# lzma-sdk-sys

Raw FFI bindings for the LZMA encoder/decoder components of LZMA-SDK (7-zip).

This crate provides low-level access to LZMA compression functionality, with support for 
the new optimized assembly routines on supported platforms.

## Motivation

The implementation of LZMA in LZMA-SDK (7z) generally tends to be slightly better than the one in [xz-utils],
at equivalent settings.

```ignore
// Meshes from Interesting NPCs SE - Loose-29194-4-3-2-157834454

Tool       Size            Ratio     Flags
----------------------------------------------------
lzma-sdk   42.24 MiB       27.71%    7z a -txz -mx=9
xz         42.73 MiB       28.03%    xz -k -e -z -9
```

In all, compression speed, size and decompression speed.

However, for library usage, historically [xz-utils] has been much more common; be it a more familiar API,
ease of integration, or any other possible reason.

[In recent years however][7z-changelog], Igor Pavlov started adding hand-crafted assembly routines
for LZMA (de)compression in 7-Zip. These, notably increase decompression speed, by as much as 30%,
on supported platforms.

However LZMA-SDK with these optimizations may be non-trivial to build.
This Rust crate provides bindings for the LZMA Encoding/Decoding routines of LZMA-SDK, with the possibility
of using the hand-optimized assembly routines.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
lzma-sdk-sys = "0.1.0"
```

Basic example of compression and decompression:

```rust
use lzma_sdk_sys::*;
use std::ptr;

fn main() -> Result<(), &'static str> {
    // Sample data to compress
    let input = b"Hello LZMA compression!";
    
    // Create an allocator for memory management
    let alloc = Allocator::default();
    
    // Compression
    let (compressed, props) = unsafe {
        // Initialize compression properties
        let mut props = [0u8; LZMA_PROPS_SIZE as usize];
        let mut props_size = LZMA_PROPS_SIZE as SizeT;
        
        // Prepare output buffer
        let mut compressed = vec![0u8; input.len() * 2];
        let mut compressed_size = compressed.len() as SizeT;
        
        // Create and configure encoder
        let enc = LzmaEnc_Create(alloc.as_ref() as *const _);
        if enc.is_null() {
            return Err("Failed to create encoder");
        }
        
        let mut enc_props = CLzmaEncProps::default();
        LzmaEncProps_Init(&mut enc_props);
        
        // You can customize compression settings here, for example:
        // enc_props.level = 9; // Maximum compression
        // enc_props.dictSize = 1 << 20; // 1MB dictionary
        
        if LzmaEnc_SetProps(enc, &enc_props) != SZ_OK as i32 {
            return Err("Failed to set encoder properties");
        }

        // Perform compression
        let res = LzmaEncode(
            compressed.as_mut_ptr() as *mut Byte,
            &mut compressed_size,
            input.as_ptr() as *const Byte,
            input.len() as SizeT,
            &enc_props,
            props.as_mut_ptr() as *mut Byte,
            &mut props_size,
            0,
            ptr::null_mut(),
            alloc.as_ref(),
            alloc.as_ref(),
        );
        
        LzmaEnc_Destroy(enc, alloc.as_ref(), alloc.as_ref());
        
        if res != SZ_OK as i32 {
            return Err("Compression failed");
        }
        
        // Trim compressed buffer to actual size
        compressed.truncate(compressed_size as usize);
        (compressed, props)
    };
    
    // Decompression
    let decompressed = unsafe {
        let mut decompressed = vec![0u8; input.len()];
        let mut decompressed_size = decompressed.len() as SizeT;
        let mut source_len = compressed.len() as SizeT;
        let mut status = ELzmaStatus::LZMA_STATUS_NOT_SPECIFIED;
        
        let res = LzmaDecode(
            decompressed.as_mut_ptr() as *mut Byte,
            &mut decompressed_size,
            compressed.as_ptr() as *const Byte,
            &mut source_len,
            props.as_ptr() as *const Byte,
            LZMA_PROPS_SIZE as u32,
            ELzmaFinishMode::LZMA_FINISH_END,
            &mut status,
            alloc.as_ref(),
        );
        
        if res != SZ_OK as i32 {
            return Err("Decompression failed");
        }
        
        decompressed.truncate(decompressed_size as usize);
        decompressed
    };
    
    // Verify the round-trip
    assert_eq!(decompressed, input);
    println!("Successfully compressed and decompressed: {:?}", 
             std::str::from_utf8(&decompressed).unwrap());
    
    Ok(())
}
```

## Features

The crate provides several configuration options through Cargo features:

### Core Features

- `enable-asm`: Use hand-optimized assembly routines for improved performance (enabled by default)

### Threading Options

- Default: Multi-threaded operation
- `st`: Single-threaded mode

### Additional Options

- `debug-build-logs`: Enable detailed build configuration logging
- `debug-build-script`: Enable debugging of the build script (via CodeLLDB on Linux/macOS)

## Future Features

These features of the crate are set up correctly for future LZMA-SDK bindings, but are not currently
used by any of the code we create bindings for.

### Core Features

- `external-codecs`: Support for external codecs in 7z archive format (enabled by default)

### Additional Options

- `large-pages`: Enable large pages support for potential performance improvements
- `long-paths`: Support for long file paths

## Performance

When using optimized assembly routines (`enable-asm` feature), significant
performance improvements are achieved.

Here are benchmark results using a Ryzen 5900X, compressing and
decompressing the `COPYING` file from the 7z distribution:

| Operation                                    | Size (bytes)    | Time      | Throughput   |
| -------------------------------------------- | --------------- | --------- | ------------ |
| lzma/decompress/26530                        | 26530 (0.03 MB) | 268.29 µs | 94.304 MiB/s |
| lzma/decompress/26530 (x64 ASM Optimization) | 26530 (0.03 MB) | 196.77 µs | 128.58 MiB/s |

Tested on a Ryzen 5900X

### Platform Support [WIP: Adding CI Tests Soon]

The optimized assembly routines have been tested on:

- i686 Linux
- x86_64 Linux

While other platforms may work (CI tests pass), they haven't been extensively verified on actual hardware.

## Building

To build this crate, you should [install clang] as per the `bindgen` documentation.

This crate uses clang for both building the 7z library code, and for creating 7z bindings.

This choice is made such that, hopefully in the future, cross-language LTO will be supported, with
both Rust and 7z code built via LLVM.

No additional dependencies are required. Normally 7z requires you to bring your own assembler, but
this crate provides precompiled assembly code, to make it usable just like any other Rust crate.

## Current Status

This crate currently provides bindings sufficient for LZMA Encoding/Decoding functionality.

While most of the groundwork has been laid for supporting remaining 7z features, a small amount of extra
work is needed in terms of writing sanity tests around functionality using optimized assembly routines.

So for now, this crate only outputs bindings for functionality I personally am using.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
If you're adding new APIs, please add some basic sanity tests first to verify they work.

If you're updating the version of `LZMA-SDK`, please also [update the precompiled assembly code][precomp-asm].

## Credits & License

This crate is maintained by Sewer56 for usage in [sewer56-archives-nx].
Licensed under the MIT License.

LZMA-SDK was written by Igor Pavlov. 
The LZMA encoder/decoder exposed via their bindings are *public domain* (see file headers).

For any other future code exposed, please see 7zip's own licensing terms. Chances are it's LGPL-V3.

[sewer56-archives-nx]: https://github.com/Sewer56/sewer56-archives-nx
[xz-utils]: https://github.com/tukaani-project/xz
[7z-changelog]: https://www.7-zip.org/history.txt
[install clang]: https://rust-lang.github.io/rust-bindgen/requirements.html
[precomp-asm]: ./precompiled-asm/readme.md