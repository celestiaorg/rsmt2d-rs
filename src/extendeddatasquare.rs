use crate::{Axis, ByzantineDataError, Codec, DataSquare, Error, Result, TreeConstructorFn};
use std::sync::Arc;

/// ExtendedDataSquare represents an extended piece of data.
#[derive(Clone)]
pub struct ExtendedDataSquare {
    data_square: DataSquare,
    codec: Arc<dyn Codec>,
    original_data_width: usize,
}

impl ExtendedDataSquare {
    /// Create a new ExtendedDataSquare with specified parameters.
    pub fn new(
        codec: Arc<dyn Codec>,
        tree_creator_fn: TreeConstructorFn,
        eds_width: usize,
        share_size: usize,
    ) -> Result<Self> {
        validate_eds_width(eds_width)?;
        codec.validate_chunk_size(share_size)?;

        let data = vec![None; eds_width * eds_width];
        let data_square = DataSquare::new(data, tree_creator_fn, share_size)?;

        let original_data_width = eds_width / 2;
        Ok(ExtendedDataSquare {
            data_square,
            codec,
            original_data_width,
        })
    }

    /// Perform erasure extension of the square.
    fn erasure_extend_square(&mut self) -> Result<()> {
        self.original_data_width = self.data_square.width();

        // Extend original square with filler shares
        let filler_share = vec![0u8; self.data_square.share_size()];
        self.data_square
            .extend_square(self.data_square.width(), filler_share)?;

        // Populate filler shares in Q1 and Q2
        for i in 0..self.original_data_width {
            self.erasure_extend_row(i)?;
            self.erasure_extend_col(i)?;
        }

        // Populate filler shares in Q3
        for i in self.original_data_width..self.data_square.width() {
            self.erasure_extend_row(i)?;
        }

        Ok(())
    }

    /// Extend a row with erasure data.
    fn erasure_extend_row(&mut self, row_idx: usize) -> Result<()> {
        let row_slice = self
            .data_square
            .row_slice(row_idx, 0, self.original_data_width);

        // Convert Option<Vec<u8>> to Vec<u8> for encoding
        let data: Vec<Vec<u8>> = row_slice
            .into_iter()
            .map(|opt| opt.expect("Original data should be complete for encoding"))
            .collect();

        let parity_shares = self.codec.encode(&data)?;
        self.data_square
            .set_row_slice(row_idx, self.original_data_width, parity_shares)?;
        Ok(())
    }

    /// Extend a column with erasure data.
    fn erasure_extend_col(&mut self, col_idx: usize) -> Result<()> {
        let col_slice = self
            .data_square
            .col_slice(0, col_idx, self.original_data_width);

        // Convert Option<Vec<u8>> to Vec<u8> for encoding
        let data: Vec<Vec<u8>> = col_slice
            .into_iter()
            .map(|opt| opt.expect("Original data should be complete for encoding"))
            .collect();

        let parity_shares = self.codec.encode(&data)?;
        self.data_square
            .set_col_slice(col_idx, self.original_data_width, parity_shares)?;
        Ok(())
    }

    /// Get a copy of a column.
    pub fn col(&self, col_idx: usize) -> Vec<Option<Vec<u8>>> {
        deep_copy_optional(&self.data_square.col(col_idx))
    }

    /// Get column roots.
    pub fn col_roots(&mut self) -> Result<Vec<Vec<u8>>> {
        let col_roots = self.data_square.get_col_roots()?;
        Ok(deep_copy(&col_roots))
    }

    /// Get a copy of a row.
    pub fn row(&self, row_idx: usize) -> Vec<Option<Vec<u8>>> {
        deep_copy_optional(&self.data_square.row(row_idx))
    }

    /// Get row roots.
    pub fn row_roots(&mut self) -> Result<Vec<Vec<u8>>> {
        let row_roots = self.data_square.get_row_roots()?;
        Ok(deep_copy(&row_roots))
    }

    /// Get the width of the square.
    pub fn width(&self) -> usize {
        self.data_square.width()
    }

    /// Get the flattened extended data square.
    pub fn flattened(&self) -> Vec<Option<Vec<u8>>> {
        deep_copy_optional(&self.data_square.flattened())
    }

    /// Get the flattened original data square.
    pub fn flattened_ods(&self) -> Vec<Option<Vec<u8>>> {
        let mut flattened = Vec::with_capacity(self.original_data_width * self.original_data_width);
        for row_idx in 0..self.original_data_width {
            let row = self.row(row_idx);
            for item in row.iter().take(self.original_data_width) {
                flattened.push(item.clone());
            }
        }
        flattened
    }

    /// Check if this EDS equals another.
    pub fn equals(&mut self, other: &mut ExtendedDataSquare) -> Result<bool> {
        if self.original_data_width != other.original_data_width
            || self.codec.name() != other.codec.name()
            || self.data_square.share_size() != other.data_square.share_size()
            || self.data_square.width() != other.data_square.width()
        {
            return Ok(false);
        }

        for row_idx in 0..self.width() {
            let self_row = self.row(row_idx);
            let other_row = other.row(row_idx);

            for col_idx in 0..self_row.len() {
                match (&self_row[col_idx], &other_row[col_idx]) {
                    (Some(a), Some(b)) => {
                        if a != b {
                            return Ok(false);
                        }
                    }
                    (None, None) => {}
                    _ => return Ok(false),
                }
            }
        }

        Ok(true)
    }

    /// Get combined row and column roots.
    pub fn roots(&mut self) -> Result<Vec<Vec<u8>>> {
        let row_roots = self.row_roots()?;
        let col_roots = self.col_roots()?;

        let mut roots = Vec::with_capacity(row_roots.len() + col_roots.len());
        roots.extend(row_roots);
        roots.extend(col_roots);
        Ok(roots)
    }

    /// Repair attempts to repair an incomplete extended data square.
    pub fn repair(&mut self, row_roots: &[Vec<u8>], col_roots: &[Vec<u8>]) -> Result<()> {
        self.pre_repair_sanity_check(row_roots, col_roots)?;
        self.solve_crossword(row_roots, col_roots)
    }

    /// Perform sanity checks before repair.
    fn pre_repair_sanity_check(
        &mut self,
        row_roots: &[Vec<u8>],
        col_roots: &[Vec<u8>],
    ) -> Result<()> {
        if row_roots.len() != self.width() || col_roots.len() != self.width() {
            return Err(Error::UnrepairableDataSquare);
        }

        // Check if any complete rows/columns have mismatched roots
        for i in 0..self.width() {
            let row = self.row(i);
            if is_complete_optional(&row) {
                let computed_root = self.data_square.get_row_root(i)?;
                if computed_root != row_roots[i] {
                    return Err(Error::ReedSolomonError(format!(
                        "Pre-repair sanity check failed: row {} has incorrect root",
                        i
                    )));
                }
            }

            let col = self.col(i);
            if is_complete_optional(&col) {
                let computed_root = self.data_square.get_col_root(i)?;
                if computed_root != col_roots[i] {
                    return Err(Error::ReedSolomonError(format!(
                        "Pre-repair sanity check failed: column {} has incorrect root",
                        i
                    )));
                }
            }
        }

        Ok(())
    }

    /// Solve the crossword puzzle to repair the EDS.
    fn solve_crossword(&mut self, row_roots: &[Vec<u8>], col_roots: &[Vec<u8>]) -> Result<()> {
        // Keep repeating until the square is solved
        loop {
            let mut solved = true;
            let mut progress_made = false;

            // Try to solve each row and column
            for i in 0..self.width() {
                let (solved_row, progress_row) =
                    self.solve_crossword_row(i, row_roots, col_roots)?;
                let (solved_col, progress_col) =
                    self.solve_crossword_col(i, row_roots, col_roots)?;

                if !solved_row || !solved_col {
                    solved = false;
                }
                if progress_row || progress_col {
                    progress_made = true;
                }
            }

            if solved {
                return Ok(());
            }
            if !progress_made {
                return Err(Error::UnrepairableDataSquare);
            }
        }
    }

    /// Attempt to solve a specific row.
    fn solve_crossword_row(
        &mut self,
        row_idx: usize,
        row_roots: &[Vec<u8>],
        _col_roots: &[Vec<u8>],
    ) -> Result<(bool, bool)> {
        let row = self.row(row_idx);

        if is_complete_optional(&row) {
            return Ok((true, false));
        }

        // Try to reconstruct using Reed-Solomon
        if self.can_reconstruct(&row) {
            let reconstructed = self.codec.decode(&row)?;

            // Verify the reconstruction
            let mut tree = crate::new_default_tree(Axis::Row, row_idx);
            for share in &reconstructed {
                tree.push(share)?;
            }
            let computed_root = tree.root()?;

            if computed_root == row_roots[row_idx] {
                // Update the row
                for (col_idx, share) in reconstructed.into_iter().enumerate() {
                    if row[col_idx].is_none() {
                        self.data_square.set_cell(row_idx, col_idx, share)?;
                    }
                }
                return Ok((true, true));
            } else {
                // Byzantine data detected
                let shares = row
                    .into_iter()
                    .enumerate()
                    .filter(|(col_idx, _)| self.data_square.get_cell(row_idx, *col_idx).is_some())
                    .map(|(_, share)| share)
                    .collect();

                return Err(Error::ReedSolomonError(format!(
                    "Byzantine data in row {}: {:?}",
                    row_idx,
                    ByzantineDataError {
                        axis: Axis::Row,
                        index: row_idx,
                        shares,
                    }
                )));
            }
        }

        Ok((false, false))
    }

    /// Attempt to solve a specific column.
    fn solve_crossword_col(
        &mut self,
        col_idx: usize,
        _row_roots: &[Vec<u8>],
        col_roots: &[Vec<u8>],
    ) -> Result<(bool, bool)> {
        let col = self.col(col_idx);

        if is_complete_optional(&col) {
            return Ok((true, false));
        }

        // Try to reconstruct using Reed-Solomon
        if self.can_reconstruct(&col) {
            let reconstructed = self.codec.decode(&col)?;

            // Verify the reconstruction
            let mut tree = crate::new_default_tree(Axis::Col, col_idx);
            for share in &reconstructed {
                tree.push(share)?;
            }
            let computed_root = tree.root()?;

            if computed_root == col_roots[col_idx] {
                // Update the column
                for (row_idx, share) in reconstructed.into_iter().enumerate() {
                    if col[row_idx].is_none() {
                        self.data_square.set_cell(row_idx, col_idx, share)?;
                    }
                }
                return Ok((true, true));
            } else {
                // Byzantine data detected
                let shares = col
                    .into_iter()
                    .enumerate()
                    .filter(|(row_idx, _)| self.data_square.get_cell(*row_idx, col_idx).is_some())
                    .map(|(_, share)| share)
                    .collect();

                return Err(Error::ReedSolomonError(format!(
                    "Byzantine data in column {}: {:?}",
                    col_idx,
                    ByzantineDataError {
                        axis: Axis::Col,
                        index: col_idx,
                        shares,
                    }
                )));
            }
        }

        Ok((false, false))
    }

    /// Check if we have enough data to attempt reconstruction.
    fn can_reconstruct(&self, data: &[Option<Vec<u8>>]) -> bool {
        let present_count = data.iter().filter(|x| x.is_some()).count();
        let total = data.len();
        present_count >= total / 2 // Need at least half the data for Reed-Solomon
    }
}

/// Compute an extended data square from original data.
pub fn compute_extended_data_square(
    data: Vec<Vec<u8>>,
    codec: Arc<dyn Codec>,
    tree_creator_fn: TreeConstructorFn,
) -> Result<ExtendedDataSquare> {
    if data.len() > codec.max_chunks() {
        return Err(Error::TooManyChunks);
    }

    let share_size = get_share_size(&data);
    codec.validate_chunk_size(share_size)?;

    // Convert to Option<Vec<u8>> format
    let data_with_options: Vec<Option<Vec<u8>>> = data.into_iter().map(Some).collect();

    let data_square = DataSquare::new(data_with_options, tree_creator_fn, share_size)?;
    let mut eds = ExtendedDataSquare {
        data_square,
        codec,
        original_data_width: 0, // Will be set in erasure_extend_square
    };

    eds.erasure_extend_square()?;
    Ok(eds)
}

/// Import an extended data square from flattened data.
pub fn import_extended_data_square(
    data: Vec<Option<Vec<u8>>>,
    codec: Arc<dyn Codec>,
    tree_creator_fn: TreeConstructorFn,
) -> Result<ExtendedDataSquare> {
    if data.len() > 4 * codec.max_chunks() {
        return Err(Error::TooManyChunks);
    }

    let share_size = get_share_size_optional(&data);
    codec.validate_chunk_size(share_size)?;

    let data_square = DataSquare::new(data, tree_creator_fn, share_size)?;
    let eds_width = data_square.width();
    validate_eds_width(eds_width)?;

    let original_data_width = eds_width / 2;
    Ok(ExtendedDataSquare {
        data_square,
        codec,
        original_data_width,
    })
}

/// Validate that the EDS width is valid (must be even).
fn validate_eds_width(eds_width: usize) -> Result<()> {
    if eds_width % 2 != 0 {
        return Err(Error::InvalidEdsWidth { width: eds_width });
    }
    Ok(())
}

/// Get the share size from the first non-None share.
fn get_share_size(data: &[Vec<u8>]) -> usize {
    data.first().map(|d| d.len()).unwrap_or(0)
}

/// Get the share size from the first non-None share in optional data.
fn get_share_size_optional(data: &[Option<Vec<u8>>]) -> usize {
    data.iter()
        .flatten()
        .next()
        .map(|share| share.len())
        .unwrap_or(0)
}

/// Deep copy a vector of byte vectors.
fn deep_copy(original: &[Vec<u8>]) -> Vec<Vec<u8>> {
    original.to_vec()
}

/// Deep copy a vector of optional byte vectors.
fn deep_copy_optional(original: &[Option<Vec<u8>>]) -> Vec<Option<Vec<u8>>> {
    original.to_vec()
}

/// Check if all shares in an optional slice are present.
fn is_complete_optional(shares: &[Option<Vec<u8>>]) -> bool {
    shares.iter().all(|share| share.is_some())
}
