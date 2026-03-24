// lumindd — Variable interaction matrix for reordering optimization
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use crate::manager::Manager;
use crate::node::DdNode;

/// Bit-packed symmetric interaction matrix.
///
/// Tracks which variable pairs co-occur on some root-to-terminal path
/// in the BDD. When two variables never interact, swapping them during
/// sifting cannot change the BDD size, so the swap can be skipped.
///
/// The matrix is stored as a flat bit vector in row-major lower-triangular
/// form: entry (i, j) with i > j is stored at bit index `i*(i-1)/2 + j`.
/// This type is public so that advanced reordering consumers can inspect
/// the matrix, but most users will interact with it only through `Manager` methods.
#[allow(dead_code)]
pub struct InteractionMatrix {
    /// Bit-packed storage for the lower-triangular matrix.
    bits: Vec<u64>,
    /// Number of variables (matrix dimension).
    n: usize,
}

impl InteractionMatrix {
    /// Create a zero-filled matrix for `n` variables.
    pub fn new(n: usize) -> Self {
        // Number of entries in the lower triangle (excluding diagonal).
        let num_entries = n * (n.saturating_sub(1)) / 2;
        let num_words = (num_entries + 63) / 64;
        InteractionMatrix {
            bits: vec![0u64; num_words],
            n,
        }
    }

    /// Compute the flat bit index for the pair (i, j), where i != j.
    #[inline]
    fn bit_index(mut i: usize, mut j: usize) -> usize {
        if i < j {
            std::mem::swap(&mut i, &mut j);
        }
        i * (i - 1) / 2 + j
    }

    /// Set the interaction bit for variables i and j.
    #[inline]
    pub fn set(&mut self, i: usize, j: usize) {
        if i == j {
            return;
        }
        let idx = Self::bit_index(i, j);
        let word = idx / 64;
        let bit = idx % 64;
        if word < self.bits.len() {
            self.bits[word] |= 1u64 << bit;
        }
    }

    /// Test whether variables i and j interact.
    #[inline]
    pub fn test(&self, i: usize, j: usize) -> bool {
        if i == j {
            return true; // a variable always "interacts" with itself
        }
        let idx = Self::bit_index(i, j);
        let word = idx / 64;
        let bit = idx % 64;
        if word < self.bits.len() {
            (self.bits[word] >> bit) & 1 != 0
        } else {
            false
        }
    }

    /// Number of variables this matrix covers.
    #[inline]
    pub fn size(&self) -> usize {
        self.n
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        for w in self.bits.iter_mut() {
            *w = 0;
        }
    }

    /// Count how many variables interact with variable `v`.
    pub fn interaction_count(&self, v: usize) -> usize {
        let mut count = 0;
        for u in 0..self.n {
            if u != v && self.test(u, v) {
                count += 1;
            }
        }
        count
    }
}

impl Manager {
    /// Build the variable interaction matrix by scanning all live nodes.
    ///
    /// Two variables x_i and x_j interact if there exists a path from a
    /// node labelled x_i to a node labelled x_j (or vice versa) that does
    /// not pass through any other variable node. In practice, we detect
    /// interactions by checking parent-child relationships: if a node with
    /// variable index `i` has a child (then or else) with variable index `j`,
    /// then `i` and `j` interact.
    ///
    /// Additionally, variables that appear in the support of the same root
    /// function interact. We approximate this by scanning every internal node
    /// and recording the direct parent-child variable pairs, plus transitive
    /// interactions along chains of skipped levels.
    pub fn build_interaction_matrix(&self) -> InteractionMatrix {
        let n = self.num_vars as usize;
        let mut matrix = InteractionMatrix::new(n);

        // For each live internal node, record interactions between its variable
        // and the variables of its immediate children. This captures direct
        // interactions. For BDDs with complemented edges, the variable index
        // is on the regular node.
        for idx in 0..self.nodes.len() {
            if let DdNode::Internal {
                var_index,
                then_child,
                else_child,
                ref_count,
                ..
            } = self.nodes[idx]
            {
                if ref_count == 0 {
                    continue;
                }

                let vi = var_index as usize;

                // Check then-child
                let t_raw = then_child.raw_index() as usize;
                if t_raw < self.nodes.len() {
                    if let DdNode::Internal {
                        var_index: t_var, ..
                    } = self.nodes[t_raw]
                    {
                        let vt = t_var as usize;
                        if vi < n && vt < n {
                            matrix.set(vi, vt);
                        }
                    }
                }

                // Check else-child
                let e_raw = else_child.raw_index() as usize;
                if e_raw < self.nodes.len() {
                    if let DdNode::Internal {
                        var_index: e_var, ..
                    } = self.nodes[e_raw]
                    {
                        let ve = e_var as usize;
                        if vi < n && ve < n {
                            matrix.set(vi, ve);
                        }
                    }
                }
            }
        }

        // Compute transitive closure so that variables connected through
        // intermediate nodes are also marked as interacting. We use a
        // simple BFS-style flooding: repeat until no new interactions are
        // discovered. For moderate variable counts (< 1000) this is fast.
        let mut changed = true;
        while changed {
            changed = false;
            for i in 0..n {
                for j in (i + 1)..n {
                    if matrix.test(i, j) {
                        // Propagate: if i interacts with j and j interacts with k,
                        // then i interacts with k.
                        for k in 0..n {
                            if k != i && k != j {
                                if matrix.test(j, k) && !matrix.test(i, k) {
                                    matrix.set(i, k);
                                    changed = true;
                                }
                                if matrix.test(i, k) && !matrix.test(j, k) {
                                    matrix.set(j, k);
                                    changed = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        matrix
    }

    /// Build a lightweight (direct-only) interaction matrix without
    /// transitive closure. Faster for use in sifting heuristics where
    /// only direct parent-child interactions matter for skipping swaps.
    pub fn build_direct_interaction_matrix(&self) -> InteractionMatrix {
        let n = self.num_vars as usize;
        let mut matrix = InteractionMatrix::new(n);

        for idx in 0..self.nodes.len() {
            if let DdNode::Internal {
                var_index,
                then_child,
                else_child,
                ref_count,
                ..
            } = self.nodes[idx]
            {
                if ref_count == 0 {
                    continue;
                }

                let vi = var_index as usize;

                let t_raw = then_child.raw_index() as usize;
                if t_raw < self.nodes.len() {
                    if let DdNode::Internal {
                        var_index: t_var, ..
                    } = self.nodes[t_raw]
                    {
                        let vt = t_var as usize;
                        if vi < n && vt < n {
                            matrix.set(vi, vt);
                        }
                    }
                }

                let e_raw = else_child.raw_index() as usize;
                if e_raw < self.nodes.len() {
                    if let DdNode::Internal {
                        var_index: e_var, ..
                    } = self.nodes[e_raw]
                    {
                        let ve = e_var as usize;
                        if vi < n && ve < n {
                            matrix.set(vi, ve);
                        }
                    }
                }
            }
        }

        matrix
    }

    /// Check if two variables interact (convenience wrapper that builds
    /// a direct interaction matrix on the fly). For repeated queries,
    /// prefer building the matrix once and reusing it.
    pub fn variables_interact(&self, var_a: u16, var_b: u16) -> bool {
        let matrix = self.build_direct_interaction_matrix();
        matrix.test(var_a as usize, var_b as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interaction_matrix_basic() {
        let mut m = InteractionMatrix::new(5);
        assert!(!m.test(0, 1));
        m.set(0, 1);
        assert!(m.test(0, 1));
        assert!(m.test(1, 0)); // symmetric
        assert!(!m.test(0, 2));
    }

    #[test]
    fn test_interaction_matrix_self() {
        let m = InteractionMatrix::new(3);
        assert!(m.test(0, 0)); // self always interacts
        assert!(m.test(2, 2));
    }

    #[test]
    fn test_interaction_count() {
        let mut m = InteractionMatrix::new(4);
        m.set(0, 1);
        m.set(0, 2);
        m.set(0, 3);
        assert_eq!(m.interaction_count(0), 3);
        assert_eq!(m.interaction_count(1), 1);
    }

    #[test]
    fn test_build_interaction_matrix() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let _z = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y);
        mgr.ref_node(f);
        let matrix = mgr.build_direct_interaction_matrix();
        // x and y should interact because AND creates a node with x
        // whose child is a node with y.
        assert!(matrix.test(0, 1));
    }
}
