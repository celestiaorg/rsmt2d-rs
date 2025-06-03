use rsmt2d::*;
use std::sync::Arc;

#[test]
fn test_repair_functionality() {
    let share_size = 64;
    let codec = Arc::new(LeoRSCodec::new());

    // Create test data
    let test_data = vec![
        vec![1u8; share_size],
        vec![2u8; share_size],
        vec![3u8; share_size],
        vec![4u8; share_size],
    ];
    let original_data = test_data.clone();

    // Compute EDS
    let mut eds = compute_extended_data_square(test_data, codec.clone(), new_default_tree_constructor())
        .expect("Failed to compute EDS");

    let row_roots = eds.row_roots().expect("Failed to get row roots");
    let col_roots = eds.col_roots().expect("Failed to get col roots");

    // Damage some data
    let mut flattened = eds.flattened();
    flattened[0] = None; // Remove some shares
    flattened[2] = None;
    flattened[4] = None;
    flattened[6] = None;

    // Import damaged data
    let mut damaged_eds = import_extended_data_square(flattened, codec, new_default_tree_constructor())
        .expect("Failed to import damaged EDS");

    // Repair
    damaged_eds
        .repair(&row_roots, &col_roots)
        .expect("Failed to repair EDS");

    // Verify repair
    let repaired_ods = damaged_eds.flattened_ods();
    for (i, expected) in original_data.iter().enumerate() {
        if let Some(ref actual) = repaired_ods[i] {
            assert_eq!(actual, expected, "Share {} not repaired correctly", i);
        } else {
            panic!("Share {} missing after repair", i);
        }
    }
}

#[test]
fn test_invalid_share_size() {
    let codec = LeoRSCodec::new();

    // Test invalid share sizes (not multiples of 64)
    assert!(codec.validate_chunk_size(63).is_err());
    assert!(codec.validate_chunk_size(65).is_err());
    assert!(codec.validate_chunk_size(100).is_err());

    // Test valid share sizes
    assert!(codec.validate_chunk_size(64).is_ok());
    assert!(codec.validate_chunk_size(128).is_ok());
    assert!(codec.validate_chunk_size(256).is_ok());
}

#[test]
fn test_uneven_chunks_error() {
    let codec = Arc::new(LeoRSCodec::new());

    // Create data with uneven share sizes
    let data = vec![
        vec![1u8; 64],
        vec![2u8; 128], // Different size
        vec![3u8; 64],
        vec![4u8; 64],
    ];

    let result = compute_extended_data_square(data, codec, new_default_tree_constructor());
    assert!(result.is_err());
}

#[test]
fn test_data_square_operations() {
    let share_size = 64;
    let data = vec![
        Some(vec![1u8; share_size]),
        Some(vec![2u8; share_size]),
        Some(vec![3u8; share_size]),
        Some(vec![4u8; share_size]),
    ];

    let mut ds =
        DataSquare::new(data, new_default_tree_constructor(), share_size).expect("Failed to create DataSquare");

    // Test getting cells
    let cell_0_0 = ds.get_cell(0, 0);
    assert!(cell_0_0.is_some());
    assert_eq!(cell_0_0.unwrap(), vec![1u8; share_size]);

    // Test getting row and column
    let row_0 = ds.row(0);
    assert_eq!(row_0.len(), 2);
    assert!(row_0[0].is_some());
    assert!(row_0[1].is_some());

    let col_0 = ds.col(0);
    assert_eq!(col_0.len(), 2);
    assert!(col_0[0].is_some());
    assert!(col_0[1].is_some());

    // Test setting a cell
    let new_share = vec![99u8; share_size];
    let result = ds.set_cell(0, 0, new_share.clone());
    assert!(result.is_err()); // Should fail because cell is already set

    // Test getting roots
    let row_roots = ds.get_row_roots();
    assert!(row_roots.is_ok());
    let col_roots = ds.get_col_roots();
    assert!(col_roots.is_ok());
}

#[test]
fn test_eds_width_validation() {
    let codec = Arc::new(LeoRSCodec::new());

    // Odd widths should fail
    let result = ExtendedDataSquare::new(codec.clone(), new_default_tree_constructor(), 3, 64);
    assert!(result.is_err());

    // Even widths should succeed
    let result = ExtendedDataSquare::new(codec, new_default_tree_constructor(), 4, 64);
    assert!(result.is_ok());
}

#[test]
fn test_codec_encode_decode() {
    let codec = LeoRSCodec::new();
    let share_size = 64;

    let data = vec![vec![1u8; share_size], vec![2u8; share_size]];

    // Test encoding
    let parity = codec.encode(&data).expect("Encoding failed");
    assert_eq!(parity.len(), 2); // Should have same number of parity as data

    // Test decoding with missing data
    let mut combined: Vec<Option<Vec<u8>>> = Vec::new();
    combined.push(Some(data[0].clone()));
    combined.push(None); // Missing data share
    combined.push(Some(parity[0].clone()));
    combined.push(Some(parity[1].clone()));

    let decoded = codec.decode(&combined).expect("Decoding failed");
    assert_eq!(decoded.len(), 4);
    assert_eq!(decoded[0], data[0]);
    assert_eq!(decoded[1], data[1]); // Should be reconstructed
}
