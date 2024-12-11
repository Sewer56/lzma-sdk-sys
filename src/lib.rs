#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use core::ffi::c_void;

extern crate alloc;
extern crate core;

#[cfg(not(feature = "generate-bindings"))]
include!("bindings.rs");

#[cfg(feature = "generate-bindings")]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// Provide a default allocator implementation for lzma encoder.
pub struct Allocator {
    alloc: ISzAlloc,
}

impl Default for Allocator {
    fn default() -> Self {
        Self {
            alloc: ISzAlloc {
                Alloc: Some(sz_alloc),
                Free: Some(sz_free),
            },
        }
    }
}

impl Allocator {
    pub fn as_ref(&self) -> &ISzAlloc {
        &self.alloc
    }
}

unsafe extern "C" fn sz_alloc(_p: ISzAllocPtr, size: usize) -> *mut c_void {
    libc::malloc(size)
}

unsafe extern "C" fn sz_free(_p: ISzAllocPtr, address: *mut c_void) {
    libc::free(address)
}

#[cfg(feature = "test-build-size")]
#[no_mangle]
pub unsafe extern "C" fn LzmaDecode_Out(
    dest: *mut Byte,
    destLen: *mut SizeT,
    src: *const Byte,
    srcLen: *mut SizeT,
    propData: *const Byte,
    propSize: ::core::ffi::c_uint,
    finishMode: ELzmaFinishMode,
    status: *mut ELzmaStatus,
    alloc: ISzAllocPtr,
) -> SRes {
    LzmaDecode(dest, destLen, src, srcLen, propData, propSize, finishMode, status, alloc)
}

#[cfg(feature = "test-build-size")]
#[no_mangle]
pub unsafe extern "C" fn LzmaEncode_Out(
    dest: *mut Byte,
    destLen: *mut SizeT,
    src: *const Byte,
    srcLen: SizeT,
    props: *const CLzmaEncProps,
    propsEncoded: *mut Byte,
    propsSize: *mut SizeT,
    writeEndMark: ::core::ffi::c_int,
    progress: ICompressProgressPtr,
    alloc: ISzAllocPtr,
    allocBig: ISzAllocPtr,
) -> SRes
{
    LzmaEncode(dest, destLen, src, srcLen, props, propsEncoded, propsSize, writeEndMark, progress, alloc, allocBig)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn test_lzma_round_trip() {
        let input = b"Hello LZMA compression!";
        let mut props = [0u8; LZMA_PROPS_SIZE as usize];
        let mut props_size = LZMA_PROPS_SIZE as SizeT;
        
        // Encode
        let mut compressed = vec![0u8; input.len() * 2];
        let mut compressed_size = compressed.len() as SizeT;
        
        let alloc = Allocator::default();
        
        unsafe {
            let enc = LzmaEnc_Create(alloc.as_ref() as *const _);
            assert!(!enc.is_null());
            
            let mut enc_props = CLzmaEncProps::default();
            LzmaEncProps_Init(&mut enc_props);
            
            let res = LzmaEnc_SetProps(enc, &enc_props);
            assert_eq!(res, SZ_OK as i32);

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
            assert_eq!(res, SZ_OK as i32);
            
            LzmaEnc_Destroy(enc, alloc.as_ref(), alloc.as_ref());

            // Trim compressed buffer to actual size
            compressed.truncate(compressed_size as usize);

            // Decode
            // The LZMA format requires the decompressed size for decoding
            let mut dest = vec![0u8; input.len()];
            let mut dest_size = dest.capacity() as SizeT;
            let mut source_len: SizeT = compressed.len();
            let mut status: ELzmaStatus = ELzmaStatus::LZMA_STATUS_NOT_SPECIFIED;

            let res = LzmaDecode(
                dest.as_mut_ptr() as *mut Byte,
                &mut dest_size,
                compressed.as_ptr() as *const Byte,
                &mut source_len,
                props.as_ptr() as *const Byte,
                props_size as u32,
                ELzmaFinishMode::LZMA_FINISH_END,
                &mut status,
                alloc.as_ref(),
            );
            assert_eq!(res, SZ_OK as i32);

            // Verify the round-trip
            assert_eq!(dest_size as usize, input.len());
            assert_eq!(&dest[..dest_size as usize], input);
        }
    }
}