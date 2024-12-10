use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use lzma_sdk_sys::{Allocator, CLzmaEncProps, ELzmaFinishMode, ELzmaStatus, LZMA_PROPS_SIZE};
use lzma_sdk_sys::{LzmaEnc_Create, LzmaEnc_Destroy, LzmaEnc_SetProps, LzmaEncProps_Init};
use lzma_sdk_sys::{LzmaDecode, LzmaEncode, SZ_OK, SizeT, Byte};
use std::{fs, ptr};

fn compress_data(input: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let mut props = vec![0u8; LZMA_PROPS_SIZE as usize];
    let mut props_size = LZMA_PROPS_SIZE as SizeT;
    let mut compressed = vec![0u8; input.len() * 2];
    let mut compressed_size = compressed.len() as SizeT;
    let alloc = Allocator::default();

    unsafe {
        let enc = LzmaEnc_Create(alloc.as_ref() as *const _);
        assert!(!enc.is_null());

        let mut enc_props = CLzmaEncProps::default();
        LzmaEncProps_Init(&mut enc_props);
        
        // Set maximum compression level (9)
        enc_props.level = 9;
        // Use maximum dictionary size for best compression
        enc_props.dictSize = 1 << 24; // 16MB dictionary
        // Use more fast bytes for better compression
        enc_props.fb = 273;
        // Use maximum search depth
        enc_props.mc = 1 << 30;
        // Use single thread
        enc_props.numThreads = 1;

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
    }

    compressed.truncate(compressed_size as usize);
    (compressed, props)
}

fn decompress_data(compressed: &[u8], props: &[u8], original_size: usize) -> Vec<u8> {
    let mut dest = vec![0u8; original_size];
    let mut dest_size = dest.capacity() as SizeT;
    let mut source_len: SizeT = compressed.len() as SizeT;
    let mut status: ELzmaStatus = ELzmaStatus::LZMA_STATUS_NOT_SPECIFIED;
    let alloc = Allocator::default();

    unsafe {
        let res = LzmaDecode(
            dest.as_mut_ptr() as *mut Byte,
            &mut dest_size,
            compressed.as_ptr() as *const Byte,
            &mut source_len,
            props.as_ptr() as *const Byte,
            LZMA_PROPS_SIZE as u32,
            ELzmaFinishMode::LZMA_FINISH_END,
            &mut status,
            alloc.as_ref(),
        );
        assert_eq!(res, SZ_OK as i32);
    }

    dest.truncate(dest_size as usize);
    dest
}

fn bench_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("lzma");
    
    // Read the COPYING file
    let input = fs::read("7z/COPYING").expect("Failed to read COPYING file");
    let size = input.len();
    
    // Set throughput for MB/s calculation
    group.throughput(Throughput::Bytes(size as u64));
    
    println!("File size: {} bytes ({:.2} MB)", size, size as f64 / (1024.0 * 1024.0));
    
    group.bench_with_input(BenchmarkId::new("compress", size), &input, |b, input| {
        b.iter(|| {
            let (compressed, _) = compress_data(black_box(input));
            black_box(compressed)
        })
    });
    
    group.finish();
}

fn bench_decompression(c: &mut Criterion) {
    let mut group = c.benchmark_group("lzma");
    
    // Read and pre-compress the COPYING file
    let input = fs::read("7z/COPYING").expect("Failed to read COPYING file");
    let size = input.len();
    let (compressed, props) = compress_data(&input);
    
    // Set throughput for MB/s calculation
    group.throughput(Throughput::Bytes(size as u64));
    
    println!("Original size: {} bytes ({:.2} MB)", size, size as f64 / (1024.0 * 1024.0));
    println!("Compressed size: {} bytes ({:.2} MB)", compressed.len(), compressed.len() as f64 / (1024.0 * 1024.0));
    println!("Compression ratio: {:.2}%", (compressed.len() as f64 / size as f64) * 100.0);
    
    group.bench_with_input(BenchmarkId::new("decompress", size), &size, |b, &size| {
        b.iter(|| {
            let decompressed = decompress_data(
                black_box(&compressed),
                black_box(&props),
                black_box(size)
            );
            black_box(decompressed)
        })
    });
    
    group.finish();
}

criterion_group!(benches, bench_compression, bench_decompression);
criterion_main!(benches);