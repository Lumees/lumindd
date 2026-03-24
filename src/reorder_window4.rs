// lumindd — Window4 and convergence-enhanced window reordering
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

//! Window permutation reordering with size 4, plus convergence wrappers
//! for window2, window3, window4, and sift.

use crate::manager::Manager;

/// Generate all 24 permutations of [0, 1, 2, 3].
///
/// Uses Heap's algorithm unrolled into a static table.
fn all_permutations_4() -> [[u32; 4]; 24] {
    let mut result = [[0u32; 4]; 24];
    let mut count = 0;
    let base = [0u32, 1, 2, 3];
    // Generate via nested loops (lexicographic)
    for a in 0..4u32 {
        for b in 0..4u32 {
            if b == a {
                continue;
            }
            for c in 0..4u32 {
                if c == a || c == b {
                    continue;
                }
                let d = 6 - a - b - c; // 0+1+2+3 = 6
                result[count] = [base[a as usize], base[b as usize], base[c as usize], base[d as usize]];
                count += 1;
            }
        }
    }
    debug_assert_eq!(count, 24);
    result
}

impl Manager {
    /// Try all 24 permutations of the 4 variables at levels `start..start+4`
    /// and keep the one that minimizes total live nodes.
    fn optimize_window4(&mut self, start: usize) {
        let n = self.num_vars as usize;
        if start + 4 > n {
            return;
        }

        // Snapshot the current best state
        let mut best_size = self.total_live_nodes();
        let mut best_perm_snapshot = self.perm.clone();

        // Record which variables currently occupy levels start..start+3
        let original_vars: [u32; 4] = [
            self.inv_perm[start],
            self.inv_perm[start + 1],
            self.inv_perm[start + 2],
            self.inv_perm[start + 3],
        ];

        let perms = all_permutations_4();

        for perm in &perms {
            // Build a full permutation array with only the 4-window positions changed.
            // We start from the initial snapshot and override just the window slots,
            // testing all 24 orderings of the same 4 variables into the same 4 levels.
            let mut candidate_perm = best_perm_snapshot.clone();
            for (i, &p) in perm.iter().enumerate() {
                let var = original_vars[p as usize];
                candidate_perm[var as usize] = (start + i) as u32;
            }

            self.apply_permutation(&candidate_perm);

            let node_count = self.total_live_nodes();
            if node_count < best_size {
                best_size = node_count;
                best_perm_snapshot = self.perm.clone();
            }
        }

        // Restore the best permutation found
        self.apply_permutation(&best_perm_snapshot);
    }

    /// Window permutation reordering with window size 4.
    ///
    /// Slides a window of 4 adjacent levels across the entire variable order,
    /// trying all 24 permutations at each position and keeping the best.
    pub fn window4_reorder(&mut self) {
        let n = self.num_vars as usize;
        if n < 4 {
            // Fall back to window3 or window2 if not enough variables
            if n >= 2 {
                self.reduce_heap(crate::reorder::ReorderingMethod::Window3);
            }
            return;
        }

        for start in 0..=(n - 4) {
            self.optimize_window4(start);
        }

        self.cache.clear();
        self.reordered = true;
    }

    /// Window4 reordering with convergence.
    ///
    /// Repeats full window4 passes until no further improvement is found
    /// (the total live node count does not decrease).
    pub fn window4_reorder_converge(&mut self) {
        loop {
            let before = self.total_live_nodes();
            self.window4_reorder();
            let after = self.total_live_nodes();
            if after >= before {
                break;
            }
        }
    }

    /// Window2 reordering with convergence.
    ///
    /// Repeats window2 passes until no further improvement.
    pub fn window2_converge(&mut self) {
        loop {
            let before = self.total_live_nodes();
            self.reduce_heap(crate::reorder::ReorderingMethod::Window2);
            let after = self.total_live_nodes();
            if after >= before {
                break;
            }
        }
    }

    /// Window3 reordering with convergence.
    ///
    /// Repeats window3 passes until no further improvement.
    pub fn window3_converge(&mut self) {
        loop {
            let before = self.total_live_nodes();
            self.reduce_heap(crate::reorder::ReorderingMethod::Window3);
            let after = self.total_live_nodes();
            if after >= before {
                break;
            }
        }
    }

    /// Sift reordering with convergence (public convenience wrapper).
    ///
    /// Equivalent to `reduce_heap(SiftConverge)` but callable directly.
    pub fn sift_converge(&mut self) {
        self.reduce_heap(crate::reorder::ReorderingMethod::SiftConverge);
    }
}

// ======================================================================
// Tests
// ======================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Manager;

    #[test]
    fn test_all_permutations_4_count() {
        let perms = all_permutations_4();
        assert_eq!(perms.len(), 24);
        // Each permutation must contain exactly {0,1,2,3}
        for p in &perms {
            let mut sorted = *p;
            sorted.sort();
            assert_eq!(sorted, [0, 1, 2, 3]);
        }
        // All permutations must be distinct
        let mut set: std::collections::HashSet<[u32; 4]> = std::collections::HashSet::new();
        for p in &perms {
            assert!(set.insert(*p));
        }
    }

    #[test]
    fn test_window4_reorder_basic() {
        let mut mgr = Manager::new();
        let vars: Vec<_> = (0..6).map(|_| mgr.bdd_new_var()).collect();

        // Build a function that benefits from specific orderings
        // f = (x0 & x1) | (x2 & x3) | (x4 & x5)
        let a = mgr.bdd_and(vars[0], vars[1]);
        let b = mgr.bdd_and(vars[2], vars[3]);
        let c = mgr.bdd_and(vars[4], vars[5]);
        let ab = mgr.bdd_or(a, b);
        let f = mgr.bdd_or(ab, c);
        mgr.ref_node(f);

        let before = mgr.total_live_nodes();
        mgr.window4_reorder();
        let after = mgr.total_live_nodes();

        // Should not increase node count (may stay same or decrease)
        assert!(after <= before, "window4 should not increase nodes: before={}, after={}", before, after);
    }

    #[test]
    fn test_window4_reorder_converge() {
        let mut mgr = Manager::new();
        let vars: Vec<_> = (0..5).map(|_| mgr.bdd_new_var()).collect();

        let a = mgr.bdd_and(vars[0], vars[4]);
        let b = mgr.bdd_and(vars[1], vars[3]);
        let f = mgr.bdd_or(a, b);
        mgr.ref_node(f);

        let before = mgr.total_live_nodes();
        mgr.window4_reorder_converge();
        let after = mgr.total_live_nodes();

        assert!(after <= before);
    }

    #[test]
    fn test_window2_converge() {
        let mut mgr = Manager::new();
        let vars: Vec<_> = (0..4).map(|_| mgr.bdd_new_var()).collect();

        let a = mgr.bdd_and(vars[0], vars[3]);
        let b = mgr.bdd_and(vars[1], vars[2]);
        let f = mgr.bdd_or(a, b);
        mgr.ref_node(f);

        let before = mgr.total_live_nodes();
        mgr.window2_converge();
        let after = mgr.total_live_nodes();

        assert!(after <= before);
    }

    #[test]
    fn test_window3_converge() {
        let mut mgr = Manager::new();
        let vars: Vec<_> = (0..5).map(|_| mgr.bdd_new_var()).collect();

        let a = mgr.bdd_and(vars[0], vars[4]);
        let b = mgr.bdd_and(vars[1], vars[3]);
        let f = mgr.bdd_or(a, b);
        mgr.ref_node(f);

        let before = mgr.total_live_nodes();
        mgr.window3_converge();
        let after = mgr.total_live_nodes();

        assert!(after <= before);
    }

    #[test]
    fn test_sift_converge() {
        let mut mgr = Manager::new();
        let vars: Vec<_> = (0..4).map(|_| mgr.bdd_new_var()).collect();

        let a = mgr.bdd_and(vars[0], vars[3]);
        let f = mgr.bdd_or(a, vars[1]);
        mgr.ref_node(f);

        // Should not panic
        mgr.sift_converge();
    }

    #[test]
    fn test_window4_small_var_count() {
        // Fewer than 4 variables — should not panic
        let mut mgr = Manager::new();
        let v0 = mgr.bdd_new_var();
        let v1 = mgr.bdd_new_var();
        let f = mgr.bdd_and(v0, v1);
        mgr.ref_node(f);

        mgr.window4_reorder(); // should gracefully handle < 4 vars
    }
}
