use crate::error::CodecResult;

/// Abstract interface for Reed-Solomon codec implementations
pub trait Codec {
    /// Encode data chunks with Reed-Solomon parity
    /// Takes original data and returns data + parity chunks
    fn encode(&self, data: Vec<Vec<u8>>) -> CodecResult<Vec<Vec<u8>>>;
    
    /// Decode and repair data from potentially corrupted chunks
    /// Takes data + parity chunks (some may be None for missing data)
    /// Returns original data chunks
    fn decode(&self, data: Vec<Option<Vec<u8>>>) -> CodecResult<Vec<Vec<u8>>>;
    
    /// Get the maximum number of data chunks this codec can handle
    fn max_chunks(&self) -> usize;
    
    /// Get the number of parity chunks this codec generates
    fn parity_chunks(&self) -> usize;
}

/// Abstract interface for merkle tree operations
pub trait Tree {
    /// Add data to merkle tree for root computation
    fn push(&mut self, data: &[u8]);
    
    /// Compute and return merkle root hash
    fn root(&self) -> Vec<u8>;
    
    /// Reset the tree to empty state
    fn reset(&mut self);
    
    /// Get the number of leaves in the tree
    fn len(&self) -> usize;
    
    /// Check if the tree is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Mock implementations for testing
    struct MockCodec;
    
    impl Codec for MockCodec {
        fn encode(&self, data: Vec<Vec<u8>>) -> CodecResult<Vec<Vec<u8>>> {
            // Simple mock: just duplicate the input as "parity"
            let mut result = data.clone();
            result.extend(data);
            Ok(result)
        }
        
        fn decode(&self, data: Vec<Option<Vec<u8>>>) -> CodecResult<Vec<Vec<u8>>> {
            // Simple mock: just take the first half of non-None entries
            let non_none: Vec<_> = data.into_iter().flatten().collect();
            let half = non_none.len() / 2;
            Ok(non_none.into_iter().take(half).collect())
        }
        
        fn max_chunks(&self) -> usize { 16 }
        fn parity_chunks(&self) -> usize { 16 }
    }
    
    struct MockTree {
        data: Vec<Vec<u8>>,
    }
    
    impl MockTree {
        fn new() -> Self {
            Self { data: Vec::new() }
        }
    }
    
    impl Tree for MockTree {
        fn push(&mut self, data: &[u8]) {
            self.data.push(data.to_vec());
        }
        
        fn root(&self) -> Vec<u8> {
            // Simple mock: just concatenate all data and hash it
            let mut all_data = Vec::new();
            for chunk in &self.data {
                all_data.extend_from_slice(chunk);
            }
            all_data
        }
        
        fn reset(&mut self) {
            self.data.clear();
        }
        
        fn len(&self) -> usize {
            self.data.len()
        }
    }
    
    #[test]
    fn test_codec_trait() {
        let codec = MockCodec;
        let data = vec![vec![1, 2, 3], vec![4, 5, 6]];
        
        let encoded = codec.encode(data.clone()).unwrap();
        assert_eq!(encoded.len(), 4); // original + parity
        
        let to_decode = encoded.into_iter().map(Some).collect();
        let decoded = codec.decode(to_decode).unwrap();
        assert_eq!(decoded, data);
    }
    
    #[test]
    fn test_tree_trait() {
        let mut tree = MockTree::new();
        assert!(tree.is_empty());
        
        tree.push(&[1, 2, 3]);
        tree.push(&[4, 5, 6]);
        assert_eq!(tree.len(), 2);
        assert!(!tree.is_empty());
        
        let root1 = tree.root();
        let root2 = tree.root();
        assert_eq!(root1, root2); // deterministic
        
        tree.reset();
        assert!(tree.is_empty());
    }
}