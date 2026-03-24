// lumindd — Group sifting reordering algorithm
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

//! Group sifting respects variable group constraints (e.g., from an MTR
//! tree). Variables within the same group are kept together and sifted
//! as a block. This is essential when encoding multi-valued variables
//! or maintaining user-specified variable groupings.

use crate::manager::Manager;

/// A variable group defined by a contiguous level range.
/// `start` is the first level in the group, `size` is the number of
/// levels in the group.
#[derive(Clone, Debug)]
pub struct VarGroup {
    /// First level of this group.
    pub start: usize,
    /// Number of consecutive levels in this group.
    pub size: usize,
}

impl Manager {
    // ------------------------------------------------------------------
    // Group management
    // ------------------------------------------------------------------

    /// Define variable groups as a list of `(start_level, size)` pairs.
    /// Each group must be a contiguous range of levels. Groups must not
    /// overlap. Variables not covered by any group form singleton groups.
    ///
    /// This information is stored externally and passed into the group
    /// sifting methods.
    pub fn make_var_groups(
        &self,
        group_specs: &[(usize, usize)],
    ) -> Vec<VarGroup> {
        let n = self.num_vars as usize;
        let mut covered = vec![false; n];
        let mut groups = Vec::new();

        for &(start, size) in group_specs {
            assert!(
                start + size <= n,
                "group ({}, {}) exceeds number of variables {}",
                start,
                size,
                n
            );
            for l in start..start + size {
                assert!(!covered[l], "overlapping groups at level {}", l);
                covered[l] = true;
            }
            groups.push(VarGroup { start, size });
        }

        // Create singleton groups for uncovered levels.
        for l in 0..n {
            if !covered[l] {
                groups.push(VarGroup { start: l, size: 1 });
            }
        }

        // Sort groups by start level.
        groups.sort_by_key(|g| g.start);
        groups
    }

    // ------------------------------------------------------------------
    // Group sifting
    // ------------------------------------------------------------------

    /// Group sifting: sift variable groups while keeping group members
    /// adjacent. Each group is treated as an atomic block.
    ///
    /// `groups` should be produced by `make_var_groups`. If `None`, every
    /// variable is its own singleton group and this degenerates to
    /// regular sifting.
    pub fn group_sift(&mut self, groups: &[VarGroup], converge: bool) {
        let n = self.num_vars as u32;
        if n <= 1 {
            return;
        }

        loop {
            let initial_size = self.total_live_nodes();
            let mut improved = false;

            // Build a fresh group layout based on current level mapping.
            // Re-derive which variables are in each group.
            let group_info = self.resolve_groups(groups);

            // Sort groups by total subtable size (largest first).
            let mut order: Vec<usize> = (0..group_info.len()).collect();
            order.sort_by(|&a, &b| {
                let sa: usize = group_info[a]
                    .1
                    .iter()
                    .map(|&v| self.subtable_size(v))
                    .sum();
                let sb: usize = group_info[b]
                    .1
                    .iter()
                    .map(|&v| self.subtable_size(v))
                    .sum();
                sb.cmp(&sa)
            });

            for &gi in &order {
                let (_, ref vars) = group_info[gi];
                if vars.len() == 1 {
                    let old_level = self.perm[vars[0] as usize];
                    let new_level = self.sift_variable(vars[0]);
                    if new_level != old_level {
                        improved = true;
                    }
                } else {
                    let before = self.total_live_nodes();
                    self.sift_group_block(vars);
                    let after = self.total_live_nodes();
                    if after < before {
                        improved = true;
                    }
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

        self.cache.clear();
        self.reordered = true;
    }

    /// Group sifting with convergence — repeats until no improvement.
    pub fn group_sift_converge(&mut self, groups: &[VarGroup]) {
        self.group_sift(groups, true);
    }

    /// Resolve group definitions to actual variable indices based on the
    /// current ordering. Returns `(start_level, vars)` for each group.
    fn resolve_groups(&self, groups: &[VarGroup]) -> Vec<(u32, Vec<u16>)> {
        let n = self.num_vars as usize;
        let mut result = Vec::new();

        for g in groups {
            let start = g.start.min(n.saturating_sub(1));
            let end = (g.start + g.size).min(n);
            let mut vars: Vec<u16> = (start..end)
                .map(|l| self.inv_perm[l] as u16)
                .collect();

            // Sort variables by their current level.
            vars.sort_by_key(|&v| self.perm[v as usize]);

            let first_level = if vars.is_empty() {
                0
            } else {
                self.perm[vars[0] as usize]
            };
            result.push((first_level, vars));
        }

        result
    }

    /// Sift a block of variables (group) to its best position.
    ///
    /// The variables in `vars` are first moved to be adjacent, then the
    /// entire block is sifted up and down.
    fn sift_group_block(&mut self, vars: &[u16]) {
        let n = self.num_vars as u32;
        let block_size = vars.len() as u32;
        if block_size == 0 || n <= 1 {
            return;
        }

        // Step 1: Consolidate — ensure all vars are at adjacent levels.
        let mut levels: Vec<u32> = vars.iter().map(|&v| self.perm[v as usize]).collect();
        levels.sort();

        // Move vars together starting at the minimum level.
        let target_start = levels[0];
        for (offset, &var) in vars.iter().enumerate() {
            let target = target_start + offset as u32;
            let mut current = self.perm[var as usize];
            while current > target {
                self.swap_adjacent_levels(current - 1);
                current -= 1;
            }
            while current < target {
                self.swap_adjacent_levels(current);
                current += 1;
            }
        }

        // Step 2: Sift the consolidated block.
        let mut current_start = target_start;
        let mut best_start = current_start;
        let mut best_size = self.total_live_nodes();

        // Sift down.
        while current_start + block_size < n {
            self.move_block_down(current_start, block_size);
            current_start += 1;

            let size = self.total_live_nodes();
            if size < best_size {
                best_size = size;
                best_start = current_start;
            }
        }

        // Return to original position.
        while current_start > target_start {
            self.move_block_up(current_start, block_size);
            current_start -= 1;
        }

        // Sift up.
        while current_start > 0 {
            self.move_block_up(current_start, block_size);
            current_start -= 1;

            let size = self.total_live_nodes();
            if size < best_size {
                best_size = size;
                best_start = current_start;
            }
        }

        // Move to best position.
        while current_start < best_start {
            self.move_block_down(current_start, block_size);
            current_start += 1;
        }
        while current_start > best_start {
            self.move_block_up(current_start, block_size);
            current_start -= 1;
        }
    }

    /// Move a contiguous block of `size` levels starting at `start` down
    /// by one position. This swaps the level just below the block with
    /// each level in the block from bottom to top.
    fn move_block_down(&mut self, start: u32, size: u32) {
        let bottom = start + size - 1;
        let n = self.num_vars as u32;
        if bottom + 1 >= n {
            return;
        }
        // Bubble the element at bottom+1 up through the block.
        for i in (0..size).rev() {
            self.swap_adjacent_levels(start + i);
        }
    }

    /// Move a contiguous block of `size` levels starting at `start` up
    /// by one position. This swaps the level just above the block with
    /// each level in the block from top to bottom.
    fn move_block_up(&mut self, start: u32, size: u32) {
        if start == 0 {
            return;
        }
        // Bubble the element at start-1 down through the block.
        for i in 0..size {
            if start + i > 0 {
                self.swap_adjacent_levels(start - 1 + i);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Manager;

    #[test]
    fn test_make_var_groups() {
        let mut mgr = Manager::new();
        for _ in 0..6 {
            mgr.bdd_new_var();
        }
        let groups = mgr.make_var_groups(&[(0, 3), (3, 3)]);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].start, 0);
        assert_eq!(groups[0].size, 3);
        assert_eq!(groups[1].start, 3);
        assert_eq!(groups[1].size, 3);
    }

    #[test]
    fn test_group_sift_singletons() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var();
        let x1 = mgr.bdd_new_var();
        let x2 = mgr.bdd_new_var();
        let f = mgr.bdd_and(x0, x1);
        let _g = mgr.bdd_or(f, x2);

        let groups = mgr.make_var_groups(&[]);
        mgr.group_sift(&groups, false);
    }

    #[test]
    fn test_group_sift_block() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var();
        let x1 = mgr.bdd_new_var();
        let x2 = mgr.bdd_new_var();
        let x3 = mgr.bdd_new_var();
        let a = mgr.bdd_and(x0, x1);
        let b = mgr.bdd_and(x2, x3);
        let _f = mgr.bdd_or(a, b);

        let groups = mgr.make_var_groups(&[(0, 2), (2, 2)]);
        mgr.group_sift(&groups, false);
    }

    #[test]
    fn test_group_sift_converge() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var();
        let x1 = mgr.bdd_new_var();
        let x2 = mgr.bdd_new_var();
        let x3 = mgr.bdd_new_var();
        let a = mgr.bdd_and(x0, x1);
        let b = mgr.bdd_and(x2, x3);
        let _f = mgr.bdd_or(a, b);

        let groups = mgr.make_var_groups(&[(0, 2), (2, 2)]);
        mgr.group_sift_converge(&groups);
    }
}
