use crate::error::{CodecError, CodecResult};
use crate::traits::Codec;
use reed_solomon_erasure::galois_8::ReedSolomon;

/// Leo Reed-Solomon codec implementation
pub struct LeoRSCodec {
    rs: ReedSolomon,
    data_shards: usize,
    parity_shards: usize,
}

impl LeoRSCodec {
    /// Create a new Leo Reed-Solomon codec with the specified parameters
    pub fn new(data_shards: usize, parity_shards: usize) -> CodecResult<Self> {
        let rs = ReedSolomon::new(data_shards, parity_shards)
            .map_err(|e| CodecError::EncodingFailed(format!("Failed to create Reed-Solomon codec: {}", e)))?;
        
        Ok(Self {
            rs,
            data_shards,
            parity_shards,
        })
    }
}

impl Codec for LeoRSCodec {
    fn encode(&self, data: Vec<Vec<u8>>) -> CodecResult<Vec<Vec<u8>>> {
        if data.len() != self.data_shards {
            return Err(CodecError::InvalidChunkSize {
                expected: self.data_shards,
                actual: data.len(),
            });
        }

        // Validate all chunks have the same size
        if let Some(first_chunk) = data.first() {
            let chunk_size = first_chunk.len();
            for (i, chunk) in data.iter().enumerate() {
                if chunk.len() != chunk_size {
                    return Err(CodecError::EncodingFailed(
                        format!("Chunk {} has size {} but expected {}", i, chunk.len(), chunk_size)
                    ));
                }
            }
        }

        // Convert to the format expected by reed-solomon-erasure
        let mut shards: Vec<Vec<u8>> = data;
        
        // Add empty parity shards
        if let Some(first_chunk) = shards.first() {
            let chunk_size = first_chunk.len();
            for _ in 0..self.parity_shards {
                shards.push(vec![0u8; chunk_size]);
            }
        }

        // Encode
        self.rs.encode(&mut shards)
            .map_err(|e| CodecError::EncodingFailed(format!("Reed-Solomon encoding failed: {}", e)))?;

        Ok(shards)
    }

    fn decode(&self, data: Vec<Option<Vec<u8>>>) -> CodecResult<Vec<Vec<u8>>> {
        if data.len() != self.data_shards + self.parity_shards {
            return Err(CodecError::InvalidChunkSize {
                expected: self.data_shards + self.parity_shards,
                actual: data.len(),
            });
        }

        // Count available shards
        let available_shards = data.iter().filter(|chunk| chunk.is_some()).count();
        if available_shards < self.data_shards {
            return Err(CodecError::InsufficientData);
        }

        // Determine chunk size from the first available chunk
        let chunk_size = data.iter()
            .find_map(|chunk| chunk.as_ref())
            .map(|chunk| chunk.len())
            .ok_or(CodecError::InsufficientData)?;

        // Convert to the format expected by reed-solomon-erasure
        let mut shards: Vec<Option<Vec<u8>>> = data;

        // Reconstruct missing shards
        self.rs.reconstruct(&mut shards)
            .map_err(|e| CodecError::DecodingFailed(format!("Reed-Solomon reconstruction failed: {}", e)))?;

        // Return only the data shards (first data_shards elements)
        Ok(shards.into_iter()
            .take(self.data_shards)
            .map(|shard| shard.unwrap_or_else(|| vec![0u8; chunk_size]))
            .collect())
    }

    fn max_chunks(&self) -> usize {
        self.data_shards
    }

    fn parity_chunks(&self) -> usize {
        self.parity_shards
    }
}

/// Factory function to create new Leo Reed-Solomon codec with default parameters
pub fn new_leo_rs_codec() -> LeoRSCodec {
    // Default to 16 data shards and 16 parity shards (common configuration)
    LeoRSCodec::new(16, 16).expect("Failed to create default Leo RS codec")
}

/// Factory function to create a smaller Leo Reed-Solomon codec for testing
pub fn new_test_leo_rs_codec() -> LeoRSCodec {
    // Use 2 data shards and 2 parity shards for testing
    LeoRSCodec::new(2, 2).expect("Failed to create test Leo RS codec")
}

/// Factory function to create a small Leo Reed-Solomon codec for small data
pub fn new_small_leo_rs_codec() -> LeoRSCodec {
    // Use 4 data shards and 4 parity shards for smaller data sets
    LeoRSCodec::new(4, 4).expect("Failed to create small Leo RS codec")
}

/// Factory function to create a medium Leo Reed-Solomon codec
pub fn new_medium_leo_rs_codec() -> LeoRSCodec {
    // Use 8 data shards and 8 parity shards for medium data sets
    LeoRSCodec::new(8, 8).expect("Failed to create medium Leo RS codec")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let codec = new_leo_rs_codec();
        let chunk_size = 32;
        
        // Create test data
        let mut data = Vec::new();
        for i in 0..codec.max_chunks() {
            let mut chunk = vec![i as u8; chunk_size];
            chunk[0] = i as u8; // Make chunks distinguishable
            data.push(chunk);
        }

        // Encode
        let encoded = codec.encode(data.clone()).unwrap();
        assert_eq!(encoded.len(), codec.max_chunks() + codec.parity_chunks());

        // Convert to Option format for decoding
        let to_decode: Vec<Option<Vec<u8>>> = encoded.into_iter().map(Some).collect();

        // Decode
        let decoded = codec.decode(to_decode).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_error_correction() {
        let codec = new_leo_rs_codec();
        let chunk_size = 32;
        
        // Create test data
        let mut data = Vec::new();
        for i in 0..codec.max_chunks() {
            let mut chunk = vec![i as u8; chunk_size];
            chunk[0] = i as u8;
            data.push(chunk);
        }

        // Encode
        let encoded = codec.encode(data.clone()).unwrap();

        // Simulate corruption by removing some chunks (within error correction capacity)
        let mut corrupted: Vec<Option<Vec<u8>>> = encoded.into_iter().map(Some).collect();
        
        // Remove up to parity_shards chunks
        let corruption_count = codec.parity_chunks().min(8); // Remove 8 chunks
        for i in 0..corruption_count {
            corrupted[i] = None;
        }

        // Should still be able to decode
        let decoded = codec.decode(corrupted).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_beyond_error_limit() {
        let codec = new_leo_rs_codec();
        let chunk_size = 32;
        
        // Create test data
        let mut data = Vec::new();
        for i in 0..codec.max_chunks() {
            data.push(vec![i as u8; chunk_size]);
        }

        // Encode
        let encoded = codec.encode(data).unwrap();

        // Simulate corruption beyond repair capacity
        let mut corrupted: Vec<Option<Vec<u8>>> = encoded.into_iter().map(Some).collect();
        
        // Remove more chunks than we can recover
        let corruption_count = codec.parity_chunks() + 1;
        for i in 0..corruption_count {
            corrupted[i] = None;
        }

        // Should fail to decode
        assert!(codec.decode(corrupted).is_err());
    }

    #[test]
    fn test_invalid_chunk_sizes() {
        let codec = new_leo_rs_codec();
        
        // Test encoding with wrong number of chunks
        let data = vec![vec![1, 2, 3]]; // Too few chunks
        assert!(codec.encode(data).is_err());
        
        // Test encoding with mismatched chunk sizes
        let mut data = Vec::new();
        for i in 0..codec.max_chunks() {
            let size = if i == 0 { 32 } else { 31 }; // One chunk different size
            data.push(vec![i as u8; size]);
        }
        assert!(codec.encode(data).is_err());
    }

    #[test]
    fn test_codec_parameters() {
        let codec = new_leo_rs_codec();
        assert_eq!(codec.max_chunks(), 16);
        assert_eq!(codec.parity_chunks(), 16);
    }
}