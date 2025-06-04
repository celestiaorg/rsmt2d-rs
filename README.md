# rsmt2d-rs

Rust implementation of two dimensional Reed-Solomon merkle tree data availability scheme.

[![Build Status](https://github.com/celestiaorg/rsmt2d-rs/workflows/CI/badge.svg)](https://github.com/celestiaorg/rsmt2d-rs/actions)
[![Documentation](https://docs.rs/rsmt2d/badge.svg)](https://docs.rs/rsmt2d)
[![Crates.io](https://img.shields.io/crates/v/rsmt2d)](https://crates.io/crates/rsmt2d)

## Overview

This library provides a high-performance Rust implementation of the two-dimensional Reed-Solomon merkle tree data availability scheme, originally designed for blockchain data availability layers. It offers:

- **Reed-Solomon Erasure Coding**: Efficient error correction with configurable redundancy levels
- **Merkle Tree Construction**: Fast cryptographic commitment to data integrity  
- **2D Data Layout**: Optimal organization for fraud proof generation and verification
- **Repair Functionality**: Automatic reconstruction of missing or corrupted data
- **High Performance**: Optimized implementation with benchmarking support

## Features

- 🚀 **High Performance**: Optimized Reed-Solomon implementation with SIMD support
- 🔒 **Memory Safe**: Zero unsafe code blocks, leveraging Rust's safety guarantees  
- 🧮 **Flexible Configuration**: Multiple codec sizes (2+2, 4+4, 8+8, 16+16)
- 🔧 **Comprehensive API**: Full-featured API with repair, import/export, statistics
- ✅ **Well Tested**: 36 unit tests + property-based tests + benchmarks
- 📚 **Documented**: Complete API documentation with examples

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
rsmt2d = "0.1.0"
```

### Basic Usage

```rust
use rsmt2d::{new_test_leo_rs_codec, new_default_tree, ExtendedDataSquare};

// Create codec and tree factory
let codec = new_test_leo_rs_codec();
let tree_fn = || Box::new(new_default_tree()) as Box<dyn rsmt2d::Tree>;

// Create some test data (4 chunks for a 2x2 square)
let data = vec![
    vec![1, 2, 3, 4],   // chunk 0
    vec![5, 6, 7, 8],   // chunk 1  
    vec![9, 10, 11, 12], // chunk 2
    vec![13, 14, 15, 16], // chunk 3
];

// Compute extended data square with Reed-Solomon encoding
let eds = ExtendedDataSquare::compute_extended_data_square(
    data,
    Box::new(codec),
    tree_fn,
)?;

println!("Extended data square width: {}", eds.width()); // 4
println!("Original width: {}", eds.original_width());   // 2

// Compute merkle roots for fraud proofs
let row_roots = eds.row_roots()?;
let col_roots = eds.col_roots()?;
```

### Advanced Features

```rust
use rsmt2d::{new_small_leo_rs_codec, ExtendedDataSquare, SquareIndex};

// Create a larger codec for bigger data sets
let codec = Box::new(new_small_leo_rs_codec()); // 4+4 configuration

// Import from flattened data (e.g., from network)
let flattened_data: Vec<Option<Vec<u8>>> = /* ... */;
let eds = ExtendedDataSquare::import_extended_data_square(
    flattened_data,
    codec,
    tree_fn,
)?;

// Simulate missing data and check repair capabilities
let mut eds_copy = eds.clone();
eds_copy.simulate_missing_data(vec![
    SquareIndex::new(0, 1),
    SquareIndex::new(1, 0),
])?;

// Get statistics about the data square
let stats = eds_copy.stats()?;
println!("Missing chunks: {}", stats.missing_chunks);
println!("Completion ratio: {:.2}%", stats.completion_ratio * 100.0);

// Attempt repair using merkle roots
let row_roots = eds.row_roots()?;
let col_roots = eds.col_roots()?;
match eds_copy.repair(row_roots, col_roots) {
    Ok(()) => println!("Repair successful"),
    Err(e) => println!("Repair failed: {}", e),
}
```

## Architecture

The library is organized into several key modules:

- **`codec`**: Reed-Solomon erasure coding implementation
- **`tree`**: Merkle tree construction and root computation  
- **`extended_data_square`**: Main 2D data structure and operations
- **`error`**: Comprehensive error handling
- **`traits`**: Abstract interfaces for extensibility

## Performance

The library includes comprehensive benchmarks:

```bash
cargo bench
```

Key performance characteristics:
- **Codec Operations**: Sub-microsecond encoding/decoding for small data
- **Merkle Trees**: Efficient batch hashing with SHA256
- **Data Squares**: Optimized 2D layout with minimal memory allocations

## Testing

Run the test suite:

```bash
cargo test
```

The library includes:
- 36 comprehensive unit tests
- Property-based tests with `proptest`
- Integration tests
- Documentation tests

## Configuration Options

### Codec Configurations

| Function | Data Shards | Parity Shards | Use Case |
|----------|-------------|---------------|----------|
| `new_test_leo_rs_codec()` | 2 | 2 | Testing, small data |
| `new_small_leo_rs_codec()` | 4 | 4 | Small applications |
| `new_medium_leo_rs_codec()` | 8 | 8 | Medium workloads |
| `new_leo_rs_codec()` | 16 | 16 | Production (default) |

### Error Tolerance

Each configuration can tolerate corruption in up to 50% of the total chunks while maintaining recoverability.

## Contributing

Contributions are welcome! Please read our contributing guidelines and submit pull requests to the main branch.

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

## References

- [Original Go Implementation](https://github.com/celestiaorg/rsmt2d)
- [Reed-Solomon Erasure Codes](https://en.wikipedia.org/wiki/Reed%E2%80%93Solomon_error_correction)
- [Merkle Trees](https://en.wikipedia.org/wiki/Merkle_tree)
