# rsmt2d-rs

Rust implementation of two dimensional Reed-Solomon merkle tree data availability scheme.

This is a Rust port of the original Go implementation: https://github.com/celestiaorg/rsmt2d

## Features

- **Reed-Solomon Erasure Coding**: Efficient encoding and decoding using the Leopard codec
- **2D Data Squares**: Management of data in both row-major and column-major layouts
- **Merkle Tree Verification**: Compute and verify Merkle roots for data integrity
- **Data Recovery**: Repair incomplete data squares using crossword solving
- **Extensible Design**: Pluggable codec and tree implementations

## Example

```rust
use rsmt2d::{LeoRSCodec, compute_extended_data_square, new_default_tree};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // shareSize is the size of each share (in bytes).
    let share_size = 512;
    
    // Init new codec
    let codec = Arc::new(LeoRSCodec::new());

    let ones = vec![1u8; share_size];
    let twos = vec![2u8; share_size];
    let threes = vec![3u8; share_size];
    let fours = vec![4u8; share_size];

    // Compute parity shares
    let mut eds = compute_extended_data_square(
        vec![ones, twos, threes, fours],
        codec.clone(),
        new_default_tree,
    )?;

    let row_roots = eds.row_roots()?;
    let col_roots = eds.col_roots()?;

    let flattened = eds.flattened();

    // Delete some shares, just enough so that repairing is possible.
    let mut damaged_data = flattened;
    damaged_data[0] = None;
    damaged_data[2] = None;
    damaged_data[3] = None;
    damaged_data[4] = None;
    damaged_data[5] = None;
    damaged_data[6] = None;
    damaged_data[7] = None;

    // Re-import the data square.
    let mut incomplete_eds = rsmt2d::import_extended_data_square(
        damaged_data, 
        codec, 
        new_default_tree
    )?;

    // Repair square.
    incomplete_eds.repair(&row_roots, &col_roots)?;

    println!("Data square repaired successfully!");
    Ok(())
}
```

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rsmt2d = "0.1.0"
```

## Examples

Run the examples to see the library in action:

```bash
# Basic usage example
cargo run --example basic_usage

# Repair functionality demo  
cargo run --example repair_demo
```

## Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture
```

## Performance

The library uses the Leopard Reed-Solomon codec which is optimized for performance. Share sizes must be multiples of 64 bytes for optimal performance.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details. 
