// lumindd — Multi-way Tree (MTR) for variable grouping
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

//! Multi-way Tree (MTR) for variable grouping constraints.
//!
//! The MTR tree mirrors the structure used in CUDD to declare groups of
//! variables that must stay together during dynamic reordering.  Users can
//! mark a contiguous range of variable levels as a *group*; groups may nest
//! to form a hierarchy.  During sifting, a variable can only move within its
//! innermost group, and a group can only move as a unit within its parent.
//!
//! # Example
//!
//! ```rust,ignore
//! use lumindd::{Manager, mtr::{GroupFlags, MtrTree}};
//!
//! let mut mgr = Manager::new();
//! // create 8 variables …
//! for _ in 0..8 { mgr.bdd_new_var(); }
//!
//! // present-state bits 0..4 must stay together
//! mgr.make_bdd_group(0, 4, GroupFlags::DEFAULT);
//! // next-state bits 4..8 must stay together
//! mgr.make_bdd_group(4, 4, GroupFlags::DEFAULT);
//!
//! mgr.reduce_heap_with_groups();
//! ```

use crate::manager::Manager;

// -----------------------------------------------------------------------
// GroupFlags
// -----------------------------------------------------------------------

bitflags::bitflags! {
    /// Flags that control how a variable group behaves during reordering.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct GroupFlags: u8 {
        /// The group can be reordered freely (default behaviour).
        const DEFAULT  = 0b0000_0000;
        /// The relative order of variables inside this group is fixed.
        const FIXED    = 0b0000_0001;
        /// Leaf group — no sub-groups are allowed.
        const TERMINAL = 0b0000_0010;
    }
}

impl Default for GroupFlags {
    fn default() -> Self {
        GroupFlags::DEFAULT
    }
}

// -----------------------------------------------------------------------
// MtrNode
// -----------------------------------------------------------------------

/// A single node in the multi-way group tree.
///
/// Each node represents a contiguous range of variable levels
/// `[low .. low + size)`.  Children partition (or partially cover) that
/// range into sub-groups; any levels not covered by a child are treated
/// as singleton groups that can be sifted individually.
#[derive(Clone, Debug)]
pub struct MtrNode {
    /// Lowest variable level in this group.
    pub low: u16,
    /// Number of variable levels in this group.
    pub size: u16,
    /// Behavioural flags (FIXED, TERMINAL, …).
    pub flags: GroupFlags,
    /// Child groups (sub-groups).  Must be sorted by `low` and
    /// non-overlapping; each child's range must be contained in the
    /// parent's range.
    pub children: Vec<MtrNode>,
}

impl MtrNode {
    /// Create a new MTR node covering `[low .. low + size)`.
    pub fn new(low: u16, size: u16, flags: GroupFlags) -> Self {
        Self {
            low,
            size,
            flags,
            children: Vec::new(),
        }
    }

    /// The (exclusive) upper bound of the level range.
    #[inline]
    pub fn high(&self) -> u16 {
        self.low + self.size
    }

    /// Returns `true` when `level` falls inside this group.
    #[inline]
    pub fn contains(&self, level: u16) -> bool {
        level >= self.low && level < self.high()
    }

    /// Insert a new child group into this node.
    ///
    /// The new child's range `[low .. low + size)` must be fully contained
    /// in `self`'s range.  If the new child overlaps existing children those
    /// children become grand-children of the new node.
    ///
    /// Returns a reference to the newly inserted child.
    pub fn insert_child(&mut self, low: u16, size: u16, flags: GroupFlags) -> &MtrNode {
        assert!(
            low >= self.low && low + size <= self.high(),
            "child [{}, {}) is not contained in parent [{}, {})",
            low,
            low + size,
            self.low,
            self.high()
        );

        if self.flags.contains(GroupFlags::TERMINAL) {
            panic!("cannot insert a child into a TERMINAL group");
        }

        let high = low + size;

        // Collect indices of existing children that are fully contained
        // in the new child's range — they will become its grand-children.
        let mut absorbed: Vec<usize> = Vec::new();
        for (i, child) in self.children.iter().enumerate() {
            if child.low >= low && child.high() <= high {
                absorbed.push(i);
            } else {
                // Partial overlap is not allowed.
                assert!(
                    child.high() <= low || child.low >= high,
                    "new group [{}, {}) partially overlaps existing child [{}, {})",
                    low,
                    high,
                    child.low,
                    child.high()
                );
            }
        }

        // Build the new child, absorbing the covered children.
        let mut new_child = MtrNode::new(low, size, flags);
        // Remove absorbed children in reverse order so indices stay valid.
        for &i in absorbed.iter().rev() {
            new_child.children.push(self.children.remove(i));
        }
        // Sort grand-children by `low`.
        new_child.children.sort_by_key(|c| c.low);

        // Insert the new child in sorted position.
        let pos = self
            .children
            .binary_search_by_key(&low, |c| c.low)
            .unwrap_or_else(|p| p);
        self.children.insert(pos, new_child);

        &self.children[pos]
    }

    /// Find the innermost group that contains `level`.
    ///
    /// Returns `self` when no child covers `level`.
    pub fn find_group(&self, level: u16) -> &MtrNode {
        for child in &self.children {
            if child.contains(level) {
                return child.find_group(level);
            }
        }
        self
    }

    /// Mutable version of [`find_group`](Self::find_group).
    pub fn find_group_mut(&mut self, level: u16) -> &mut MtrNode {
        // Find which child contains the level (index-based to satisfy borrow checker)
        let child_idx = self.children.iter().position(|c| c.contains(level));
        match child_idx {
            Some(idx) => self.children[idx].find_group_mut(level),
            None => self,
        }
    }

    /// Recursively validate the tree invariants (debug builds).
    ///
    /// - Every child is fully contained in its parent.
    /// - Children are sorted by `low` and non-overlapping.
    /// - TERMINAL nodes have no children.
    pub fn validate(&self) {
        if self.flags.contains(GroupFlags::TERMINAL) {
            assert!(
                self.children.is_empty(),
                "TERMINAL group [{}, {}) has children",
                self.low,
                self.high()
            );
        }
        let mut prev_high: u16 = self.low;
        for child in &self.children {
            assert!(
                child.low >= prev_high,
                "overlapping children at level {}",
                child.low
            );
            assert!(
                child.high() <= self.high(),
                "child [{}, {}) exceeds parent [{}, {})",
                child.low,
                child.high(),
                self.low,
                self.high()
            );
            child.validate();
            prev_high = child.high();
        }
    }
}

// -----------------------------------------------------------------------
// MtrTree
// -----------------------------------------------------------------------

/// A complete multi-way group tree for a set of decision diagram variables.
///
/// The root node spans all variable levels `[0 .. num_vars)`.
#[derive(Clone, Debug)]
pub struct MtrTree {
    root: MtrNode,
}

impl MtrTree {
    /// Create a trivial tree with a single root spanning `[0 .. num_vars)`.
    pub fn new(num_vars: u16) -> Self {
        Self {
            root: MtrNode::new(0, num_vars, GroupFlags::DEFAULT),
        }
    }

    /// Access the root node.
    #[inline]
    pub fn root(&self) -> &MtrNode {
        &self.root
    }

    /// Mutable access to the root node.
    #[inline]
    pub fn root_mut(&mut self) -> &mut MtrNode {
        &mut self.root
    }

    /// Insert a group covering `[low .. low + size)` into the tree.
    ///
    /// The group is placed at the correct depth: it becomes a child of the
    /// deepest existing node whose range fully contains `[low .. low+size)`.
    pub fn make_group(&mut self, low: u16, size: u16, flags: GroupFlags) -> &MtrNode {
        assert!(
            low + size <= self.root.high(),
            "group [{}, {}) exceeds tree root [{}, {})",
            low,
            low + size,
            self.root.low,
            self.root.high()
        );
        Self::insert_into(&mut self.root, low, size, flags)
    }

    /// Recursive helper — insert into the deepest fitting node.
    fn insert_into(
        node: &mut MtrNode,
        low: u16,
        size: u16,
        flags: GroupFlags,
    ) -> &MtrNode {
        let high = low + size;

        // Find child whose range fully contains the new group (index-based for borrow checker)
        let child_idx = node.children.iter().position(|c| c.low <= low && c.high() >= high);
        if let Some(idx) = child_idx {
            return Self::insert_into(&mut node.children[idx], low, size, flags);
        }

        // No child fully contains it — insert as a new child of `node`.
        node.insert_child(low, size, flags)
    }

    /// Find the innermost group that contains `level`.
    pub fn find_group(&self, level: u16) -> &MtrNode {
        self.root.find_group(level)
    }

    /// Validate the entire tree (debug assertion helper).
    pub fn validate(&self) {
        self.root.validate();
    }

    /// Iterate over all *leaf-level* sifting blocks produced by the tree.
    ///
    /// Each block is `(low, size, fixed)` describing a contiguous range
    /// within which variables can (or cannot, if fixed) be freely sifted.
    /// This flattening is the basis for group-constrained sifting.
    pub fn leaf_blocks(&self) -> Vec<(u16, u16, bool)> {
        let mut blocks = Vec::new();
        Self::collect_leaf_blocks(&self.root, &mut blocks);
        blocks
    }

    fn collect_leaf_blocks(node: &MtrNode, out: &mut Vec<(u16, u16, bool)>) {
        if node.children.is_empty() {
            // Leaf of the tree — one sifting block.
            let fixed = node.flags.contains(GroupFlags::FIXED);
            out.push((node.low, node.size, fixed));
            return;
        }

        // Process gaps and children in level order.
        let mut cursor = node.low;
        for child in &node.children {
            // Gap before this child becomes a singleton-per-level block.
            while cursor < child.low {
                out.push((cursor, 1, false));
                cursor += 1;
            }
            Self::collect_leaf_blocks(child, out);
            cursor = child.high();
        }
        // Gap after last child.
        while cursor < node.high() {
            out.push((cursor, 1, false));
            cursor += 1;
        }
    }
}

// -----------------------------------------------------------------------
// Manager integration
// -----------------------------------------------------------------------

impl Manager {
    /// Create a new group in the BDD/ADD group tree covering levels
    /// `[low .. low + size)` and return a reference to the resulting
    /// [`MtrNode`].
    ///
    /// If no group tree exists yet, one is initialised automatically.
    pub fn make_tree_node(&mut self, low: u16, size: u16, flags: GroupFlags) -> &MtrNode {
        let n = self.num_vars;
        let tree = self.group_tree.get_or_insert_with(|| MtrTree::new(n));
        tree.make_group(low, size, flags)
    }

    /// Replace the entire group tree.
    pub fn set_group_tree(&mut self, tree: MtrTree) {
        self.group_tree = Some(tree);
    }

    /// Get a reference to the current group tree, if any.
    pub fn group_tree(&self) -> Option<&MtrTree> {
        self.group_tree.as_ref()
    }

    /// Convenience wrapper — same as [`make_tree_node`](Self::make_tree_node)
    /// but named to match CUDD's `Cudd_MakeTreeNode` for BDDs.
    pub fn make_bdd_group(&mut self, low: u16, size: u16, flags: GroupFlags) {
        self.make_tree_node(low, size, flags);
    }

    /// Create a group in the ZDD group tree.
    ///
    /// If no ZDD group tree exists yet, one is initialised automatically.
    pub fn make_zdd_group(&mut self, low: u16, size: u16, flags: GroupFlags) {
        let n = self.num_zdd_vars;
        let tree = self.zdd_group_tree.get_or_insert_with(|| MtrTree::new(n));
        tree.make_group(low, size, flags);
    }

    // ----- bound variables ---------------------------------------------

    /// Bind a variable so that it cannot be moved during reordering.
    pub fn bind_var(&mut self, var: u16) {
        assert!(
            (var as usize) < self.num_vars as usize,
            "variable index {} out of range (num_vars = {})",
            var,
            self.num_vars
        );
        if self.bound_vars.len() <= var as usize {
            self.bound_vars.resize(self.num_vars as usize, false);
        }
        self.bound_vars[var as usize] = true;
    }

    /// Unbind a variable, allowing it to be reordered again.
    pub fn unbind_var(&mut self, var: u16) {
        if (var as usize) < self.bound_vars.len() {
            self.bound_vars[var as usize] = false;
        }
    }

    /// Returns `true` when `var` is bound (excluded from reordering).
    pub fn is_var_bound(&self, var: u16) -> bool {
        self.bound_vars
            .get(var as usize)
            .copied()
            .unwrap_or(false)
    }

    // ----- group-constrained sifting -----------------------------------

    /// Run sifting reordering while respecting the group tree constraints.
    ///
    /// Variables are only moved within their innermost group.  FIXED groups
    /// keep their internal order unchanged.  Bound variables are never moved.
    pub fn reduce_heap_with_groups(&mut self) {
        let n = self.num_vars;
        if n <= 1 {
            return;
        }

        // If there is no group tree, fall back to a root-only tree (no
        // constraints at all, equivalent to plain sifting).
        if self.group_tree.is_none() {
            self.group_tree = Some(MtrTree::new(n));
        }

        // Gather the leaf blocks.  We clone the tree snapshot so we don't
        // borrow `self` while mutating it during sifting.
        let blocks = self.group_tree.as_ref().unwrap().leaf_blocks();

        // Process each leaf block independently.
        for (block_low, block_size, fixed) in &blocks {
            if *fixed || *block_size <= 1 {
                continue;
            }
            self.sift_block(*block_low, *block_size);
        }

        // After all blocks, try sifting whole groups inside their parent.
        self.sift_groups_in_tree();

        self.cache.clear();
        self.reordered = true;
    }

    /// Sift each variable inside a non-fixed leaf block to its best
    /// position, respecting block boundaries.
    fn sift_block(&mut self, block_low: u16, block_size: u16) {
        let low = block_low as u32;
        let high = (block_low + block_size) as u32;

        // Collect variables in this block, sorted by subtable size (largest
        // first) so the most impactful variables are sifted first.
        let mut vars: Vec<u16> = (low..high)
            .map(|l| self.inv_perm[l as usize] as u16)
            .collect();
        vars.sort_by(|&a, &b| {
            self.subtable_size(b).cmp(&self.subtable_size(a))
        });

        for var in vars {
            if self.is_var_bound(var) {
                continue;
            }
            self.sift_variable_in_range(var, low, high);
        }
    }

    /// Sift `var` only within `[range_low .. range_high)`.
    fn sift_variable_in_range(&mut self, var: u16, range_low: u32, range_high: u32) {
        let start_level = self.perm[var as usize];
        debug_assert!(start_level >= range_low && start_level < range_high);

        let mut best_level = start_level;
        let mut best_size = self.total_live_nodes();

        // Sift down within range.
        let mut cur = start_level;
        while cur + 1 < range_high {
            self.swap_adjacent_levels(cur);
            cur += 1;
            let sz = self.total_live_nodes();
            if sz < best_size {
                best_size = sz;
                best_level = cur;
            }
        }

        // Sift back to start.
        while cur > start_level {
            self.swap_adjacent_levels(cur - 1);
            cur -= 1;
        }

        // Sift up within range.
        while cur > range_low {
            self.swap_adjacent_levels(cur - 1);
            cur -= 1;
            let sz = self.total_live_nodes();
            if sz < best_size {
                best_size = sz;
                best_level = cur;
            }
        }

        // Move to the best level found.
        while cur < best_level {
            self.swap_adjacent_levels(cur);
            cur += 1;
        }
        while cur > best_level {
            self.swap_adjacent_levels(cur - 1);
            cur -= 1;
        }
    }

    /// Sift whole groups as single units within their parent group.
    ///
    /// This implements the second phase of CUDD's group sifting: after
    /// leaf-level sifting is done, each non-root group is treated as an
    /// atomic block and tried at every position inside its parent.
    fn sift_groups_in_tree(&mut self) {
        // Take a snapshot of the tree so we can iterate without borrowing
        // `self` immutably.
        let tree = match self.group_tree.as_ref() {
            Some(t) => t.clone(),
            None => return,
        };
        self.sift_children_of(&tree.root);
    }

    /// For each child of `parent`, try shifting the child-block to every
    /// valid position inside the parent and keep the best.
    fn sift_children_of(&mut self, parent: &MtrNode) {
        if parent.children.len() <= 1 {
            // Recurse into the single child, if any.
            for child in &parent.children {
                self.sift_children_of(child);
            }
            return;
        }

        // Recurse first (bottom-up).
        for child in &parent.children {
            self.sift_children_of(child);
        }

        // Now sift the child blocks within the parent range.
        // We identify each child by its `(low, size)` and try moving the
        // block left and right by swapping *all* its levels with the
        // adjacent level outside the block.
        //
        // Because this is a heuristic optimisation, we do a simplified
        // version: for each adjacent pair of child blocks (plus singleton
        // gaps), try swapping them and keep the swap if it reduces size.
        let _parent_low = parent.low as u32;
        let _parent_high = parent.high() as u32;

        // Build a flat list of contiguous segments inside the parent.
        let segments = self.segments_of(parent);
        if segments.len() <= 1 {
            return;
        }

        // Bubble-sort style: repeatedly scan adjacent segments and swap if
        // it reduces total live nodes.  At most O(k^2) iterations for k
        // segments, which is acceptable because k is typically small.
        let mut improved = true;
        while improved {
            improved = false;
            let segs = self.segments_of(parent);
            for i in 0..segs.len() - 1 {
                let (a_low, a_size) = segs[i];
                let (b_low, b_size) = segs[i + 1];
                debug_assert_eq!(a_low + a_size, b_low);

                let before = self.total_live_nodes();
                self.swap_blocks(a_low, a_size, b_size);
                let after = self.total_live_nodes();
                if after >= before {
                    // Undo — swap back.
                    self.swap_blocks(a_low, b_size, a_size);
                } else {
                    improved = true;
                }
            }
        }
    }

    /// Return the contiguous segments (child blocks + singleton gaps) that
    /// partition a parent group.
    fn segments_of(&self, parent: &MtrNode) -> Vec<(u32, u32)> {
        let mut segs = Vec::new();
        let mut cursor = parent.low as u32;
        for child in &parent.children {
            let cl = child.low as u32;
            // Gap before this child — each level is its own segment.
            while cursor < cl {
                segs.push((cursor, 1));
                cursor += 1;
            }
            segs.push((cl, child.size as u32));
            cursor = child.high() as u32;
        }
        let ph = parent.high() as u32;
        while cursor < ph {
            segs.push((cursor, 1));
            cursor += 1;
        }
        segs
    }

    /// Swap two adjacent contiguous blocks of levels.
    ///
    /// Block A occupies `[low .. low + size_a)` and block B occupies
    /// `[low + size_a .. low + size_a + size_b)`.  After the call, B
    /// occupies `[low .. low + size_b)` and A occupies
    /// `[low + size_b .. low + size_b + size_a)`.
    ///
    /// This is done by repeated adjacent-level swaps (bubble the smaller
    /// block across the larger one).
    fn swap_blocks(&mut self, low: u32, size_a: u32, size_b: u32) {
        // Bubble each level of A across all levels of B.
        // A is at [low .. low+size_a), B at [low+size_a .. low+size_a+size_b).
        //
        // We move A one level at a time to the right past B:
        //   for each element of A (from rightmost to leftmost), swap it
        //   across all of B.
        for i in (0..size_a).rev() {
            let mut cur = low + i;
            for _ in 0..size_b {
                self.swap_adjacent_levels(cur);
                cur += 1;
            }
        }
    }
}

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mtr_node_basic() {
        let node = MtrNode::new(0, 8, GroupFlags::DEFAULT);
        assert_eq!(node.low, 0);
        assert_eq!(node.size, 8);
        assert_eq!(node.high(), 8);
        assert!(node.contains(0));
        assert!(node.contains(7));
        assert!(!node.contains(8));
    }

    #[test]
    fn test_mtr_tree_insert() {
        let mut tree = MtrTree::new(8);
        tree.make_group(0, 4, GroupFlags::DEFAULT);
        tree.make_group(4, 4, GroupFlags::FIXED);
        tree.make_group(0, 2, GroupFlags::TERMINAL);

        tree.validate();
        assert_eq!(tree.root().children.len(), 2);

        // The group [0,2) should be a child of [0,4).
        let g04 = &tree.root().children[0];
        assert_eq!(g04.low, 0);
        assert_eq!(g04.size, 4);
        assert_eq!(g04.children.len(), 1);
        assert_eq!(g04.children[0].low, 0);
        assert_eq!(g04.children[0].size, 2);
        assert!(g04.children[0].flags.contains(GroupFlags::TERMINAL));
    }

    #[test]
    fn test_find_group() {
        let mut tree = MtrTree::new(8);
        tree.make_group(0, 4, GroupFlags::DEFAULT);
        tree.make_group(4, 4, GroupFlags::DEFAULT);

        let g = tree.find_group(2);
        assert_eq!(g.low, 0);
        assert_eq!(g.size, 4);

        let g2 = tree.find_group(5);
        assert_eq!(g2.low, 4);
        assert_eq!(g2.size, 4);
    }

    #[test]
    fn test_leaf_blocks() {
        let mut tree = MtrTree::new(8);
        tree.make_group(0, 4, GroupFlags::DEFAULT);
        tree.make_group(4, 4, GroupFlags::FIXED);

        let blocks = tree.leaf_blocks();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0], (0, 4, false));
        assert_eq!(blocks[1], (4, 4, true));
    }

    #[test]
    fn test_leaf_blocks_with_gaps() {
        let mut tree = MtrTree::new(8);
        tree.make_group(2, 3, GroupFlags::DEFAULT);

        let blocks = tree.leaf_blocks();
        // levels 0,1 are singletons, then [2,5) block, then 5,6,7 singletons
        assert_eq!(blocks.len(), 6);
        assert_eq!(blocks[0], (0, 1, false));
        assert_eq!(blocks[1], (1, 1, false));
        assert_eq!(blocks[2], (2, 3, false));
        assert_eq!(blocks[3], (5, 1, false));
    }

    #[test]
    #[should_panic(expected = "cannot insert a child into a TERMINAL group")]
    fn test_terminal_no_children() {
        let mut tree = MtrTree::new(8);
        tree.make_group(0, 4, GroupFlags::TERMINAL);
        tree.make_group(0, 2, GroupFlags::DEFAULT); // should panic
    }

    #[test]
    #[should_panic(expected = "partially overlaps")]
    fn test_partial_overlap_panics() {
        let mut tree = MtrTree::new(8);
        tree.make_group(0, 4, GroupFlags::DEFAULT);
        tree.make_group(2, 4, GroupFlags::DEFAULT); // overlaps [0,4) partially
    }

    #[test]
    fn test_bind_unbind_var() {
        let mut mgr = Manager::new();
        for _ in 0..4 {
            mgr.bdd_new_var();
        }
        assert!(!mgr.is_var_bound(0));
        mgr.bind_var(0);
        assert!(mgr.is_var_bound(0));
        mgr.unbind_var(0);
        assert!(!mgr.is_var_bound(0));
    }

    #[test]
    fn test_make_bdd_group() {
        let mut mgr = Manager::new();
        for _ in 0..8 {
            mgr.bdd_new_var();
        }
        mgr.make_bdd_group(0, 4, GroupFlags::DEFAULT);
        mgr.make_bdd_group(4, 4, GroupFlags::FIXED);

        let tree = mgr.group_tree().expect("group tree should exist");
        tree.validate();
        assert_eq!(tree.root().children.len(), 2);
    }

    #[test]
    fn test_reduce_heap_with_groups_no_panic() {
        let mut mgr = Manager::new();
        for _ in 0..8 {
            mgr.bdd_new_var();
        }
        mgr.make_bdd_group(0, 4, GroupFlags::DEFAULT);
        mgr.make_bdd_group(4, 4, GroupFlags::FIXED);
        // Should not panic; correctness of reordering is validated by
        // the BDD invariants being preserved.
        mgr.reduce_heap_with_groups();
    }
}
