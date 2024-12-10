#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

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

unsafe extern "C" fn sz_alloc(_p: ISzAllocPtr, size: usize) -> *mut ::std::os::raw::c_void {
    libc::malloc(size)
}

unsafe extern "C" fn sz_free(_p: ISzAllocPtr, address: *mut ::std::os::raw::c_void) {
    libc::free(address)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn test_lzma_encode() {
        let input = b"Hello LZMA compression!";
        let mut props = [0u8; LZMA_PROPS_SIZE as usize];
        let mut props_size = LZMA_PROPS_SIZE as SizeT;
        
        let mut output = vec![0u8; input.len() * 2];
        let mut output_size = output.len() as SizeT;
        
        let alloc = Allocator::default();
        
        unsafe {
            let enc = LzmaEnc_Create(alloc.as_ref() as *const _);
            assert!(!enc.is_null());
            
            let mut enc_props = CLzmaEncProps::default();
            LzmaEncProps_Init(&mut enc_props);
            
            let res = LzmaEnc_SetProps(enc, &enc_props);
            assert_eq!(res, SZ_OK as i32);

            let res = LzmaEncode(
                output.as_mut_ptr() as *mut Byte,
                &mut output_size,
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
        }
    }
}