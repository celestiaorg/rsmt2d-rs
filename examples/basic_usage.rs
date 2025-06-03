use rsmt2d::{compute_extended_data_square, new_default_tree_constructor, LeoRSCodec};
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

    // Keep copies for verification
    let expected_shares = vec![ones.clone(), twos.clone(), threes.clone(), fours.clone()];

    // Compute parity shares
    let mut eds = compute_extended_data_square(
        vec![ones, twos, threes, fours],
        codec.clone(),
        new_default_tree_constructor(),
    )?;

    println!("Extended data square computed successfully!");
    println!("Width: {}", eds.width());

    let row_roots = eds.row_roots()?;
    let col_roots = eds.col_roots()?;

    println!("Row roots: {} roots computed", row_roots.len());
    println!("Column roots: {} roots computed", col_roots.len());

    let flattened = eds.flattened();
    println!("Flattened EDS has {} shares", flattened.len());

    // Test that we can get the original data square
    let ods = eds.flattened_ods();
    println!("Original data square has {} shares", ods.len());

    // Verify that original shares are present and correct
    for (i, expected) in expected_shares.iter().enumerate() {
        if let Some(ref actual) = ods[i] {
            assert_eq!(actual, expected, "Share {} doesn't match", i);
        } else {
            panic!("Share {} is missing", i);
        }
    }

    println!("All original shares verified successfully!");

    // TODO: Demonstrate repair functionality
    // This would involve:
    // 1. Deleting some shares from the flattened data
    // 2. Re-importing the incomplete EDS
    // 3. Using repair() to reconstruct missing shares

    Ok(())
}
