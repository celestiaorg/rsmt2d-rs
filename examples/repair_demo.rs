use rsmt2d::{
    compute_extended_data_square, import_extended_data_square, new_default_tree_constructor, LeoRSCodec,
};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let share_size = 64; // Using smaller size for testing
    let codec = Arc::new(LeoRSCodec::new());

    let ones = vec![1u8; share_size];
    let twos = vec![2u8; share_size];
    let threes = vec![3u8; share_size];
    let fours = vec![4u8; share_size];

    // Compute parity shares
    let mut eds = compute_extended_data_square(
        vec![ones.clone(), twos.clone(), threes.clone(), fours.clone()],
        codec.clone(),
        new_default_tree_constructor(),
    )?;

    println!("Original extended data square computed successfully!");

    let row_roots = eds.row_roots()?;
    let col_roots = eds.col_roots()?;

    let mut flattened = eds.flattened();
    println!("Original flattened EDS has {} shares", flattened.len());

    // Delete some shares, just enough so that repairing is possible.
    // For a 4x4 EDS (16 shares total), we need to keep at least 8 shares (half) for repair
    println!("Deleting some shares to simulate data loss...");
    flattened[0] = None; // (0,0)
    flattened[2] = None; // (0,2)
    flattened[3] = None; // (0,3)
    flattened[4] = None; // (1,0)
    flattened[5] = None; // (1,1)
    flattened[6] = None; // (1,2)
    flattened[7] = None; // (1,3)

    let present_count = flattened.iter().filter(|x| x.is_some()).count();
    println!("Shares remaining: {}/16", present_count);

    // Re-import the data square.
    let mut incomplete_eds =
        import_extended_data_square(flattened, codec.clone(), new_default_tree_constructor())?;
    println!("Imported incomplete EDS");

    // Attempt repair
    println!("Attempting to repair the data square...");
    match incomplete_eds.repair(&row_roots, &col_roots) {
        Ok(()) => {
            println!("Repair successful!");

            // Verify repair by checking original data
            let repaired_ods = incomplete_eds.flattened_ods();
            let expected_shares = vec![ones, twos, threes, fours];

            for (i, expected) in expected_shares.iter().enumerate() {
                if let Some(ref actual) = repaired_ods[i] {
                    assert_eq!(
                        actual, expected,
                        "Repaired share {} doesn't match expected",
                        i
                    );
                } else {
                    panic!("Repaired share {} is missing", i);
                }
            }

            println!("All original shares successfully recovered!");
        }
        Err(e) => {
            println!("Repair failed: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
