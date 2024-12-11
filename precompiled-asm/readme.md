# About this Folder

The files here correspond to those in the `7z/Asm` folder.
These were all precompiled for the given target platforms.

When the `enable-asm` Rust feature is enabled, we will link against these hand written
assembly routines, rather than the C versions.

## Last Update Info

Last update: 24.09
The commands below require [uasm] and have been derived from reading `7zip_gcc_c.mak`.

# Building 7-Zip Assembly Files with UASM

You can find the assembly files in `7z

## Linux x64 (64-bit ELF)

```bash
uasm -elf64 -DABI_LINUX 7zCrcOpt.asm
uasm -elf64 -DABI_LINUX XzCrc64Opt.asm
uasm -elf64 -DABI_LINUX AesOpt.asm
uasm -elf64 -DABI_LINUX Sha1Opt.asm
uasm -elf64 -DABI_LINUX Sha256Opt.asm
uasm -elf64 -DABI_LINUX LzmaDecOpt.asm
```

## Linux x86 (32-bit ELF)

```bash
uasm -elf -DABI_LINUX -DABI_CDECL 7zCrcOpt.asm
uasm -elf -DABI_LINUX -DABI_CDECL XzCrc64Opt.asm
uasm -elf -DABI_LINUX -DABI_CDECL AesOpt.asm
uasm -elf -DABI_LINUX -DABI_CDECL Sha1Opt.asm
uasm -elf -DABI_LINUX -DABI_CDECL Sha256Opt.asm
```

## Windows x64 (64-bit)

```bash
uasm -win64 -DABI_LINUX 7zCrcOpt.asm
uasm -win64 -DABI_LINUX XzCrc64Opt.asm
uasm -win64 -DABI_LINUX AesOpt.asm
uasm -win64 -DABI_LINUX Sha1Opt.asm
uasm -win64 -DABI_LINUX Sha256Opt.asm
uasm -win64 -DABI_LINUX LzmaDecOpt.asm
```

## Windows x86 (32-bit)

```bash
uasm -coff -DABI_WINDOWS -DABI_CDECL 7zCrcOpt.asm
uasm -coff -DABI_WINDOWS -DABI_CDECL XzCrc64Opt.asm
uasm -coff -DABI_WINDOWS -DABI_CDECL AesOpt.asm
uasm -coff -DABI_WINDOWS -DABI_CDECL Sha1Opt.asm
uasm -coff -DABI_WINDOWS -DABI_CDECL Sha256Opt.asm
```

## ARM64 (64-bit)

No action is needed here, the assembly written there is supported by gcc/clang, and will be
correctly built by the Rust build script.

## Apple x64 (64-bit Mach-O)

```bash
uasm -macho64 -DABI_LINUX 7zCrcOpt.asm
uasm -macho64 -DABI_LINUX XzCrc64Opt.asm
uasm -macho64 -DABI_LINUX AesOpt.asm
uasm -macho64 -DABI_LINUX Sha1Opt.asm
uasm -macho64 -DABI_LINUX Sha256Opt.asm
uasm -macho64 -DABI_LINUX LzmaDecOpt.asm
```

`DABI_LINUX` is not a typo here.

This one was complex, compiler does not like UASM output:

```
cargo:warning=/Applications/Xcode_15.2.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/bin/ranlib: file: /Users/runner/work/lzma-sdk-sys/lzma-sdk-sys/target/x86_64-apple-darwin/debug/build/lzma-sdk-sys-6e4ba5a1833852f6/out/lib7zip.a(LzmaDecOpt.o) malformed object (symbol 0 must not have NO_SECT for its n_sect field given its type (N_SECT))
cargo:warning=/Applications/Xcode_15.2.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/bin/ranlib: file: /Users/runner/work/lzma-sdk-sys/lzma-sdk-sys/target/x86_64-apple-darwin/debug/build/lzma-sdk-sys-6e4ba5a1833852f6/out/lib7zip.a(LzmaDecOpt.o) malformed object (symbol 1 must not have NO_SECT for its n_sect field given its type (N_SECT))
cargo:warning=/Applications/Xcode_15.2.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/bin/ranlib: file: /Users/runner/work/lzma-sdk-sys/lzma-sdk-sys/target/x86_64-apple-darwin/debug/build/lzma-sdk-sys-6e4ba5a1833852f6/out/lib7zip.a(LzmaDecOpt.o) malformed object (symbol 2 must not have NO_SECT for its n_sect field given its type (N_SECT))
cargo:warning=/Applications/Xcode_15.2.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/bin/ar: internal ranlib command failed
```

I've tried some more things, but was unable to get it working.

### Notes

- These commands assume all .asm files are in the current directory
- `LzmaDecOpt.asm` is only compiled for 64-bit `x64` and `ARM64` configurations 
- Linux builds will produce .o (ELF) files
- Windows builds will produce .obj (COFF) files

[uasm]: https://www.terraspace.co.uk/uasm.html