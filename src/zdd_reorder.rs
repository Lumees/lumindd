// lumindd — ZDD-specific variable reordering
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use crate::manager::Manager;
use crate::node::DdNode;
use crate::unique_table::UniqueSubtable;

impl Manager {
    /// Manually trigger ZDD variable reordering with the given method.
    ///
    /// This mirrors [`Manager::reduce_heap`] but operates on the ZDD variable
    /// ordering (`zdd_perm`, `zdd_inv_perm`) and ZDD unique tables.
    pub fn zdd_reduce_heap(&mut self, method: crate::reorder::ReorderingMethod) {
        use crate::reorder::ReorderingMethod;

        match method {
            ReorderingMethod::None => {}
            ReorderingMethod::Sift | ReorderingMethod::SiftConverge => {
                let converge = method == ReorderingMethod::SiftConverge;
                self.zdd_sift_reorder(converge);
            }
            ReorderingMethod::Window2 => self.zdd_window_reorder(2),
            ReorderingMethod::Window3 => self.zdd_window_reorder(3),
            ReorderingMethod::Random => self.zdd_random_reorder(),
        }
        self.cache.clear();
        self.reordered = true;
    }

    /// Apply a specific permutation to the ZDD variables.
    ///
    /// `permutation[i]` is the new level for ZDD variable index `i`.
    /// The permutation must be a valid bijection on `0..num_zdd_vars`.
    pub fn zdd_shuffle_heap(&mut self, permutation: &[u32]) {
        assert_eq!(
            permutation.len(),
            self.num_zdd_vars as usize,
            "permutation length {} != num_zdd_vars {}",
            permutation.len(),
            self.num_zdd_vars
        );

        // Validate: must be a valid permutation
        let n = self.num_zdd_vars as usize;
        let mut seen = vec![false; n];
        for &p in permutation {
            assert!(
                (p as usize) < n,
                "invalid level {} in ZDD permutation (num_zdd_vars = {})",
                p,
                n
            );
            assert!(
                !seen[p as usize],
                "duplicate level {} in ZDD permutation",
                p
            );
            seen[p as usize] = true;
        }

        self.zdd_apply_permutation(permutation);
    }

    /// Sifting reordering for ZDD variables.
    ///
    /// Tries moving each ZDD variable to its best position by sifting it
    /// through all levels, measuring the total ZDD node count at each position.
    /// If `converge` is true, repeats until no further improvement is found.
    pub fn zdd_sift_reorder(&mut self, converge: bool) {
        let n = self.num_zdd_vars as usize;
        if n <= 1 {
            return;
        }

        loop {
            let initial_size = self.zdd_total_live_nodes();
            let mut improved = false;

            // Order variables by ZDD subtable size (largest first)
            let mut var_order: Vec<u16> = (0..self.num_zdd_vars).collect();
            var_order.sort_by(|&a, &b| {
                let sa = self.zdd_subtable_size(a);
                let sb = self.zdd_subtable_size(b);
                sb.cmp(&sa)
            });

            for &var in &var_order {
                let current_level = self.zdd_perm[var as usize];
                let best = self.zdd_sift_variable(var);
                if best != current_level {
                    improved = true;
                }
            }

            if !converge || !improved {
                break;
            }

            let new_size = self.zdd_total_live_nodes();
            if new_size >= initial_size {
                break;
            }
        }
    }

    /// Swap two adjacent ZDD levels and rebuild ZDD unique tables.
    ///
    /// This is the fundamental operation for ZDD sifting-based reordering.
    /// It swaps the variables at `level` and `level + 1` in the ZDD ordering.
    pub fn zdd_swap_adjacent_levels(&mut self, level: u32) {
        let n = self.num_zdd_vars as u32;
        if level + 1 >= n {
            return;
        }

        let var_at_level = self.zdd_inv_perm[level as usize];
        let var_at_next = self.zdd_inv_perm[(level + 1) as usize];

        // Update ZDD permutation
        self.zdd_perm[var_at_level as usize] = level + 1;
        self.zdd_perm[var_at_next as usize] = level;
        self.zdd_inv_perm[level as usize] = var_at_next;
        self.zdd_inv_perm[(level + 1) as usize] = var_at_level;

        // Rebuild ZDD unique tables to reflect the new ordering
        self.zdd_rebuild_unique_tables();
        self.cache.clear();
    }

    /// Rebuild all ZDD unique tables from the node arena.
    ///
    /// Scans all live internal nodes and re-inserts them into the correct
    /// ZDD level's unique table based on the current `zdd_perm` mapping.
    /// This is needed after any change to the ZDD variable ordering.
    pub fn zdd_rebuild_unique_tables(&mut self) {
        let num_levels = self.num_zdd_vars as usize;
        let mut new_tables: Vec<UniqueSubtable> = (0..num_levels)
            .map(|_| UniqueSubtable::new())
            .collect();

        // We need to identify which nodes belong to ZDD unique tables.
        // ZDD nodes are those whose var_index maps to a valid ZDD level.
        // We check if the var_index is within ZDD variable range and the node
        // was originally in a ZDD unique table.
        //
        // Heuristic: scan all internal nodes and insert those whose var_index
        // has a valid ZDD permutation entry and whose ref_count > 0.
        for idx in 0..self.nodes.len() {
            if let DdNode::Internal {
                var_index,
                then_child,
                else_child,
                ref_count,
                ..
            } = self.nodes[idx]
            {
                if ref_count > 0 && var_index < self.num_zdd_vars {
                    let level = self.zdd_perm[var_index as usize] as usize;
                    if level < new_tables.len() {
                        // Check if this looks like a ZDD node: in ZDD, the
                        // then-child should not be ZERO (ZDD reduction rule).
                        // If then_child is ZERO, it's likely a BDD node that
                        // happens to share the same var_index.
                        if !then_child.is_zero() {
                            new_tables[level].insert(then_child, else_child, idx as u32);
                        }
                    }
                }
            }
        }

        self.zdd_unique_tables = new_tables;
    }

    // ------------------------------------------------------------------
    // Internal helpers for ZDD reordering
    // ------------------------------------------------------------------

    /// Apply a permutation to ZDD variables and rebuild ZDD unique tables.
    fn zdd_apply_permutation(&mut self, new_perm: &[u32]) {
        for i in 0..self.num_zdd_vars as usize {
            self.zdd_perm[i] = new_perm[i];
        }
        for i in 0..self.num_zdd_vars as usize {
            self.zdd_inv_perm[self.zdd_perm[i] as usize] = i as u32;
        }

        self.zdd_rebuild_unique_tables();
        self.cache.clear();
    }

    /// Sift a single ZDD variable to its best level. Returns the best level.
    fn zdd_sift_variable(&mut self, var: u16) -> u32 {
        let n = self.num_zdd_vars as u32;
        let start_level = self.zdd_perm[var as usize];
        let mut best_level = start_level;
        let mut best_size = self.zdd_total_live_nodes();

        // Sift down
        let mut current_level = start_level;
        while current_level + 1 < n {
            self.zdd_swap_adjacent_levels(current_level);
            current_level += 1;
            let size = self.zdd_total_live_nodes();
            if size < best_size {
                best_size = size;
                best_level = current_level;
            }
        }

        // Sift back to start, then up
        while current_level > start_level {
            self.zdd_swap_adjacent_levels(current_level - 1);
            current_level -= 1;
        }

        // Sift up
        while current_level > 0 {
            self.zdd_swap_adjacent_levels(current_level - 1);
            current_level -= 1;
            let size = self.zdd_total_live_nodes();
            if size < best_size {
                best_size = size;
                best_level = current_level;
            }
        }

        // Move to best level
        while current_level < best_level {
            self.zdd_swap_adjacent_levels(current_level);
            current_level += 1;
        }
        while current_level > best_level {
            self.zdd_swap_adjacent_levels(current_level - 1);
            current_level -= 1;
        }

        best_level
    }

    /// Window reordering for ZDD with the given window size.
    fn zdd_window_reorder(&mut self, window_size: usize) {
        let n = self.num_zdd_vars as usize;
        if n <= window_size {
            return;
        }

        for start in 0..=(n - window_size) {
            self.zdd_optimize_window(start, window_size);
        }
    }

    /// Try all permutations within a ZDD window and pick the best.
    fn zdd_optimize_window(&mut self, start: usize, size: usize) {
        if size == 2 {
            let current_size = self.zdd_total_live_nodes();
            self.zdd_swap_adjacent_levels(start as u32);
            let swapped_size = self.zdd_total_live_nodes();
            if swapped_size >= current_size {
                self.zdd_swap_adjacent_levels(start as u32); // swap back
            }
        } else if size == 3 {
            let mut best_size = self.zdd_total_live_nodes();
            let mut best_perm_snapshot = self.zdd_perm.clone();

            let original_vars: Vec<u32> = (start..start + size)
                .map(|l| self.zdd_inv_perm[l])
                .collect();

            // Try all 6 permutations of 3 elements
            let perms: [[u32; 3]; 6] = [
                [0, 1, 2],
                [0, 2, 1],
                [1, 0, 2],
                [1, 2, 0],
                [2, 0, 1],
                [2, 1, 0],
            ];

            for perm in &perms {
                let mut new_full_perm = self.zdd_perm.clone();
                for (i, &p) in perm.iter().enumerate() {
                    let var = original_vars[p as usize];
                    new_full_perm[var as usize] = (start + i) as u32;
                }
                self.zdd_apply_permutation(&new_full_perm);

                let node_count = self.zdd_total_live_nodes();
                if node_count < best_size {
                    best_size = node_count;
                    best_perm_snapshot = self.zdd_perm.clone();
                }
            }

            self.zdd_apply_permutation(&best_perm_snapshot);
        }
    }

    /// Random ZDD reordering (for testing/benchmarking).
    fn zdd_random_reorder(&mut self) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let n = self.num_zdd_vars as usize;
        if n <= 1 {
            return;
        }

        let mut perm: Vec<u32> = (0..n as u32).collect();
        let mut hasher = DefaultHasher::new();
        self.nodes.len().hash(&mut hasher);
        self.num_zdd_vars.hash(&mut hasher);
        let mut seed = hasher.finish();

        for i in (1..n).rev() {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let j = (seed as usize) % (i + 1);
            perm.swap(i, j);
        }

        self.zdd_apply_permutation(&perm);
    }

    /// Count total live nodes in all ZDD unique tables.
    fn zdd_total_live_nodes(&self) -> usize {
        self.zdd_unique_tables.iter().map(|t| t.keys).sum()
    }

    /// Count nodes in a ZDD variable's subtable.
    fn zdd_subtable_size(&self, var: u16) -> usize {
        let level = self.zdd_perm[var as usize] as usize;
        if level < self.zdd_unique_tables.len() {
            self.zdd_unique_tables[level].len()
        } else {
            0
        }
    }
}
