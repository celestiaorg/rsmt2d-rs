//! # rsmt2d
//!
//! Rust implementation of two dimensional Reed-Solomon Merkle tree data availability scheme.
//!
//! This library provides functionality for:
//! - Computing extended data squares from original data using Reed-Solomon erasure coding
//! - Importing and exporting extended data squares
//! - Repairing incomplete data squares using the crossword solving algorithm
//! - Computing Merkle roots for data integrity verification
//!
//! ## Quick Start
//!
//! ```rust
//! use rsmt2d::{LeoRSCodec, compute_extended_data_square, new_default_tree_constructor};
//! use std::sync::Arc;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let share_size = 64; // Must be multiple of 64 for Leopard codec
//! let codec = Arc::new(LeoRSCodec::new());
//!
//! // Create some test data
//! let data = vec![
//!     vec![1u8; share_size],
//!     vec![2u8; share_size],
//!     vec![3u8; share_size],
//!     vec![4u8; share_size],
//! ];
//!
//! // Compute extended data square with parity
//! let mut eds = compute_extended_data_square(data, codec, new_default_tree_constructor())?;
//!
//! // Get row and column roots for verification
//! let row_roots = eds.row_roots()?;
//! let col_roots = eds.col_roots()?;
//!
//! println!("Created {}x{} extended data square", eds.width(), eds.width());
//! # Ok(())
//! # }
//! ```

pub mod codec;
pub mod datasquare;
pub mod error;
pub mod extendeddatasquare;
pub mod tree;

use std::sync::Arc;

// Re-export commonly used types
pub use codec::{Codec, LeoRSCodec};
pub use datasquare::DataSquare;
pub use error::{ByzantineDataError, Error, Result};
pub use extendeddatasquare::{
    compute_extended_data_square, import_extended_data_square, ExtendedDataSquare,
};
pub use tree::{Axis, DefaultTree, DefaultTreeConstructor, Tree, TreeConstructor};

/// The maximum number of shares supported by the Leopard codec in a 2D original data square.
pub const MAX_CHUNKS_LEOPARD: usize = 32768 * 32768;

/// Create a default tree constructor that creates DefaultTree instances.
pub fn new_default_tree_constructor() -> Arc<dyn TreeConstructor> {
    Arc::new(DefaultTreeConstructor::new())
}

/// Legacy function for backward compatibility - creates a default tree.
/// Prefer using `new_default_tree_constructor()` for new code.
pub fn new_default_tree(_axis: Axis, _index: usize) -> Box<dyn Tree> {
    Box::new(DefaultTree::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_basic_functionality() {
        // Create some test data
        let share_size = 64; // Must be multiple of 64 for Leopard codec
        let ones = vec![1u8; share_size];
        let twos = vec![2u8; share_size];
        let threes = vec![3u8; share_size];
        let fours = vec![4u8; share_size];

        let data = vec![ones, twos, threes, fours];
        let codec = Arc::new(LeoRSCodec::new());

        // Compute extended data square
        let result = compute_extended_data_square(data, codec, new_default_tree_constructor());
        assert!(
            result.is_ok(),
            "Failed to compute extended data square: {:?}",
            result.err()
        );

        let mut eds = result.unwrap();
        assert_eq!(eds.width(), 4); // 2x2 original becomes 4x4 extended

        // Test getting row and column roots
        let row_roots_result = eds.row_roots();
        assert!(
            row_roots_result.is_ok(),
            "Failed to get row roots: {:?}",
            row_roots_result.err()
        );

        let col_roots_result = eds.col_roots();
        assert!(
            col_roots_result.is_ok(),
            "Failed to get column roots: {:?}",
            col_roots_result.err()
        );

        let row_roots = row_roots_result.unwrap();
        let col_roots = col_roots_result.unwrap();

        assert_eq!(row_roots.len(), 4);
        assert_eq!(col_roots.len(), 4);

        println!("Basic functionality test passed!");
    }

    #[test]
    fn test_codec_validation() {
        let codec = LeoRSCodec::new();

        // Valid share size (multiple of 64)
        assert!(codec.validate_chunk_size(64).is_ok());
        assert!(codec.validate_chunk_size(128).is_ok());

        // Invalid share size (not multiple of 64)
        assert!(codec.validate_chunk_size(63).is_err());
        assert!(codec.validate_chunk_size(65).is_err());
    }

    #[test]
    fn test_tree_functionality() {
        let mut tree = DefaultTree::new();

        // Push some data
        tree.push(b"hello").unwrap();
        tree.push(b"world").unwrap();

        // Get root
        let root = tree.root().unwrap();
        assert_eq!(root.len(), 32); // SHA-256 produces 32-byte hashes

        // Root should be deterministic
        let mut tree2 = DefaultTree::new();
        tree2.push(b"hello").unwrap();
        tree2.push(b"world").unwrap();
        let root2 = tree2.root().unwrap();
        assert_eq!(root, root2);
    }
}
