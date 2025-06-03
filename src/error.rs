use thiserror::Error;
use crate::Axis;

/// Result type alias for rsmt2d operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in rsmt2d operations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum Error {
    #[error("non-nil shares not all of equal size")]
    UnevenChunks,

    #[error("number of chunks must be a square number")]
    NotSquareNumber,

    #[error("number of chunks exceeds the maximum")]
    TooManyChunks,

    #[error("filler chunk size does not match data square chunk size")]
    FillerChunkSizeMismatch,

    #[error("invalid chunk size: {0}")]
    InvalidChunkSize(String),

    #[error("cannot set row slice at ({row}, {from}) of length {length}: because it would exceed the data square width {width}")]
    RowSliceOutOfBounds { row: usize, from: usize, length: usize, width: usize },

    #[error("cannot set col slice at ({from}, {col}) of length {length}: because it would exceed the data square width {width}")]
    ColSliceOutOfBounds { from: usize, col: usize, length: usize, width: usize },

    #[error("cannot set cell ({row}, {col}) as it already has a value")]
    CellAlreadySet { row: usize, col: usize },

    #[error("cannot set cell with chunk size {chunk_size} because dataSquare chunk size is {expected_size}")]
    CellChunkSizeMismatch { chunk_size: usize, expected_size: usize },

    #[error("cannot compute root of incomplete {axis}")]
    IncompleteAxis { axis: Axis },

    #[error("extended data square width {width} must be even")]
    InvalidEdsWidth { width: usize },

    #[error("shareSize {share_size} must be a multiple of 64 bytes")]
    InvalidShareSize { share_size: usize },

    #[error("failed to solve data square")]
    UnrepairableDataSquare,

    #[error("Reed-Solomon codec error: {0}")]
    ReedSolomonError(String),

    #[error("Merkle tree error: {0}")]
    MerkleTreeError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Error returned when a repaired row or column does not match the expected
/// row or column Merkle root. It is also returned when the parity data from
/// a row or a column is not equal to the encoded original data.
#[derive(Error, Debug, Clone, PartialEq)]
#[error("byzantine {axis}: {index}")]
pub struct ByzantineDataError {
    /// Axis describes if this error is for a row or column.
    pub axis: Axis,
    /// Index is the row or column index.
    pub index: usize,
    /// Shares contain the shares in the row or column that the client can
    /// determine proofs for. Missing shares are None.
    pub shares: Vec<Option<Vec<u8>>>,
}

impl From<reed_solomon_erasure::Error> for Error {
    fn from(err: reed_solomon_erasure::Error) -> Self {
        Error::ReedSolomonError(err.to_string())
    }
}