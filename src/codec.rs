use reed_solomon_erasure::{ReedSolomon, galois_8::Field as Galois8Field};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::{Error, Result};

/// Codec trait for Reed-Solomon encoding and decoding.
pub trait Codec: Send + Sync {
    /// Encode encodes original data, automatically extracting share size.
    /// There must be no missing shares. Only returns parity shares.
    fn encode(&self, data: &[Vec<u8>]) -> Result<Vec<Vec<u8>>>;

    /// Decode decodes sparse original + parity data, automatically extracting share size.
    /// Missing shares must be None. Returns original + parity data.
    fn decode(&self, data: &[Option<Vec<u8>>]) -> Result<Vec<Vec<u8>>>;

    /// Returns the max number of chunks this codec supports in a 2D original data square.
    fn max_chunks(&self) -> usize;

    /// Returns the name of the codec.
    fn name(&self) -> &'static str;

    /// Returns an error if this codec does not support chunk_size.
    fn validate_chunk_size(&self, chunk_size: usize) -> Result<()>;
}

/// Leopard Reed-Solomon codec implementation.
pub struct LeoRSCodec {
    /// Cache the encoders of various sizes to not have to re-instantiate those
    /// as it is costly.
    enc_cache: Arc<RwLock<HashMap<usize, ReedSolomon<Galois8Field>>>>,
}

impl LeoRSCodec {
    /// Create a new LeoRSCodec instance.
    pub fn new() -> Self {
        Self {
            enc_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load or initialize a Reed-Solomon encoder for the given data length.
    fn load_or_init_encoder(&self, data_len: usize) -> Result<ReedSolomon<Galois8Field>> {
        // Try to read from cache first
        {
            let cache = self.enc_cache.read().map_err(|e| {
                Error::ReedSolomonError(format!("Failed to read encoder cache: {}", e))
            })?;
            
            if let Some(encoder) = cache.get(&data_len) {
                return Ok(encoder.clone());
            }
        }

        // Create new encoder
        let encoder = ReedSolomon::new(data_len, data_len)?;

        // Cache the encoder
        {
            let mut cache = self.enc_cache.write().map_err(|e| {
                Error::ReedSolomonError(format!("Failed to write encoder cache: {}", e))
            })?;
            cache.insert(data_len, encoder.clone());
        }

        Ok(encoder)
    }
}

impl Default for LeoRSCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Codec for LeoRSCodec {
    fn encode(&self, data: &[Vec<u8>]) -> Result<Vec<Vec<u8>>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let data_len = data.len();
        let share_size = data[0].len();
        let encoder = self.load_or_init_encoder(data_len)?;

        // Prepare shares: original data + space for parity
        let mut shares: Vec<Vec<u8>> = Vec::with_capacity(data_len * 2);
        
        // Add original data
        for chunk in data {
            shares.push(chunk.clone());
        }
        
        // Add empty parity slots
        for _ in 0..data_len {
            shares.push(vec![0; share_size]);
        }

        // Encode
        encoder.encode(&mut shares)?;

        // Return only the parity shares
        Ok(shares[data_len..].to_vec())
    }

    fn decode(&self, data: &[Option<Vec<u8>>]) -> Result<Vec<Vec<u8>>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let half = data.len() / 2;
        let encoder = self.load_or_init_encoder(half)?;

        // Convert Option<Vec<u8>> to Option<&mut [u8]> format expected by the library
        let mut shares: Vec<Option<Vec<u8>>> = data.to_vec();
        encoder.reconstruct(&mut shares)?;

        Ok(shares
            .into_iter()
            .map(|s| s.expect("All shares should be present after reconstruction"))
            .collect())
    }

    fn max_chunks(&self) -> usize {
        // klauspost/reedsolomon supports an EDS width of 65536.
        // An EDS width of 65536 is an ODS width of 32768.
        // The max number of shares in a 2D original data square is 32768 * 32768.
        crate::MAX_CHUNKS_LEOPARD
    }

    fn name(&self) -> &'static str {
        "Leopard"
    }

    fn validate_chunk_size(&self, share_size: usize) -> Result<()> {
        // Leopard codec requires share size to be a multiple of 64 bytes
        if share_size % 64 != 0 {
            return Err(Error::InvalidShareSize { share_size });
        }
        Ok(())
    }
}