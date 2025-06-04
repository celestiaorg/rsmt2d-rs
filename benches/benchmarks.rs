use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rsmt2d::{
    new_test_leo_rs_codec, new_small_leo_rs_codec, new_medium_leo_rs_codec,
    new_default_tree, ExtendedDataSquare, Codec, Tree,
};

fn bench_codec_encode_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("codec_operations");
    
    for size in [2, 4, 8].iter() {
        let codec = match *size {
            2 => new_test_leo_rs_codec(),
            4 => new_small_leo_rs_codec(),
            8 => new_medium_leo_rs_codec(),
            _ => new_test_leo_rs_codec(),
        };
        
        // Create test data
        let test_data: Vec<Vec<u8>> = (0..*size)
            .map(|i| vec![i as u8; 32])
            .collect();
        
        group.bench_with_input(
            BenchmarkId::new("encode", size),
            size,
            |b, _| {
                b.iter(|| {
                    let result = codec.encode(black_box(test_data.clone()));
                    black_box(result).unwrap()
                })
            },
        );
        
        // Benchmark decode
        let encoded = codec.encode(test_data.clone()).unwrap();
        let decode_data: Vec<Option<Vec<u8>>> = encoded.into_iter().map(Some).collect();
        
        group.bench_with_input(
            BenchmarkId::new("decode", size),
            size,
            |b, _| {
                b.iter(|| {
                    let result = codec.decode(black_box(decode_data.clone()));
                    black_box(result).unwrap()
                })
            },
        );
    }
    
    group.finish();
}

fn bench_merkle_tree(c: &mut Criterion) {
    let mut group = c.benchmark_group("merkle_tree");
    
    for chunk_count in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("tree_construction", chunk_count),
            chunk_count,
            |b, &count| {
                let chunks: Vec<Vec<u8>> = (0..count)
                    .map(|i| vec![i as u8; 32])
                    .collect();
                
                b.iter(|| {
                    let mut tree = new_default_tree();
                    for chunk in &chunks {
                        tree.push(black_box(chunk));
                    }
                    black_box(tree.root())
                })
            },
        );
    }
    
    group.finish();
}

fn bench_extended_data_square(c: &mut Criterion) {
    let mut group = c.benchmark_group("extended_data_square");
    
    // Test with 2x2 square (4 chunks)
    let codec = Box::new(new_test_leo_rs_codec());
    let tree_fn = || Box::new(new_default_tree()) as Box<dyn rsmt2d::Tree>;
    
    let data = vec![
        vec![1, 2, 3, 4],
        vec![5, 6, 7, 8],
        vec![9, 10, 11, 12],
        vec![13, 14, 15, 16],
    ];
    
    group.bench_function("compute_extended_data_square", |b| {
        b.iter(|| {
            let result = ExtendedDataSquare::compute_extended_data_square(
                black_box(data.clone()),
                Box::new(new_test_leo_rs_codec()),
                tree_fn,
            );
            black_box(result).unwrap()
        })
    });
    
    // Benchmark root computation
    let eds = ExtendedDataSquare::compute_extended_data_square(
        data.clone(),
        codec,
        tree_fn,
    ).unwrap();
    
    group.bench_function("row_roots", |b| {
        b.iter(|| {
            let result = eds.row_roots();
            black_box(result).unwrap()
        })
    });
    
    group.bench_function("col_roots", |b| {
        b.iter(|| {
            let result = eds.col_roots();
            black_box(result).unwrap()
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_codec_encode_decode,
    bench_merkle_tree,
    bench_extended_data_square
);
criterion_main!(benches);