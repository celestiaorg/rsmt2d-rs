use crate::Result;
use sha2::{Digest, Sha256};

/// Represents which axis (row or column) a tree is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Row,
    Col,
}

impl std::fmt::Display for Axis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Axis::Row => write!(f, "row"),
            Axis::Col => write!(f, "col"),
        }
    }
}

/// Type alias for tree constructor functions.
pub type TreeConstructorFn = fn(axis: Axis, index: usize) -> Box<dyn Tree>;

/// Tree wraps Merkle tree implementations to work with rsmt2d.
pub trait Tree: Send + Sync {
    /// Push data to the tree.
    fn push(&mut self, data: &[u8]) -> Result<()>;

    /// Compute and return the root of the tree.
    fn root(&mut self) -> Result<Vec<u8>>;
}

/// Default Merkle tree implementation using SHA-256.
pub struct DefaultTree {
    leaves: Vec<Vec<u8>>,
    root: Option<Vec<u8>>,
}

impl DefaultTree {
    /// Create a new DefaultTree.
    pub fn new() -> Self {
        Self {
            leaves: Vec::new(),
            root: None,
        }
    }

    /// Compute the Merkle root from the current leaves.
    fn compute_root(&mut self) -> Result<Vec<u8>> {
        if self.leaves.is_empty() {
            return Ok(vec![0; 32]); // Return zero hash for empty tree
        }

        let mut current_level = self.leaves.clone();

        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            for chunk in current_level.chunks(2) {
                let mut hasher = Sha256::new();

                if chunk.len() == 2 {
                    // Hash left + right
                    hasher.update(&chunk[0]);
                    hasher.update(&chunk[1]);
                } else {
                    // Odd number, hash single node with itself
                    hasher.update(&chunk[0]);
                    hasher.update(&chunk[0]);
                }

                next_level.push(hasher.finalize().to_vec());
            }

            current_level = next_level;
        }

        Ok(current_level
            .into_iter()
            .next()
            .unwrap_or_else(|| vec![0; 32]))
    }
}

impl Default for DefaultTree {
    fn default() -> Self {
        Self::new()
    }
}

impl Tree for DefaultTree {
    fn push(&mut self, data: &[u8]) -> Result<()> {
        // Hash the data before adding as a leaf
        let mut hasher = Sha256::new();
        hasher.update(data);
        self.leaves.push(hasher.finalize().to_vec());
        self.root = None; // Invalidate cached root
        Ok(())
    }

    fn root(&mut self) -> Result<Vec<u8>> {
        if let Some(ref root) = self.root {
            return Ok(root.clone());
        }

        let root = self.compute_root()?;
        self.root = Some(root.clone());
        Ok(root)
    }
}
