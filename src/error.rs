use thiserror::Error;

/// Main error type for Extended Data Square operations
#[derive(Error, Debug)]
pub enum EdsError {
    #[error("Invalid data square dimensions: {0}")]
    InvalidDimensions(String),
    #[error("Codec error: {0}")]
    Codec(#[from] CodecError),
    #[error("Repair error: {0}")]
    Repair(#[from] RepairError),
    #[error("Invalid data: {0}")]
    InvalidData(String),
    #[error("Index out of bounds: row={row}, col={col}, width={width}")]
    IndexOutOfBounds { row: usize, col: usize, width: usize },
}

/// Error type specific to codec operations
#[derive(Error, Debug)]
pub enum CodecError {
    #[error("Reed-Solomon encoding failed: {0}")]
    EncodingFailed(String),
    #[error("Reed-Solomon decoding failed: {0}")]
    DecodingFailed(String),
    #[error("Invalid chunk size: expected {expected}, got {actual}")]
    InvalidChunkSize { expected: usize, actual: usize },
    #[error("Insufficient data for decoding")]
    InsufficientData,
}

/// Error type specific to data repair operations
#[derive(Error, Debug)]
pub enum RepairError {
    #[error("Too many corrupted chunks to repair: {corrupted} > {max_repairable}")]
    TooManyCorrupted { corrupted: usize, max_repairable: usize },
    #[error("Root hash mismatch: expected {expected:?}, got {actual:?}")]
    RootHashMismatch { expected: Vec<u8>, actual: Vec<u8> },
    #[error("Cannot repair: missing required data")]
    MissingRequiredData,
    #[error("Repair validation failed: {0}")]
    ValidationFailed(String),
}

/// Convenience result types
pub type EdsResult<T> = Result<T, EdsError>;
pub type CodecResult<T> = Result<T, CodecError>;
pub type RepairResult<T> = Result<T, RepairError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_formatting() {
        let codec_error = CodecError::EncodingFailed("test error".to_string());
        assert!(codec_error.to_string().contains("Reed-Solomon encoding failed"));

        let eds_error = EdsError::Codec(codec_error);
        assert!(eds_error.to_string().contains("Codec error"));
    }

    #[test]
    fn test_error_chain_preservation() {
        use std::error::Error;
        
        let root_cause = CodecError::InsufficientData;
        let eds_error = EdsError::Codec(root_cause);
        
        // Test error source chain
        assert!(eds_error.source().is_some());
    }

    #[test]
    fn test_index_out_of_bounds_error() {
        let error = EdsError::IndexOutOfBounds {
            row: 5,
            col: 10,
            width: 8,
        };
        let message = error.to_string();
        assert!(message.contains("row=5"));
        assert!(message.contains("col=10"));
        assert!(message.contains("width=8"));
    }
}