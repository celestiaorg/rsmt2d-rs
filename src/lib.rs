//! # rsmt2d
//!
//! Rust implementation of two dimensional Reed-Solomon merkle tree data availability scheme.
//!
//! This library provides a data availability scheme using Reed-Solomon erasure coding
//! combined with merkle trees to enable efficient fraud proofs and data recovery.
//!
//! ## Features
//!
//! - Two-dimensional Reed-Solomon encoding for data availability
//! - Merkle tree construction for compact proofs
//! - Data repair capabilities with configurable redundancy
//! - High-performance implementation with optional parallelization
//!
//! ## Example
//!
//! ```rust
//! use rsmt2d::{new_test_leo_rs_codec, new_default_tree, ExtendedDataSquare};
//!
//! // Create codec and tree
//! let codec = new_test_leo_rs_codec();
//! let tree_fn = || Box::new(new_default_tree()) as Box<dyn rsmt2d::Tree>;
//!
//! // Create some test data (4 chunks for a 2x2 square)
//! let data = vec![
//!     vec![1, 2, 3, 4],  // chunk 0
//!     vec![5, 6, 7, 8],  // chunk 1
//!     vec![9, 10, 11, 12], // chunk 2
//!     vec![13, 14, 15, 16], // chunk 3
//! ];
//!
//! // Compute extended data square
//! let eds = ExtendedDataSquare::compute_extended_data_square(
//!     data,
//!     Box::new(codec),
//!     tree_fn,
//! ).expect("Failed to compute EDS");
//!
//! println!("Extended data square width: {}", eds.width());
//! ```

pub mod error;
pub mod traits;
pub mod codec;
pub mod tree;
pub mod extended_data_square;

// Re-export main types and functions for convenient access
pub use error::{EdsError, CodecError, RepairError, EdsResult, CodecResult, RepairResult};
pub use traits::{Codec, Tree};
pub use codec::{LeoRSCodec, new_leo_rs_codec, new_test_leo_rs_codec};
pub use tree::{DefaultTree, new_default_tree};
pub use extended_data_square::ExtendedDataSquare;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_example_usage() {
        // Test the example from the documentation
        let codec = new_test_leo_rs_codec();
        let tree_fn = || Box::new(new_default_tree()) as Box<dyn Tree>;

        // Create some test data (must be square and power of 2)
        // Create 4 chunks for a 2x2 square
        let data = vec![
            vec![1, 2, 3, 4],  // chunk 0
            vec![5, 6, 7, 8],  // chunk 1
            vec![9, 10, 11, 12], // chunk 2
            vec![13, 14, 15, 16], // chunk 3
        ];

        let eds = ExtendedDataSquare::compute_extended_data_square(
            data,
            Box::new(codec),
            tree_fn,
        ).expect("Failed to compute EDS");

        assert_eq!(eds.width(), 4); // 2x2 -> 4x4 after Reed-Solomon
    }

    #[test]
    fn test_api_integration() {
        // Test that all main components work together
        let codec = new_test_leo_rs_codec();
        let mut tree = new_default_tree();

        // Test codec with smaller data
        let test_data = vec![vec![1, 2, 3]; 2]; // 2 chunks for 2+2 codec
        let encoded = codec.encode(test_data.clone()).unwrap();
        assert_eq!(encoded.len(), 4); // data + parity

        // Test tree
        tree.push(b"test data");
        let root = tree.root();
        assert_eq!(root.len(), 32); // SHA256 hash

        // Test error types
        let _: EdsResult<()> = Err(EdsError::InvalidData("test".to_string()));
        let _: CodecResult<()> = Err(CodecError::InsufficientData);
        let _: RepairResult<()> = Err(RepairError::MissingRequiredData);
    }
}