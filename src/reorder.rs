// lumindd — Dynamic variable reordering
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use crate::manager::Manager;
use crate::node::DdNode;
use crate::unique_table::UniqueSubtable;

/// Available variable reordering methods.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReorderingMethod {
    /// No reordering.
    None,
    /// Rudell's sifting algorithm — moves each variable to its best position.
    Sift,
    /// Sifting with convergence — repeat until no improvement.
    SiftConverge,
    /// Window permutation of size 2.
    Window2,
    /// Window permutation of size 3.
    Window3,
    /// Random reordering (for benchmarking).
    Random,
}

impl Manager {
    /// Manually trigger variable reordering with the given method.
    pub fn reduce_heap(&mut self, method: ReorderingMethod) {
        match method {
            ReorderingMethod::None => {}
            ReorderingMethod::Sift | ReorderingMethod::SiftConverge => {
                let converge = method == ReorderingMethod::SiftConverge;
                self.sift_reorder(converge);
            }
            ReorderingMethod::Window2 => self.window_reorder(2),
            ReorderingMethod::Window3 => self.window_reorder(3),
            ReorderingMethod::Random => self.random_reorder(),
        }
        self.cache.clear();
        self.reordered = true;
    }

    /// Apply a specific permutation to the variables.
    ///
    /// `permutation[i]` is the new level for variable index `i`.
    pub fn shuffle_heap(&mut self, permutation: &[u32]) {
        assert_eq!(permutation.len(), self.num_vars as usize);

        // Validate: must be a valid permutation
        let mut seen = vec![false; self.num_vars as usize];
        for &p in permutation {
            assert!((p as usize) < self.num_vars as usize, "invalid level in permutation");
            assert!(!seen[p as usize], "duplicate level in permutation");
            seen[p as usize] = true;
        }

        self.apply_permutation(permutation);
    }

    /// Apply a permutation and rebuild all unique tables.
    pub(crate) fn apply_permutation(&mut self, new_perm: &[u32]) {
        for i in 0..self.num_vars as usize {
            self.perm[i] = new_perm[i];
        }
        for i in 0..self.num_vars as usize {
            self.inv_perm[self.perm[i] as usize] = i as u32;
        }

        self.rebuild_unique_tables();
        self.cache.clear();
    }

    /// Swap two adjacent levels and rebuild unique tables.
    ///
    /// This is the fundamental operation for sifting-based reordering.
    pub(crate) fn swap_adjacent_levels(&mut self, level: u32) {
        let n = self.num_vars as u32;
        if level + 1 >= n {
            return;
        }

        let var_at_level = self.inv_perm[level as usize];
        let var_at_next = self.inv_perm[(level + 1) as usize];

        // Update permutation
        self.perm[var_at_level as usize] = level + 1;
        self.perm[var_at_next as usize] = level;
        self.inv_perm[level as usize] = var_at_next;
        self.inv_perm[(level + 1) as usize] = var_at_level;

        // Rebuild unique tables to reflect the new ordering
        self.rebuild_unique_tables();
        self.cache.clear();
    }

    /// Sifting reordering: try moving each variable to its best position.
    fn sift_reorder(&mut self, converge: bool) {
        let n = self.num_vars as usize;
        if n <= 1 {
            return;
        }

        loop {
            let initial_size = self.total_live_nodes();
            let mut improved = false;

            // Order variables by subtable size (largest first)
            let mut var_order: Vec<u16> = (0..self.num_vars).collect();
            var_order.sort_by(|&a, &b| {
                let sa = self.subtable_size(a);
                let sb = self.subtable_size(b);
                sb.cmp(&sa)
            });

            for &var in &var_order {
                let current_level = self.perm[var as usize];
                let best = self.sift_variable(var);
                if best != current_level {
                    improved = true;
                }
            }

            if !converge || !improved {
                break;
            }

            let new_size = self.total_live_nodes();
            if new_size >= initial_size {
                break;
            }
        }
    }

    /// Sift a single variable to its best level.
    /// Returns the best level found.
    pub(crate) fn sift_variable(&mut self, var: u16) -> u32 {
        let n = self.num_vars as u32;
        let start_level = self.perm[var as usize];
        let mut best_level = start_level;
        let mut best_size = self.total_live_nodes();

        // Sift down
        let mut current_level = start_level;
        while current_level + 1 < n {
            self.swap_adjacent_levels(current_level);
            current_level += 1;
            let size = self.total_live_nodes();
            if size < best_size {
                best_size = size;
                best_level = current_level;
            }
        }

        // Sift back to start, then up
        while current_level > start_level {
            self.swap_adjacent_levels(current_level - 1);
            current_level -= 1;
        }

        // Sift up
        while current_level > 0 {
            self.swap_adjacent_levels(current_level - 1);
            current_level -= 1;
            let size = self.total_live_nodes();
            if size < best_size {
                best_size = size;
                best_level = current_level;
            }
        }

        // Move to best level
        while current_level < best_level {
            self.swap_adjacent_levels(current_level);
            current_level += 1;
        }
        while current_level > best_level {
            self.swap_adjacent_levels(current_level - 1);
            current_level -= 1;
        }

        best_level
    }

    /// Window reordering with the given window size.
    fn window_reorder(&mut self, window_size: usize) {
        let n = self.num_vars as usize;
        if n <= window_size {
            return;
        }

        for start in 0..=(n - window_size) {
            self.optimize_window(start, window_size);
        }
    }

    /// Try all permutations within a window and pick the best.
    fn optimize_window(&mut self, start: usize, size: usize) {
        if size == 2 {
            let current_size = self.total_live_nodes();
            self.swap_adjacent_levels(start as u32);
            let swapped_size = self.total_live_nodes();
            if swapped_size >= current_size {
                self.swap_adjacent_levels(start as u32); // swap back
            }
        } else if size == 3 {
            let mut best_size = self.total_live_nodes();
            let mut best_perm_snapshot = self.perm.clone();

            let original_vars: Vec<u32> = (start..start + size)
                .map(|l| self.inv_perm[l])
                .collect();

            // Try all 6 permutations of 3 elements
            let perms: [[u32; 3]; 6] = [
                [0, 1, 2], [0, 2, 1], [1, 0, 2],
                [1, 2, 0], [2, 0, 1], [2, 1, 0],
            ];

            for perm in &perms {
                // Build the full permutation array
                let mut new_full_perm = self.perm.clone();
                for (i, &p) in perm.iter().enumerate() {
                    let var = original_vars[p as usize];
                    new_full_perm[var as usize] = (start + i) as u32;
                }
                self.apply_permutation(&new_full_perm);

                let node_count = self.total_live_nodes();
                if node_count < best_size {
                    best_size = node_count;
                    best_perm_snapshot = self.perm.clone();
                }
            }

            // Apply best permutation
            self.apply_permutation(&best_perm_snapshot);
        }
    }

    /// Random reordering (for testing/benchmarking).
    fn random_reorder(&mut self) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let n = self.num_vars as usize;
        if n <= 1 {
            return;
        }

        // Fisher-Yates shuffle with a deterministic seed
        let mut perm: Vec<u32> = (0..n as u32).collect();
        let mut hasher = DefaultHasher::new();
        self.nodes.len().hash(&mut hasher);
        let mut seed = hasher.finish();

        for i in (1..n).rev() {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let j = (seed as usize) % (i + 1);
            perm.swap(i, j);
        }

        self.apply_permutation(&perm);
    }

    /// Count total live nodes in all unique tables.
    pub(crate) fn total_live_nodes(&self) -> usize {
        self.unique_tables.iter().map(|t| t.keys).sum()
    }

    /// Count nodes in a variable's subtable.
    pub(crate) fn subtable_size(&self, var: u16) -> usize {
        let level = self.perm[var as usize] as usize;
        if level < self.unique_tables.len() {
            self.unique_tables[level].len()
        } else {
            0
        }
    }

    /// Rebuild all unique tables from the node arena.
    ///
    /// Scans all live internal nodes and re-inserts them into the correct
    /// level's unique table based on the current `perm` mapping.
    pub(crate) fn rebuild_unique_tables(&mut self) {
        let num_levels = self.num_vars as usize;
        let mut new_tables: Vec<UniqueSubtable> = (0..num_levels)
            .map(|_| UniqueSubtable::new())
            .collect();

        for idx in 0..self.nodes.len() {
            if let DdNode::Internal {
                var_index,
                then_child,
                else_child,
                ref_count,
                ..
            } = self.nodes[idx]
            {
                if ref_count > 0 {
                    let level = self.perm[var_index as usize] as usize;
                    if level < new_tables.len() {
                        new_tables[level].insert(then_child, else_child, idx as u32);
                    }
                }
            }
        }

        self.unique_tables = new_tables;
    }
}
