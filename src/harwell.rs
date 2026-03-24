// lumindd — Harwell-Boeing sparse matrix import for ADDs
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

//! Harwell-Boeing sparse matrix I/O and conversion to/from ADDs.
//!
//! Implements a simplified Harwell-Boeing (compressed column storage) format
//! reader/writer, plus conversion routines that build an ADD from a sparse
//! matrix and extract a sparse matrix from an ADD.

use std::io::{self, BufRead, Write};

use crate::manager::Manager;
use crate::node::NodeId;

/// A sparse matrix in compressed column storage (CCS) format,
/// compatible with the Harwell-Boeing exchange format.
#[derive(Debug, Clone)]
pub struct HarwellMatrix {
    /// Number of rows.
    pub nrows: usize,
    /// Number of columns.
    pub ncols: usize,
    /// Column pointers: `col_ptr[j]` is the index into `row_idx`/`values`
    /// where column j starts. Length = ncols + 1.
    pub col_ptr: Vec<usize>,
    /// Row indices of non-zero entries.
    pub row_idx: Vec<usize>,
    /// Values of non-zero entries.
    pub values: Vec<f64>,
}

impl HarwellMatrix {
    /// Create an empty matrix.
    pub fn new(nrows: usize, ncols: usize) -> Self {
        HarwellMatrix {
            nrows,
            ncols,
            col_ptr: vec![0; ncols + 1],
            row_idx: Vec::new(),
            values: Vec::new(),
        }
    }

    /// Number of non-zero entries.
    pub fn nnz(&self) -> usize {
        self.values.len()
    }

    /// Parse a matrix from a simplified Harwell-Boeing text format.
    ///
    /// Expected format:
    /// ```text
    /// <title line>
    /// <nrows> <ncols> <nnz>
    /// <col_ptr values, space-separated, ncols+1 values>
    /// <row_idx values, space-separated, nnz values>
    /// <values, space-separated, nnz values>
    /// ```
    ///
    /// Row indices and column pointers are 1-based in the file (Fortran convention)
    /// and are converted to 0-based internally.
    pub fn from_reader<R: BufRead>(r: &mut R) -> io::Result<Self> {
        // Line 1: title (skip)
        let mut line = String::new();
        r.read_line(&mut line)?;

        // Line 2: nrows ncols nnz
        line.clear();
        r.read_line(&mut line)?;
        let dims: Vec<usize> = line
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if dims.len() < 3 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "expected nrows, ncols, nnz on second line",
            ));
        }
        let nrows = dims[0];
        let ncols = dims[1];
        let nnz = dims[2];

        // Read column pointers (may span multiple lines)
        let col_ptr = read_values_usize(r, ncols + 1)?;
        // Convert from 1-based to 0-based
        let col_ptr: Vec<usize> = col_ptr.iter().map(|&v| v.saturating_sub(1)).collect();

        // Read row indices
        let row_idx = read_values_usize(r, nnz)?;
        let row_idx: Vec<usize> = row_idx.iter().map(|&v| v.saturating_sub(1)).collect();

        // Read values
        let values = read_values_f64(r, nnz)?;

        Ok(HarwellMatrix {
            nrows,
            ncols,
            col_ptr,
            row_idx,
            values,
        })
    }

    /// Write the matrix in simplified Harwell-Boeing text format.
    pub fn to_writer<W: Write>(&self, w: &mut W) -> io::Result<()> {
        writeln!(w, "lumindd sparse matrix")?;
        writeln!(w, "{} {} {}", self.nrows, self.ncols, self.nnz())?;

        // Column pointers (1-based)
        for (i, &cp) in self.col_ptr.iter().enumerate() {
            if i > 0 {
                write!(w, " ")?;
            }
            write!(w, "{}", cp + 1)?;
        }
        writeln!(w)?;

        // Row indices (1-based)
        for (i, &ri) in self.row_idx.iter().enumerate() {
            if i > 0 {
                write!(w, " ")?;
            }
            write!(w, "{}", ri + 1)?;
        }
        writeln!(w)?;

        // Values
        for (i, &v) in self.values.iter().enumerate() {
            if i > 0 {
                write!(w, " ")?;
            }
            write!(w, "{}", v)?;
        }
        writeln!(w)?;

        Ok(())
    }

    /// Get value at (row, col), returning 0.0 if not present.
    pub fn get(&self, row: usize, col: usize) -> f64 {
        if col >= self.ncols {
            return 0.0;
        }
        let start = self.col_ptr[col];
        let end = self.col_ptr[col + 1];
        for k in start..end {
            if self.row_idx[k] == row {
                return self.values[k];
            }
        }
        0.0
    }
}

/// Read `count` usize values from the reader (possibly across multiple lines).
fn read_values_usize<R: BufRead>(r: &mut R, count: usize) -> io::Result<Vec<usize>> {
    let mut result = Vec::with_capacity(count);
    while result.len() < count {
        let mut line = String::new();
        let n = r.read_line(&mut line)?;
        if n == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("expected {} values, got {}", count, result.len()),
            ));
        }
        for token in line.split_whitespace() {
            if result.len() >= count {
                break;
            }
            let v: usize = token.parse().map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("parse error: {}", e))
            })?;
            result.push(v);
        }
    }
    Ok(result)
}

/// Read `count` f64 values from the reader (possibly across multiple lines).
fn read_values_f64<R: BufRead>(r: &mut R, count: usize) -> io::Result<Vec<f64>> {
    let mut result = Vec::with_capacity(count);
    while result.len() < count {
        let mut line = String::new();
        let n = r.read_line(&mut line)?;
        if n == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("expected {} values, got {}", count, result.len()),
            ));
        }
        for token in line.split_whitespace() {
            if result.len() >= count {
                break;
            }
            let v: f64 = token.parse().map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("parse error: {}", e))
            })?;
            result.push(v);
        }
    }
    Ok(result)
}

impl Manager {
    /// Convert a sparse matrix to an ADD.
    ///
    /// Each non-zero entry (i, j, v) becomes a minterm path in the ADD with
    /// value v. The binary encoding of i uses `row_vars` (MSB first) and
    /// the binary encoding of j uses `col_vars` (MSB first).
    ///
    /// All other entries have value 0.0 in the resulting ADD.
    pub fn add_from_sparse_matrix(
        &mut self,
        matrix: &HarwellMatrix,
        row_vars: &[u16],
        col_vars: &[u16],
    ) -> NodeId {
        // Ensure variables exist
        for &v in row_vars.iter().chain(col_vars.iter()) {
            while self.num_vars <= v {
                self.bdd_new_var();
            }
        }

        let zero = self.add_zero();
        let mut result = zero;

        // For each non-zero entry, build a minterm ADD and accumulate with plus
        for col in 0..matrix.ncols {
            let start = matrix.col_ptr[col];
            let end = matrix.col_ptr[col + 1];
            for k in start..end {
                let row = matrix.row_idx[k];
                let val = matrix.values[k];
                if val == 0.0 {
                    continue;
                }

                let minterm = self.add_minterm(row, col, val, row_vars, col_vars);
                result = self.add_plus(result, minterm);
            }
        }

        result
    }

    /// Build a minterm ADD: value `val` at (row, col), 0 elsewhere.
    fn add_minterm(
        &mut self,
        row: usize,
        col: usize,
        val: f64,
        row_vars: &[u16],
        col_vars: &[u16],
    ) -> NodeId {
        let val_node = self.add_const(val);
        let zero = self.add_zero();
        let nr = row_vars.len();
        let nc = col_vars.len();

        // Start from the terminal value and build upward through variables
        let mut current = val_node;

        // Process col_vars from LSB to MSB (reverse order for bottom-up)
        for i in (0..nc).rev() {
            let bit = (col >> (nc - 1 - i)) & 1;
            current = if bit == 1 {
                self.add_unique_inter(col_vars[i], current, zero)
            } else {
                self.add_unique_inter(col_vars[i], zero, current)
            };
        }

        // Process row_vars from LSB to MSB
        for i in (0..nr).rev() {
            let bit = (row >> (nr - 1 - i)) & 1;
            current = if bit == 1 {
                self.add_unique_inter(row_vars[i], current, zero)
            } else {
                self.add_unique_inter(row_vars[i], zero, current)
            };
        }

        current
    }

    /// Extract a sparse matrix from an ADD by collecting all non-zero paths.
    ///
    /// `row_vars` and `col_vars` specify which ADD variables encode the row
    /// and column indices respectively (MSB first).
    pub fn add_to_sparse_matrix(
        &self,
        f: NodeId,
        row_vars: &[u16],
        col_vars: &[u16],
    ) -> HarwellMatrix {
        let nrows = 1usize << row_vars.len();
        let ncols = 1usize << col_vars.len();

        // Collect all (row, col, value) triples
        let mut entries: Vec<(usize, usize, f64)> = Vec::new();
        let mut assignment: Vec<(u16, bool)> = Vec::new();
        self.collect_add_paths(f, &mut assignment, row_vars, col_vars, &mut entries);

        // Sort by column then row for CCS format
        entries.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));

        // Build CCS
        let mut col_ptr = vec![0usize; ncols + 1];
        let mut row_idx = Vec::with_capacity(entries.len());
        let mut values = Vec::with_capacity(entries.len());

        for &(r, c, v) in &entries {
            col_ptr[c + 1] += 1;
            row_idx.push(r);
            values.push(v);
        }

        // Accumulate column pointers
        for j in 0..ncols {
            col_ptr[j + 1] += col_ptr[j];
        }

        HarwellMatrix {
            nrows,
            ncols,
            col_ptr,
            row_idx,
            values,
        }
    }

    /// Recursively collect all paths in an ADD with non-zero terminal values.
    fn collect_add_paths(
        &self,
        node: NodeId,
        assignment: &mut Vec<(u16, bool)>,
        row_vars: &[u16],
        col_vars: &[u16],
        entries: &mut Vec<(usize, usize, f64)>,
    ) {
        if let Some(val) = self.add_value(node) {
            if val != 0.0 {
                // Decode row and column from the assignment
                let row = decode_index(assignment, row_vars);
                let col = decode_index(assignment, col_vars);
                entries.push((row, col, val));
            }
            return;
        }

        let var = self.var_index(node);
        let n = self.node(node);
        let then_child = n.then_child();
        let else_child = n.else_child();

        // Explore else branch (var = false)
        assignment.push((var, false));
        self.collect_add_paths(else_child, assignment, row_vars, col_vars, entries);
        assignment.pop();

        // Explore then branch (var = true)
        assignment.push((var, true));
        self.collect_add_paths(then_child, assignment, row_vars, col_vars, entries);
        assignment.pop();
    }
}

/// Decode an integer index from the current variable assignment.
/// `vars` lists variable indices MSB first.
fn decode_index(assignment: &[(u16, bool)], vars: &[u16]) -> usize {
    let n = vars.len();
    let mut index = 0usize;
    for (i, &v) in vars.iter().enumerate() {
        let bit = assignment.iter().find(|(vi, _)| *vi == v).is_some_and(|(_, b)| *b);
        if bit {
            index |= 1 << (n - 1 - i);
        }
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    #[test]
    fn harwell_roundtrip() {
        // Build a small 2x2 matrix: [[1.0, 0.0], [2.0, 3.0]]
        let mat = HarwellMatrix {
            nrows: 2,
            ncols: 2,
            col_ptr: vec![0, 1, 2],
            row_idx: vec![0, 1],
            values: vec![1.0, 3.0],
        };

        // Write
        let mut buf = Vec::new();
        mat.to_writer(&mut buf).unwrap();

        // Read back
        let mut reader = BufReader::new(&buf[..]);
        let mat2 = HarwellMatrix::from_reader(&mut reader).unwrap();

        assert_eq!(mat2.nrows, 2);
        assert_eq!(mat2.ncols, 2);
        assert_eq!(mat2.col_ptr, vec![0, 1, 2]);
        assert_eq!(mat2.row_idx, vec![0, 1]);
        assert_eq!(mat2.values, vec![1.0, 3.0]);
    }

    #[test]
    fn harwell_get() {
        // 3x3 identity matrix
        let mat = HarwellMatrix {
            nrows: 3,
            ncols: 3,
            col_ptr: vec![0, 1, 2, 3],
            row_idx: vec![0, 1, 2],
            values: vec![1.0, 1.0, 1.0],
        };
        assert_eq!(mat.get(0, 0), 1.0);
        assert_eq!(mat.get(1, 1), 1.0);
        assert_eq!(mat.get(2, 2), 1.0);
        assert_eq!(mat.get(0, 1), 0.0);
        assert_eq!(mat.get(1, 0), 0.0);
    }

    #[test]
    fn sparse_to_add_and_back() {
        let mut mgr = Manager::new();

        // 2x2 matrix: [[5.0, 0.0], [0.0, 7.0]]
        let mat = HarwellMatrix {
            nrows: 2,
            ncols: 2,
            col_ptr: vec![0, 1, 2],
            row_idx: vec![0, 1],
            values: vec![5.0, 7.0],
        };

        let row_vars = &[0u16];
        let col_vars = &[1u16];

        let add = mgr.add_from_sparse_matrix(&mat, row_vars, col_vars);

        // Extract back
        let mat2 = mgr.add_to_sparse_matrix(add, row_vars, col_vars);

        assert_eq!(mat2.nrows, 2);
        assert_eq!(mat2.ncols, 2);
        assert_eq!(mat2.nnz(), 2);
        assert_eq!(mat2.get(0, 0), 5.0);
        assert_eq!(mat2.get(1, 1), 7.0);
        assert_eq!(mat2.get(0, 1), 0.0);
        assert_eq!(mat2.get(1, 0), 0.0);
    }

    #[test]
    fn sparse_to_add_dense() {
        let mut mgr = Manager::new();

        // 2x2 dense matrix: [[1.0, 2.0], [3.0, 4.0]]
        let mat = HarwellMatrix {
            nrows: 2,
            ncols: 2,
            col_ptr: vec![0, 2, 4],
            row_idx: vec![0, 1, 0, 1],
            values: vec![1.0, 3.0, 2.0, 4.0],
        };

        let row_vars = &[0u16];
        let col_vars = &[1u16];

        let add = mgr.add_from_sparse_matrix(&mat, row_vars, col_vars);
        let mat2 = mgr.add_to_sparse_matrix(add, row_vars, col_vars);

        assert_eq!(mat2.get(0, 0), 1.0);
        assert_eq!(mat2.get(0, 1), 2.0);
        assert_eq!(mat2.get(1, 0), 3.0);
        assert_eq!(mat2.get(1, 1), 4.0);
    }

    #[test]
    fn empty_matrix() {
        let mut mgr = Manager::new();
        let mat = HarwellMatrix::new(4, 4);
        let row_vars = &[0u16, 1];
        let col_vars = &[2u16, 3];
        let add = mgr.add_from_sparse_matrix(&mat, row_vars, col_vars);
        let mat2 = mgr.add_to_sparse_matrix(add, row_vars, col_vars);
        assert_eq!(mat2.nnz(), 0);
    }
}
