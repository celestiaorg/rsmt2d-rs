use crate::traits::Tree;
use sha2::{Digest, Sha256};

/// Default merkle tree implementation for computing roots
pub struct DefaultTree {
    leaves: Vec<Vec<u8>>,
}

impl DefaultTree {
    /// Create a new default merkle tree
    pub fn new() -> Self {
        Self {
            leaves: Vec::new(),
        }
    }

    /// Compute hash of two child nodes
    fn hash_nodes(left: &[u8], right: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(left);
        hasher.update(right);
        hasher.finalize().to_vec()
    }

    /// Hash a single piece of data
    fn hash_data(data: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }

    /// Compute merkle root from current leaves
    fn compute_root(&self) -> Vec<u8> {
        if self.leaves.is_empty() {
            return Self::hash_data(&[]);
        }

        if self.leaves.len() == 1 {
            return Self::hash_data(&self.leaves[0]);
        }

        // Hash all leaves first
        let mut level: Vec<Vec<u8>> = self.leaves.iter()
            .map(|leaf| Self::hash_data(leaf))
            .collect();

        // Build tree bottom-up
        while level.len() > 1 {
            let mut next_level = Vec::new();
            
            // Process pairs
            for chunk in level.chunks(2) {
                if chunk.len() == 2 {
                    next_level.push(Self::hash_nodes(&chunk[0], &chunk[1]));
                } else {
                    // Odd number of nodes - duplicate the last one
                    next_level.push(Self::hash_nodes(&chunk[0], &chunk[0]));
                }
            }
            
            level = next_level;
        }

        level.into_iter().next().unwrap_or_else(|| Self::hash_data(&[]))
    }
}

impl Default for DefaultTree {
    fn default() -> Self {
        Self::new()
    }
}

impl Tree for DefaultTree {
    fn push(&mut self, data: &[u8]) {
        self.leaves.push(data.to_vec());
    }

    fn root(&self) -> Vec<u8> {
        self.compute_root()
    }

    fn reset(&mut self) {
        self.leaves.clear();
    }

    fn len(&self) -> usize {
        self.leaves.len()
    }
}

/// Factory function to create new default merkle tree
pub fn new_default_tree() -> DefaultTree {
    DefaultTree::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_tree_root() {
        let tree = new_default_tree();
        let root = tree.root();
        
        // Empty tree should have a consistent root
        let expected = DefaultTree::hash_data(&[]);
        assert_eq!(root, expected);
    }

    #[test]
    fn test_single_leaf_root() {
        let mut tree = new_default_tree();
        let data = b"hello world";
        tree.push(data);
        
        let root = tree.root();
        let expected = DefaultTree::hash_data(data);
        assert_eq!(root, expected);
    }

    #[test]
    fn test_multiple_leaves_root() {
        let mut tree = new_default_tree();
        tree.push(b"data1");
        tree.push(b"data2");
        tree.push(b"data3");
        
        let root = tree.root();
        assert!(!root.is_empty());
        
        // Root should be 32 bytes (SHA256)
        assert_eq!(root.len(), 32);
    }

    #[test]
    fn test_deterministic_roots() {
        let mut tree1 = new_default_tree();
        tree1.push(b"data1");
        tree1.push(b"data2");
        tree1.push(b"data3");
        
        let mut tree2 = new_default_tree();
        tree2.push(b"data1");
        tree2.push(b"data2");
        tree2.push(b"data3");
        
        assert_eq!(tree1.root(), tree2.root());
    }

    #[test]
    fn test_tree_operations() {
        let mut tree = new_default_tree();
        assert!(tree.is_empty());
        assert_eq!(tree.len(), 0);
        
        tree.push(b"test");
        assert!(!tree.is_empty());
        assert_eq!(tree.len(), 1);
        
        tree.push(b"another");
        assert_eq!(tree.len(), 2);
        
        let root_before_reset = tree.root();
        tree.reset();
        assert!(tree.is_empty());
        assert_eq!(tree.len(), 0);
        
        // Root should be different after reset
        let root_after_reset = tree.root();
        assert_ne!(root_before_reset, root_after_reset);
    }

    #[test]
    fn test_power_of_two_vs_odd_leaves() {
        // Test with power of 2 number of leaves
        let mut tree_even = new_default_tree();
        tree_even.push(b"1");
        tree_even.push(b"2");
        tree_even.push(b"3");
        tree_even.push(b"4");
        let root_even = tree_even.root();
        
        // Test with odd number of leaves
        let mut tree_odd = new_default_tree();
        tree_odd.push(b"1");
        tree_odd.push(b"2");
        tree_odd.push(b"3");
        let root_odd = tree_odd.root();
        
        // Roots should be different
        assert_ne!(root_even, root_odd);
        
        // Both should be valid SHA256 hashes
        assert_eq!(root_even.len(), 32);
        assert_eq!(root_odd.len(), 32);
    }

    #[test]
    fn test_tree_consistency_with_duplicates() {
        let mut tree1 = new_default_tree();
        tree1.push(b"data");
        let root1 = tree1.root();
        
        let mut tree2 = new_default_tree();
        tree2.push(b"data");
        tree2.push(b"data"); // Duplicate to test odd-node handling
        let root2 = tree2.root();
        
        // Should be different due to different tree structure
        assert_ne!(root1, root2);
    }

    #[test]
    fn test_large_tree() {
        let mut tree = new_default_tree();
        
        // Add many leaves
        for i in 0u32..1000 {
            tree.push(&i.to_be_bytes());
        }
        
        let root = tree.root();
        assert_eq!(root.len(), 32);
        assert_eq!(tree.len(), 1000);
        
        // Should be deterministic
        let root2 = tree.root();
        assert_eq!(root, root2);
    }
}