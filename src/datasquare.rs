use crate::{Error, Result, TreeConstructorFn, Axis};

/// DataSquare stores all data for an original data square (ODS) or extended
/// data square (EDS). Data is duplicated in both row-major and column-major
/// order in order to be able to provide zero-allocation column slices.
#[derive(Clone)]
pub struct DataSquare {
    square_row: Vec<Vec<Option<Vec<u8>>>>, // row-major
    square_col: Vec<Vec<Option<Vec<u8>>>>, // col-major
    width: usize,
    share_size: usize,
    row_roots: Option<Vec<Vec<u8>>>,
    col_roots: Option<Vec<Vec<u8>>>,
    create_tree_fn: TreeConstructorFn,
}

impl DataSquare {
    /// Create a new DataSquare from the supplied data and tree creator.
    /// No root calculation is performed. Data may have None values.
    pub fn new(
        data: Vec<Option<Vec<u8>>>,
        tree_creator: TreeConstructorFn,
        share_size: usize,
    ) -> Result<Self> {
        let width = (data.len() as f64).sqrt().ceil() as usize;
        if width * width != data.len() {
            return Err(Error::NotSquareNumber);
        }

        // Validate share sizes
        for d in &data {
            if let Some(share) = d {
                if share.len() != share_size {
                    return Err(Error::UnevenChunks);
                }
            }
        }

        // Create row-major layout
        let mut square_row = Vec::with_capacity(width);
        for row_idx in 0..width {
            let start_idx = row_idx * width;
            let end_idx = start_idx + width;
            square_row.push(data[start_idx..end_idx].to_vec());
        }

        // Create column-major layout
        let mut square_col = Vec::with_capacity(width);
        for col_idx in 0..width {
            let mut col = Vec::with_capacity(width);
            for row_idx in 0..width {
                col.push(data[row_idx * width + col_idx].clone());
            }
            square_col.push(col);
        }

        Ok(DataSquare {
            square_row,
            square_col,
            width,
            share_size,
            row_roots: None,
            col_roots: None,
            create_tree_fn: tree_creator,
        })
    }

    /// Extend the square by extending width and fill the extended quadrants with filler_share.
    pub fn extend_square(&mut self, extended_width: usize, filler_share: Vec<u8>) -> Result<()> {
        if filler_share.len() != self.share_size {
            return Err(Error::FillerChunkSizeMismatch);
        }

        let new_width = self.width + extended_width;
        let mut new_square_row = Vec::with_capacity(new_width);

        // Create filler row for extensions
        let filler_extended_row: Vec<Option<Vec<u8>>> = 
            (0..extended_width).map(|_| Some(filler_share.clone())).collect();

        let filler_row: Vec<Option<Vec<u8>>> = 
            (0..new_width).map(|_| Some(filler_share.clone())).collect();

        // Extend existing rows
        for i in 0..self.width {
            let mut row = self.square_row[i].clone();
            row.extend(filler_extended_row.clone());
            new_square_row.push(row);
        }

        // Add new rows filled with filler
        for _ in self.width..new_width {
            new_square_row.push(filler_row.clone());
        }

        self.square_row = new_square_row;

        // Rebuild column-major layout
        let mut new_square_col = Vec::with_capacity(new_width);
        for col_idx in 0..new_width {
            let mut col = Vec::with_capacity(new_width);
            for row_idx in 0..new_width {
                col.push(self.square_row[row_idx][col_idx].clone());
            }
            new_square_col.push(col);
        }
        self.square_col = new_square_col;
        self.width = new_width;

        self.reset_roots();
        Ok(())
    }

    /// Get a row slice.
    pub fn row_slice(&self, row_idx: usize, from_idx: usize, length: usize) -> Vec<Option<Vec<u8>>> {
        let end_idx = from_idx + length;
        self.square_row[row_idx][from_idx..end_idx].to_vec()
    }

    /// Get a full row.
    pub fn row(&self, row_idx: usize) -> Vec<Option<Vec<u8>>> {
        self.row_slice(row_idx, 0, self.width)
    }

    /// Set a row slice.
    pub fn set_row_slice(
        &mut self,
        row_idx: usize,
        from_idx: usize,
        new_row: Vec<Vec<u8>>,
    ) -> Result<()> {
        for share in &new_row {
            if share.len() != self.share_size {
                return Err(Error::InvalidChunkSize("invalid chunk size".to_string()));
            }
        }

        if from_idx + new_row.len() > self.width {
            return Err(Error::RowSliceOutOfBounds {
                row: row_idx,
                from: from_idx,
                length: new_row.len(),
                width: self.width,
            });
        }

        for (i, share) in new_row.into_iter().enumerate() {
            let col_idx = from_idx + i;
            self.square_row[row_idx][col_idx] = Some(share.clone());
            self.square_col[col_idx][row_idx] = Some(share);
        }

        self.reset_roots();
        Ok(())
    }

    /// Get a column slice.
    pub fn col_slice(&self, row_idx: usize, col_idx: usize, length: usize) -> Vec<Option<Vec<u8>>> {
        let end_idx = row_idx + length;
        self.square_col[col_idx][row_idx..end_idx].to_vec()
    }

    /// Get a full column.
    pub fn col(&self, col_idx: usize) -> Vec<Option<Vec<u8>>> {
        self.col_slice(0, col_idx, self.width)
    }

    /// Set a column slice.
    pub fn set_col_slice(
        &mut self,
        col_idx: usize,
        from_idx: usize,
        new_col: Vec<Vec<u8>>,
    ) -> Result<()> {
        for share in &new_col {
            if share.len() != self.share_size {
                return Err(Error::InvalidChunkSize("invalid chunk size".to_string()));
            }
        }

        if from_idx + new_col.len() > self.width {
            return Err(Error::ColSliceOutOfBounds {
                from: from_idx,
                col: col_idx,
                length: new_col.len(),
                width: self.width,
            });
        }

        for (i, share) in new_col.into_iter().enumerate() {
            let row_idx = from_idx + i;
            self.square_row[row_idx][col_idx] = Some(share.clone());
            self.square_col[col_idx][row_idx] = Some(share);
        }

        self.reset_roots();
        Ok(())
    }

    /// Reset cached roots.
    fn reset_roots(&mut self) {
        self.row_roots = None;
        self.col_roots = None;
    }

    /// Get a copy of a specific cell.
    pub fn get_cell(&self, row_idx: usize, col_idx: usize) -> Option<Vec<u8>> {
        self.square_row[row_idx][col_idx].clone()
    }

    /// Set a specific cell. The cell to set must be None.
    pub fn set_cell(&mut self, row_idx: usize, col_idx: usize, new_share: Vec<u8>) -> Result<()> {
        if self.square_row[row_idx][col_idx].is_some() {
            return Err(Error::CellAlreadySet { row: row_idx, col: col_idx });
        }
        if new_share.len() != self.share_size {
            return Err(Error::CellChunkSizeMismatch {
                chunk_size: new_share.len(),
                expected_size: self.share_size,
            });
        }

        self.square_row[row_idx][col_idx] = Some(new_share.clone());
        self.square_col[col_idx][row_idx] = Some(new_share);
        self.reset_roots();
        Ok(())
    }

    /// Get the flattened data square.
    pub fn flattened(&self) -> Vec<Option<Vec<u8>>> {
        let mut flattened = Vec::with_capacity(self.width * self.width);
        for row in &self.square_row {
            flattened.extend(row.iter().cloned());
        }
        flattened
    }

    /// Get the width of the data square.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Get the share size.
    pub fn share_size(&self) -> usize {
        self.share_size
    }

    /// Compute the row root for a given row index.
    pub fn get_row_root(&self, row_idx: usize) -> Result<Vec<u8>> {
        let mut tree = (self.create_tree_fn)(Axis::Row, row_idx);
        let row = self.row(row_idx);
        
        if !is_complete(&row) {
            return Err(Error::IncompleteAxis { axis: Axis::Row });
        }

        for share_opt in row {
            if let Some(share) = share_opt {
                tree.push(&share)?;
            }
        }

        tree.root()
    }

    /// Compute the column root for a given column index.
    pub fn get_col_root(&self, col_idx: usize) -> Result<Vec<u8>> {
        let mut tree = (self.create_tree_fn)(Axis::Col, col_idx);
        let col = self.col(col_idx);
        
        if !is_complete(&col) {
            return Err(Error::IncompleteAxis { axis: Axis::Col });
        }

        for share_opt in col {
            if let Some(share) = share_opt {
                tree.push(&share)?;
            }
        }

        tree.root()
    }

    /// Get row roots for all rows.
    pub fn get_row_roots(&mut self) -> Result<Vec<Vec<u8>>> {
        if let Some(ref roots) = self.row_roots {
            return Ok(roots.clone());
        }

        let mut roots = Vec::with_capacity(self.width);
        for i in 0..self.width {
            roots.push(self.get_row_root(i)?);
        }

        self.row_roots = Some(roots.clone());
        Ok(roots)
    }

    /// Get column roots for all columns.
    pub fn get_col_roots(&mut self) -> Result<Vec<Vec<u8>>> {
        if let Some(ref roots) = self.col_roots {
            return Ok(roots.clone());
        }

        let mut roots = Vec::with_capacity(self.width);
        for i in 0..self.width {
            roots.push(self.get_col_root(i)?);
        }

        self.col_roots = Some(roots.clone());
        Ok(roots)
    }
}

/// Check if all shares in a slice are present (not None).
fn is_complete(shares: &[Option<Vec<u8>>]) -> bool {
    shares.iter().all(|share| share.is_some())
}