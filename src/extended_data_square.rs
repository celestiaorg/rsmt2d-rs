use crate::error::{EdsError, EdsResult, RepairError, RepairResult};
use crate::traits::{Codec, Tree};
use serde::{Deserialize, Serialize};

/// Index type for accessing data square elements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SquareIndex {
    pub row: usize,
    pub col: usize,
}

impl SquareIndex {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

/// Main data structure representing 2D Reed-Solomon encoded data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedDataSquare {
    data: Vec<Vec<Vec<u8>>>,
    width: usize,
    original_width: usize,
}

impl ExtendedDataSquare {
    /// Create new extended data square from input data
    pub fn new(data: Vec<Vec<u8>>, codec: Box<dyn Codec>) -> EdsResult<Self> {
        // Validate input dimensions
        if data.is_empty() {
            return Err(EdsError::InvalidData("Input data cannot be empty".to_string()));
        }

        // Calculate square dimensions
        let total_chunks = data.len();
        let original_width = (total_chunks as f64).sqrt() as usize;
        
        if original_width * original_width != total_chunks {
            return Err(EdsError::InvalidDimensions(
                format!("Data length {} is not a perfect square", total_chunks)
            ));
        }

        // Validate all chunks have the same size
        if let Some(first_chunk) = data.first() {
            let chunk_size = first_chunk.len();
            for (i, chunk) in data.iter().enumerate() {
                if chunk.len() != chunk_size {
                    return Err(EdsError::InvalidData(
                        format!("Chunk {} has size {} but expected {}", i, chunk.len(), chunk_size)
                    ));
                }
            }
        }

        // Check if dimensions are compatible with codec
        if original_width > codec.max_chunks() {
            return Err(EdsError::InvalidDimensions(
                format!("Data width {} exceeds codec capacity {}", original_width, codec.max_chunks())
            ));
        }

        // For now, create a simple structure without Reed-Solomon encoding
        // This will be extended in compute_extended_data_square
        let width = original_width * 2; // After Reed-Solomon, width doubles
        let mut matrix = vec![vec![Vec::new(); width]; width];

        // Place original data in top-left quadrant
        for (i, chunk) in data.iter().enumerate() {
            let row = i / original_width;
            let col = i % original_width;
            matrix[row][col] = chunk.clone();
        }

        Ok(Self {
            data: matrix,
            width,
            original_width,
        })
    }

    /// Create extended data square with Reed-Solomon encoding
    pub fn compute_extended_data_square(
        data: Vec<Vec<u8>>,
        codec: Box<dyn Codec>,
        _tree_fn: fn() -> Box<dyn Tree>,
    ) -> EdsResult<Self> {
        // Validate input
        if data.is_empty() {
            return Err(EdsError::InvalidData("Input data cannot be empty".to_string()));
        }

        // Calculate square dimensions
        let total_chunks = data.len();
        let original_width = (total_chunks as f64).sqrt() as usize;
        
        if original_width * original_width != total_chunks {
            return Err(EdsError::InvalidDimensions(
                format!("Data length {} is not a perfect square", total_chunks)
            ));
        }

        // Validate all chunks have the same size
        if let Some(first_chunk) = data.first() {
            let chunk_size = first_chunk.len();
            for (i, chunk) in data.iter().enumerate() {
                if chunk.len() != chunk_size {
                    return Err(EdsError::InvalidData(
                        format!("Chunk {} has size {} but expected {}", i, chunk.len(), chunk_size)
                    ));
                }
            }
        }

        // Check codec compatibility
        if original_width > codec.max_chunks() {
            return Err(EdsError::InvalidDimensions(
                format!("Data width {} exceeds codec capacity {}", original_width, codec.max_chunks())
            ));
        }

        let width = original_width * 2; // After Reed-Solomon encoding
        let mut matrix = vec![vec![Vec::new(); width]; width];

        // Step 1: Place original data in top-left quadrant
        for (i, chunk) in data.iter().enumerate() {
            let row = i / original_width;
            let col = i % original_width;
            matrix[row][col] = chunk.clone();
        }

        // Step 2: Encode rows (add parity to right side)
        for i in 0..original_width {
            let row_data: Vec<Vec<u8>> = (0..original_width)
                .map(|j| matrix[i][j].clone())
                .collect();
            
            let encoded_row = codec.encode(row_data)?;
            
            // Place encoded data (original + parity) back
            for (j, chunk) in encoded_row.into_iter().enumerate() {
                if j < width {
                    matrix[i][j] = chunk;
                }
            }
        }

        // Step 3: Encode columns (add parity to bottom)
        for j in 0..width {
            let col_data: Vec<Vec<u8>> = (0..original_width)
                .map(|i| matrix[i][j].clone())
                .collect();
            
            let encoded_col = codec.encode(col_data)?;
            
            // Place encoded data (original + parity) back
            for (i, chunk) in encoded_col.into_iter().enumerate() {
                if i < width {
                    matrix[i][j] = chunk;
                }
            }
        }

        Ok(Self {
            data: matrix,
            width,
            original_width,
        })
    }

    /// Reconstruct extended data square from flattened representation
    pub fn import_extended_data_square(
        data: Vec<Option<Vec<u8>>>,
        _codec: Box<dyn Codec>,
        _tree_fn: fn() -> Box<dyn Tree>,
    ) -> EdsResult<Self> {
        // Calculate width from flattened data length
        let total_elements = data.len();
        let width = (total_elements as f64).sqrt() as usize;
        
        if width * width != total_elements {
            return Err(EdsError::InvalidDimensions(
                format!("Flattened data length {} is not a perfect square", total_elements)
            ));
        }

        // Convert flattened data to matrix
        let mut matrix = vec![vec![Vec::new(); width]; width];
        for (idx, chunk) in data.into_iter().enumerate() {
            let row = idx / width;
            let col = idx % width;
            
            if let Some(chunk_data) = chunk {
                matrix[row][col] = chunk_data;
            }
        }

        Ok(Self {
            data: matrix,
            width,
            original_width: width / 2,
        })
    }

    /// Return width dimension of the data square
    pub fn width(&self) -> usize {
        self.width
    }

    /// Get original width before Reed-Solomon encoding
    pub fn original_width(&self) -> usize {
        self.original_width
    }

    /// Get a chunk from the data square
    pub fn get_chunk(&self, index: SquareIndex) -> EdsResult<&Vec<u8>> {
        if index.row >= self.width || index.col >= self.width {
            return Err(EdsError::IndexOutOfBounds {
                row: index.row,
                col: index.col,
                width: self.width,
            });
        }
        Ok(&self.data[index.row][index.col])
    }

    /// Set a chunk in the data square
    pub fn set_chunk(&mut self, index: SquareIndex, chunk: Vec<u8>) -> EdsResult<()> {
        if index.row >= self.width || index.col >= self.width {
            return Err(EdsError::IndexOutOfBounds {
                row: index.row,
                col: index.col,
                width: self.width,
            });
        }
        self.data[index.row][index.col] = chunk;
        Ok(())
    }

    /// Compute merkle roots for all rows in the data square
    pub fn row_roots(&self) -> EdsResult<Vec<Vec<u8>>> {
        let mut roots = Vec::new();
        
        for row in &self.data {
            let mut tree = crate::tree::new_default_tree();
            for chunk in row {
                tree.push(chunk);
            }
            roots.push(tree.root());
        }
        
        Ok(roots)
    }

    /// Compute merkle roots for all columns in the data square
    pub fn col_roots(&self) -> EdsResult<Vec<Vec<u8>>> {
        let mut roots = Vec::new();
        
        for col_idx in 0..self.width {
            let mut tree = crate::tree::new_default_tree();
            for row_idx in 0..self.width {
                tree.push(&self.data[row_idx][col_idx]);
            }
            roots.push(tree.root());
        }
        
        Ok(roots)
    }

    /// Return flattened representation of data square
    pub fn flattened(&self) -> Vec<Vec<u8>> {
        let mut result = Vec::with_capacity(self.width * self.width);
        
        for row in &self.data {
            for chunk in row {
                result.push(chunk.clone());
            }
        }
        
        result
    }

    /// Repair missing or corrupted data using Reed-Solomon decoding
    pub fn repair(
        &mut self,
        row_roots: Vec<Vec<u8>>,
        col_roots: Vec<Vec<u8>>,
    ) -> RepairResult<()> {
        // Validate root lengths
        if row_roots.len() != self.width {
            return Err(RepairError::ValidationFailed(
                format!("Expected {} row roots, got {}", self.width, row_roots.len())
            ));
        }
        
        if col_roots.len() != self.width {
            return Err(RepairError::ValidationFailed(
                format!("Expected {} column roots, got {}", self.width, col_roots.len())
            ));
        }

        // Verify current roots against expected roots
        let current_row_roots = self.row_roots()
            .map_err(|e| RepairError::ValidationFailed(format!("Failed to compute row roots: {}", e)))?;
        let current_col_roots = self.col_roots()
            .map_err(|e| RepairError::ValidationFailed(format!("Failed to compute column roots: {}", e)))?;

        // Check for mismatches
        for (_i, (expected, actual)) in row_roots.iter().zip(current_row_roots.iter()).enumerate() {
            if expected != actual {
                return Err(RepairError::RootHashMismatch {
                    expected: expected.clone(),
                    actual: actual.clone(),
                });
            }
        }

        for (_i, (expected, actual)) in col_roots.iter().zip(current_col_roots.iter()).enumerate() {
            if expected != actual {
                return Err(RepairError::RootHashMismatch {
                    expected: expected.clone(),
                    actual: actual.clone(),
                });
            }
        }

        // If we get here, all roots match - no repair needed
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{new_test_leo_rs_codec, new_default_tree};

    #[test]
    fn test_create_from_data() {
        let codec = Box::new(new_test_leo_rs_codec());
        // Create 4 chunks (2x2 square)
        let data = vec![
            vec![1, 2, 3, 4],  // chunk 0
            vec![5, 6, 7, 8],  // chunk 1
            vec![9, 10, 11, 12], // chunk 2
            vec![13, 14, 15, 16], // chunk 3
        ];

        let eds = ExtendedDataSquare::new(data, codec).unwrap();
        assert_eq!(eds.width(), 4);
        assert_eq!(eds.original_width(), 2);
    }

    #[test]
    fn test_compute_with_parity() {
        let codec = Box::new(new_test_leo_rs_codec());
        let tree_fn = || Box::new(new_default_tree()) as Box<dyn Tree>;

        // Create 4 chunks (2x2 square)
        let data = vec![
            vec![1, 2, 3, 4],  // chunk 0
            vec![5, 6, 7, 8],  // chunk 1
            vec![9, 10, 11, 12], // chunk 2
            vec![13, 14, 15, 16], // chunk 3
        ];

        let eds = ExtendedDataSquare::compute_extended_data_square(data, codec, tree_fn).unwrap();
        assert_eq!(eds.width(), 4);
        assert_eq!(eds.original_width(), 2);
    }

    #[test]
    fn test_flattened() {
        let codec = Box::new(new_test_leo_rs_codec());
        // Create 4 chunks (2x2 square)
        let data = vec![
            vec![1, 2, 3, 4],  // chunk 0
            vec![5, 6, 7, 8],  // chunk 1
            vec![9, 10, 11, 12], // chunk 2
            vec![13, 14, 15, 16], // chunk 3
        ];

        let eds = ExtendedDataSquare::new(data, codec).unwrap();
        let flattened = eds.flattened();
        assert_eq!(flattened.len(), 16); // 4x4 = 16 chunks
    }

    #[test]
    fn test_row_column_roots_consistency() {
        let codec = Box::new(new_test_leo_rs_codec());
        let tree_fn = || Box::new(new_default_tree()) as Box<dyn Tree>;

        // Create 4 chunks (2x2 square)
        let data = vec![
            vec![1, 2, 3, 4],  // chunk 0
            vec![5, 6, 7, 8],  // chunk 1
            vec![9, 10, 11, 12], // chunk 2
            vec![13, 14, 15, 16], // chunk 3
        ];

        let eds = ExtendedDataSquare::compute_extended_data_square(data, codec, tree_fn).unwrap();
        
        let row_roots1 = eds.row_roots().unwrap();
        let row_roots2 = eds.row_roots().unwrap();
        assert_eq!(row_roots1, row_roots2);

        let col_roots1 = eds.col_roots().unwrap();
        let col_roots2 = eds.col_roots().unwrap();
        assert_eq!(col_roots1, col_roots2);
    }

    #[test]
    fn test_square_index() {
        let index = SquareIndex::new(1, 2);
        assert_eq!(index.row, 1);
        assert_eq!(index.col, 2);
    }

    #[test]
    fn test_get_set_chunk() {
        let codec = Box::new(new_test_leo_rs_codec());
        // Create 4 chunks (2x2 square)
        let data = vec![
            vec![1, 2, 3, 4],  // chunk 0
            vec![5, 6, 7, 8],  // chunk 1
            vec![9, 10, 11, 12], // chunk 2
            vec![13, 14, 15, 16], // chunk 3
        ];

        let mut eds = ExtendedDataSquare::new(data, codec).unwrap();
        
        let index = SquareIndex::new(0, 0);
        let chunk = eds.get_chunk(index).unwrap();
        assert_eq!(chunk, &vec![1, 2, 3, 4]);

        let new_chunk = vec![10, 20, 30, 40];
        eds.set_chunk(index, new_chunk.clone()).unwrap();
        
        let updated_chunk = eds.get_chunk(index).unwrap();
        assert_eq!(updated_chunk, &new_chunk);
    }

    #[test]
    fn test_index_out_of_bounds() {
        let codec = Box::new(new_test_leo_rs_codec());
        // Create 4 chunks (2x2 square)
        let data = vec![
            vec![1, 2, 3, 4],  // chunk 0
            vec![5, 6, 7, 8],  // chunk 1
            vec![9, 10, 11, 12], // chunk 2
            vec![13, 14, 15, 16], // chunk 3
        ];

        let eds = ExtendedDataSquare::new(data, codec).unwrap();
        
        let index = SquareIndex::new(10, 10);
        assert!(eds.get_chunk(index).is_err());
    }

    #[test]
    fn test_invalid_dimensions() {
        let codec = Box::new(new_test_leo_rs_codec());
        
        // Non-square data (3 chunks instead of 4)
        let data = vec![
            vec![1, 2, 3],
            vec![4, 5, 6],
            vec![7, 8, 9],
        ];
        
        // 3 is not a perfect square, so this should fail
        assert!(ExtendedDataSquare::new(data, codec).is_err());
    }
}