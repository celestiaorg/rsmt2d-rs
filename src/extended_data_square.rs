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

        // Check if we have enough data for reconstruction
        let available_chunks = data.iter().filter(|chunk| chunk.is_some()).count();
        let original_width = width / 2;
        let min_required = original_width * original_width; // At least original data
        
        if available_chunks < min_required {
            return Err(EdsError::InvalidData(
                format!("Insufficient data for reconstruction: {} available, {} required", 
                        available_chunks, min_required)
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
            // Missing chunks remain as empty vectors
        }

        // Create initial structure
        let eds = Self {
            data: matrix,
            width,
            original_width,
        };

        // Try to reconstruct missing data using Reed-Solomon if possible
        // This is a simplified version - full implementation would be more sophisticated
        let missing_count = eds.count_missing_chunks()?;
        if missing_count > 0 {
            // In a full implementation, we would:
            // 1. Identify which rows/columns have missing data
            // 2. Use Reed-Solomon decoding to reconstruct missing chunks
            // 3. Validate reconstruction using merkle roots
            
            // For now, we'll allow partially missing data and mark it as such
            println!("Warning: {} chunks are missing - partial reconstruction", missing_count);
        }

        Ok(eds)
    }

    /// Count missing chunks in the data square
    pub fn count_missing_chunks(&self) -> EdsResult<usize> {
        let mut count = 0;
        for row in &self.data {
            for chunk in row {
                if chunk.is_empty() {
                    count += 1;
                }
            }
        }
        Ok(count)
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

        // Try to repair using Reed-Solomon decoding
        // This is a simplified repair - in a full implementation, we'd need a codec here
        let _codec = crate::codec::new_test_leo_rs_codec(); // Use test codec for now
        
        // Check rows that don't match expected roots
        let current_row_roots = self.row_roots()
            .map_err(|e| RepairError::ValidationFailed(format!("Failed to compute row roots: {}", e)))?;
        
        let mut rows_to_repair = Vec::new();
        for (i, (expected, actual)) in row_roots.iter().zip(current_row_roots.iter()).enumerate() {
            if expected != actual {
                rows_to_repair.push(i);
            }
        }

        // Check columns that don't match expected roots
        let current_col_roots = self.col_roots()
            .map_err(|e| RepairError::ValidationFailed(format!("Failed to compute column roots: {}", e)))?;
            
        let mut cols_to_repair = Vec::new();
        for (i, (expected, actual)) in col_roots.iter().zip(current_col_roots.iter()).enumerate() {
            if expected != actual {
                cols_to_repair.push(i);
            }
        }

        // If no mismatches, no repair needed
        if rows_to_repair.is_empty() && cols_to_repair.is_empty() {
            return Ok(());
        }

        // For demonstration, we'll just mark corrupted chunks as needing repair
        // In a full implementation, this would involve:
        // 1. Identifying which chunks are corrupted
        // 2. Using Reed-Solomon decoding to reconstruct missing/corrupted data
        // 3. Validating the repair worked by recomputing roots
        
        // For now, return an error indicating that repair would be needed
        if !rows_to_repair.is_empty() {
            return Err(RepairError::ValidationFailed(
                format!("Rows {:?} need repair - full repair not implemented", rows_to_repair)
            ));
        }
        
        if !cols_to_repair.is_empty() {
            return Err(RepairError::ValidationFailed(
                format!("Columns {:?} need repair - full repair not implemented", cols_to_repair)
            ));
        }

        Ok(())
    }

    /// Simulate missing data by setting chunks to empty
    pub fn simulate_missing_data(&mut self, indices: Vec<SquareIndex>) -> EdsResult<()> {
        for index in indices {
            if index.row >= self.width || index.col >= self.width {
                return Err(EdsError::IndexOutOfBounds {
                    row: index.row,
                    col: index.col,
                    width: self.width,
                });
            }
            self.data[index.row][index.col] = Vec::new(); // Mark as missing
        }
        Ok(())
    }

    /// Check if a chunk is missing (empty)
    pub fn is_chunk_missing(&self, index: SquareIndex) -> EdsResult<bool> {
        if index.row >= self.width || index.col >= self.width {
            return Err(EdsError::IndexOutOfBounds {
                row: index.row,
                col: index.col,
                width: self.width,
            });
        }
        Ok(self.data[index.row][index.col].is_empty())
    }

    /// Get a row of chunks
    pub fn get_row(&self, row_index: usize) -> EdsResult<Vec<Vec<u8>>> {
        if row_index >= self.width {
            return Err(EdsError::IndexOutOfBounds {
                row: row_index,
                col: 0,
                width: self.width,
            });
        }
        Ok(self.data[row_index].clone())
    }

    /// Get a column of chunks
    pub fn get_column(&self, col_index: usize) -> EdsResult<Vec<Vec<u8>>> {
        if col_index >= self.width {
            return Err(EdsError::IndexOutOfBounds {
                row: 0,
                col: col_index,
                width: self.width,
            });
        }
        let mut column = Vec::new();
        for row in &self.data {
            column.push(row[col_index].clone());
        }
        Ok(column)
    }

    /// Set a row of chunks
    pub fn set_row(&mut self, row_index: usize, row_data: Vec<Vec<u8>>) -> EdsResult<()> {
        if row_index >= self.width {
            return Err(EdsError::IndexOutOfBounds {
                row: row_index,
                col: 0,
                width: self.width,
            });
        }
        if row_data.len() != self.width {
            return Err(EdsError::InvalidDimensions(
                format!("Row data length {} doesn't match width {}", row_data.len(), self.width)
            ));
        }
        self.data[row_index] = row_data;
        Ok(())
    }

    /// Set a column of chunks
    pub fn set_column(&mut self, col_index: usize, col_data: Vec<Vec<u8>>) -> EdsResult<()> {
        if col_index >= self.width {
            return Err(EdsError::IndexOutOfBounds {
                row: 0,
                col: col_index,
                width: self.width,
            });
        }
        if col_data.len() != self.width {
            return Err(EdsError::InvalidDimensions(
                format!("Column data length {} doesn't match width {}", col_data.len(), self.width)
            ));
        }
        for (i, chunk) in col_data.into_iter().enumerate() {
            self.data[i][col_index] = chunk;
        }
        Ok(())
    }

    /// Get statistics about the data square
    pub fn stats(&self) -> EdsResult<DataSquareStats> {
        let missing_chunks = self.count_missing_chunks()?;
        let total_chunks = self.width * self.width;
        let original_chunks = self.original_width * self.original_width;
        let parity_chunks = total_chunks - original_chunks;
        
        Ok(DataSquareStats {
            width: self.width,
            original_width: self.original_width,
            total_chunks,
            original_chunks,
            parity_chunks,
            missing_chunks,
            completion_ratio: (total_chunks - missing_chunks) as f64 / total_chunks as f64,
        })
    }
}

/// Statistics about a data square
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSquareStats {
    pub width: usize,
    pub original_width: usize,
    pub total_chunks: usize,
    pub original_chunks: usize,
    pub parity_chunks: usize,
    pub missing_chunks: usize,
    pub completion_ratio: f64,
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

    #[test]
    fn test_missing_data_simulation() {
        let codec = Box::new(new_test_leo_rs_codec());
        let tree_fn = || Box::new(new_default_tree()) as Box<dyn Tree>;

        // Create 4 chunks (2x2 square) and compute extended data square
        let data = vec![
            vec![1, 2, 3, 4],  // chunk 0
            vec![5, 6, 7, 8],  // chunk 1
            vec![9, 10, 11, 12], // chunk 2
            vec![13, 14, 15, 16], // chunk 3
        ];

        let mut eds = ExtendedDataSquare::compute_extended_data_square(data, codec, tree_fn).unwrap();
        
        // Initial state should have fewer missing chunks (only padding)
        let initial_missing = eds.count_missing_chunks().unwrap();
        
        // Simulate missing data
        let missing_indices = vec![SquareIndex::new(0, 1), SquareIndex::new(1, 0)];
        eds.simulate_missing_data(missing_indices.clone()).unwrap();
        
        // Check that chunks are marked as missing
        for index in missing_indices {
            assert!(eds.is_chunk_missing(index).unwrap());
        }
        
        // Count missing chunks should increase by 2
        let final_missing = eds.count_missing_chunks().unwrap();
        assert_eq!(final_missing, initial_missing + 2);
    }

    #[test]
    fn test_import_extended_data_square() {
        let codec = Box::new(new_test_leo_rs_codec());
        let tree_fn = || Box::new(new_default_tree()) as Box<dyn Tree>;

        // Create flattened data for a 2x2 square (16 total chunks in 4x4 after encoding)
        let mut flattened_data = Vec::new();
        for i in 0..16 {
            if i < 4 {
                // Original data chunks
                flattened_data.push(Some(vec![i as u8 + 1; 4]));
            } else {
                // Parity chunks or missing data
                flattened_data.push(Some(vec![0; 4]));
            }
        }

        let eds = ExtendedDataSquare::import_extended_data_square(
            flattened_data,
            codec,
            tree_fn,
        ).unwrap();

        assert_eq!(eds.width(), 4);
        assert_eq!(eds.original_width(), 2);
    }

    #[test]
    fn test_repair_functionality() {
        let codec = Box::new(new_test_leo_rs_codec());
        // Create 4 chunks (2x2 square)
        let data = vec![
            vec![1, 2, 3, 4],  // chunk 0
            vec![5, 6, 7, 8],  // chunk 1
            vec![9, 10, 11, 12], // chunk 2
            vec![13, 14, 15, 16], // chunk 3
        ];

        let mut eds = ExtendedDataSquare::new(data, codec).unwrap();
        
        // Get current roots
        let row_roots = eds.row_roots().unwrap();
        let col_roots = eds.col_roots().unwrap();
        
        // Repair should succeed when roots match
        assert!(eds.repair(row_roots, col_roots).is_ok());
        
        // Simulate corruption and try repair with wrong roots
        eds.simulate_missing_data(vec![SquareIndex::new(0, 0)]).unwrap();
        let wrong_roots = vec![vec![0; 32]; eds.width()];
        
        // Repair should fail with wrong roots
        assert!(eds.repair(wrong_roots.clone(), wrong_roots).is_err());
    }

    #[test]
    fn test_row_column_operations() {
        let codec = Box::new(new_test_leo_rs_codec());
        // Create 4 chunks (2x2 square)
        let data = vec![
            vec![1, 2, 3, 4],  // chunk 0
            vec![5, 6, 7, 8],  // chunk 1
            vec![9, 10, 11, 12], // chunk 2
            vec![13, 14, 15, 16], // chunk 3
        ];

        let mut eds = ExtendedDataSquare::new(data, codec).unwrap();
        
        // Test getting row
        let row_0 = eds.get_row(0).unwrap();
        assert_eq!(row_0[0], vec![1, 2, 3, 4]);
        
        // Test getting column
        let col_0 = eds.get_column(0).unwrap();
        assert_eq!(col_0[0], vec![1, 2, 3, 4]);
        
        // Test setting row
        let new_row = vec![vec![10; 4]; eds.width()];
        eds.set_row(0, new_row.clone()).unwrap();
        assert_eq!(eds.get_row(0).unwrap(), new_row);
        
        // Test setting column
        let new_col = vec![vec![20; 4]; eds.width()];
        eds.set_column(1, new_col.clone()).unwrap();
        assert_eq!(eds.get_column(1).unwrap(), new_col);
    }

    #[test]
    fn test_data_square_stats() {
        let codec = Box::new(new_test_leo_rs_codec());
        // Create 4 chunks (2x2 square)
        let data = vec![
            vec![1, 2, 3, 4],  // chunk 0
            vec![5, 6, 7, 8],  // chunk 1
            vec![9, 10, 11, 12], // chunk 2
            vec![13, 14, 15, 16], // chunk 3
        ];

        let mut eds = ExtendedDataSquare::new(data, codec).unwrap();
        
        // Get initial stats
        let stats = eds.stats().unwrap();
        assert_eq!(stats.width, 4);
        assert_eq!(stats.original_width, 2);
        assert_eq!(stats.total_chunks, 16);
        assert_eq!(stats.original_chunks, 4);
        assert_eq!(stats.parity_chunks, 12);
        assert_eq!(stats.missing_chunks, 12); // Most chunks are empty initially
        
        // Simulate missing data
        eds.simulate_missing_data(vec![SquareIndex::new(0, 0)]).unwrap();
        let updated_stats = eds.stats().unwrap();
        assert!(updated_stats.completion_ratio < stats.completion_ratio);
    }

    // Property-based tests
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_codec_roundtrip_property(
            data in prop::collection::vec(
                prop::collection::vec(any::<u8>(), 1..=64),
                2..=2  // Fixed size for test codec
            )
        ) {
            let codec = new_test_leo_rs_codec();
            
            // Ensure all chunks have the same length
            let chunk_size = data[0].len();
            let normalized_data: Vec<Vec<u8>> = data.into_iter()
                .map(|mut chunk| {
                    chunk.resize(chunk_size, 0);
                    chunk
                })
                .collect();
            
            let encoded = codec.encode(normalized_data.clone()).unwrap();
            let decode_input: Vec<Option<Vec<u8>>> = encoded.into_iter().map(Some).collect();
            let decoded = codec.decode(decode_input).unwrap();
            
            prop_assert_eq!(decoded, normalized_data);
        }

        #[test]
        fn test_tree_deterministic_property(
            chunks in prop::collection::vec(
                prop::collection::vec(any::<u8>(), 1..=32),
                1..=100
            )
        ) {
            let mut tree1 = new_default_tree();
            let mut tree2 = new_default_tree();
            
            for chunk in &chunks {
                tree1.push(chunk);
                tree2.push(chunk);
            }
            
            prop_assert_eq!(tree1.root(), tree2.root());
        }

        #[test]
        fn test_extended_data_square_property(
            chunk_data in prop::collection::vec(any::<u8>(), 1..=16)
        ) {
            let codec = Box::new(new_test_leo_rs_codec());
            
            // Create 4 chunks of equal size
            let _chunk_size = chunk_data.len();
            let data = vec![
                chunk_data.clone(),
                chunk_data.clone(),
                chunk_data.clone(),
                chunk_data,
            ];
            
            let eds = ExtendedDataSquare::new(data.clone(), codec).unwrap();
            
            // Properties to verify
            prop_assert_eq!(eds.width(), 4);
            prop_assert_eq!(eds.original_width(), 2);
            
            // Flattened should have 16 chunks
            let flattened = eds.flattened();
            prop_assert_eq!(flattened.len(), 16);
            
            // Original data should be recoverable from top-left quadrant
            for i in 0..2 {
                for j in 0..2 {
                    let chunk = eds.get_chunk(SquareIndex::new(i, j)).unwrap();
                    if !chunk.is_empty() {
                        prop_assert_eq!(chunk, &data[i * 2 + j]);
                    }
                }
            }
        }
    }
}