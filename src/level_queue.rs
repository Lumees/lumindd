// lumindd — Level queue for level-by-level BDD traversal
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

//! A level-ordered queue for BFS (breadth-first) traversal of BDD/ADD DAGs.
//!
//! The [`LevelQueue`] maintains one bucket per variable level, allowing
//! efficient level-by-level processing of decision diagram nodes. This is
//! useful for width computation, profile analysis, and level-based
//! algorithms such as sifting.

use std::collections::HashSet;

use crate::manager::Manager;
use crate::node::NodeId;

/// A level-ordered queue that groups nodes by their variable level.
///
/// Each level has its own bucket (a `Vec<NodeId>`). Nodes are enqueued
/// into the bucket corresponding to their level and dequeued one level
/// at a time.
pub struct LevelQueue {
    /// One bucket per level. Index = level number.
    buckets: Vec<Vec<NodeId>>,
    /// Number of variable levels this queue supports.
    num_vars: usize,
}

impl LevelQueue {
    /// Create a new level queue for a diagram with `num_vars` variable levels.
    pub fn new(num_vars: usize) -> Self {
        LevelQueue {
            buckets: vec![Vec::new(); num_vars],
            num_vars,
        }
    }

    /// Add a node to the bucket for the given level.
    ///
    /// # Panics
    /// Panics if `level >= num_vars`.
    pub fn enqueue(&mut self, level: u32, node: NodeId) {
        assert!(
            (level as usize) < self.num_vars,
            "level {} out of range (num_vars = {})",
            level,
            self.num_vars
        );
        self.buckets[level as usize].push(node);
    }

    /// Remove and return all nodes at the given level.
    ///
    /// Returns an empty `Vec` if the level is empty or out of range.
    pub fn dequeue_level(&mut self, level: u32) -> Vec<NodeId> {
        if (level as usize) >= self.num_vars {
            return Vec::new();
        }
        std::mem::take(&mut self.buckets[level as usize])
    }

    /// Check if the queue has no nodes at any level.
    pub fn is_empty(&self) -> bool {
        self.buckets.iter().all(|b| b.is_empty())
    }

    /// Return the total number of nodes across all levels.
    pub fn total_size(&self) -> usize {
        self.buckets.iter().map(|b| b.len()).sum()
    }

    /// Return the number of levels this queue supports.
    pub fn num_levels(&self) -> usize {
        self.num_vars
    }

    /// Peek at the nodes at a given level without removing them.
    pub fn peek_level(&self, level: u32) -> &[NodeId] {
        if (level as usize) >= self.num_vars {
            return &[];
        }
        &self.buckets[level as usize]
    }
}

impl Manager {
    /// Perform a BFS traversal of the BDD/ADD rooted at `f`, returning
    /// all nodes grouped by their variable level.
    ///
    /// The result is a vector indexed by level, where each entry contains
    /// the unique node IDs at that level. Terminal nodes are not included
    /// (they have no variable level).
    ///
    /// # Example
    ///
    /// ```rust
    /// use lumindd::Manager;
    ///
    /// let mut mgr = Manager::new();
    /// let x = mgr.bdd_new_var();
    /// let y = mgr.bdd_new_var();
    /// let f = mgr.bdd_and(x, y);
    ///
    /// let levels = mgr.bdd_level_traverse(f);
    /// // Level 0 has the root node (x), level 1 has the y node
    /// assert_eq!(levels.len(), 2);
    /// ```
    pub fn bdd_level_traverse(&self, f: NodeId) -> Vec<Vec<NodeId>> {
        let nv = self.num_vars as usize;
        if nv == 0 || f.is_constant() {
            return vec![Vec::new(); nv];
        }

        let mut queue = LevelQueue::new(nv);
        let mut visited = HashSet::new();

        // Use the regular (non-complemented) node for traversal
        let root = f.regular();
        if !root.is_constant() {
            let level = self.level(root);
            if (level as usize) < nv {
                queue.enqueue(level, root);
                visited.insert(root);
            }
        }

        // BFS: process level by level
        let mut result = vec![Vec::new(); nv];

        for lev in 0..nv {
            let nodes = queue.dequeue_level(lev as u32);
            for &node in &nodes {
                // Enqueue children
                let t = self.raw_then(node);
                let e = self.raw_else(node);

                for child in [t, e] {
                    let child_reg = child.regular();
                    if !child_reg.is_constant() && !visited.contains(&child_reg) {
                        let child_level = self.level(child_reg);
                        if (child_level as usize) < nv {
                            queue.enqueue(child_level, child_reg);
                            visited.insert(child_reg);
                        }
                    }
                }
            }
            result[lev] = nodes;
        }

        result
    }

    /// Count the number of unique internal nodes at a specific variable level
    /// in the DAG rooted at `f`.
    ///
    /// Returns 0 if `f` is a constant or `level` is out of range.
    pub fn bdd_width_at_level(&self, f: NodeId, level: u32) -> usize {
        let nv = self.num_vars as usize;
        if f.is_constant() || (level as usize) >= nv {
            return 0;
        }

        let levels = self.bdd_level_traverse(f);
        if (level as usize) < levels.len() {
            levels[level as usize].len()
        } else {
            0
        }
    }

    /// Find the variable level with the maximum width (most nodes) in the
    /// DAG rooted at `f`.
    ///
    /// Returns `(level, width)`. If `f` is a constant, returns `(0, 0)`.
    pub fn bdd_max_width(&self, f: NodeId) -> (u32, usize) {
        if f.is_constant() {
            return (0, 0);
        }

        let levels = self.bdd_level_traverse(f);
        let mut max_level = 0u32;
        let mut max_width = 0usize;

        for (lev, nodes) in levels.iter().enumerate() {
            if nodes.len() > max_width {
                max_width = nodes.len();
                max_level = lev as u32;
            }
        }

        (max_level, max_width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_queue_basic() {
        let mut q = LevelQueue::new(4);
        assert!(q.is_empty());
        assert_eq!(q.total_size(), 0);

        q.enqueue(0, NodeId::ONE);
        q.enqueue(0, NodeId::ZERO);
        q.enqueue(2, NodeId::ONE);

        assert!(!q.is_empty());
        assert_eq!(q.total_size(), 3);

        let level0 = q.dequeue_level(0);
        assert_eq!(level0.len(), 2);
        assert_eq!(q.total_size(), 1);

        let level1 = q.dequeue_level(1);
        assert!(level1.is_empty());

        let level2 = q.dequeue_level(2);
        assert_eq!(level2.len(), 1);
        assert!(q.is_empty());
    }

    #[test]
    fn level_queue_out_of_range_dequeue() {
        let mut q = LevelQueue::new(2);
        let nodes = q.dequeue_level(10);
        assert!(nodes.is_empty());
    }

    #[test]
    #[should_panic(expected = "level 5 out of range")]
    fn level_queue_enqueue_panic() {
        let mut q = LevelQueue::new(3);
        q.enqueue(5, NodeId::ONE);
    }

    #[test]
    fn level_queue_peek() {
        let mut q = LevelQueue::new(3);
        q.enqueue(1, NodeId::ONE);
        assert_eq!(q.peek_level(1).len(), 1);
        assert_eq!(q.peek_level(0).len(), 0);
        assert_eq!(q.peek_level(99).len(), 0);
    }

    #[test]
    fn traverse_constant() {
        let mgr = Manager::new();
        let levels = mgr.bdd_level_traverse(NodeId::ONE);
        assert!(levels.iter().all(|l| l.is_empty()));
    }

    #[test]
    fn traverse_single_var() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var(); // level 0
        let levels = mgr.bdd_level_traverse(x);
        assert_eq!(levels.len(), 1);
        assert_eq!(levels[0].len(), 1); // one node at level 0
    }

    #[test]
    fn traverse_and() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var(); // level 0
        let y = mgr.bdd_new_var(); // level 1
        let f = mgr.bdd_and(x, y);

        let levels = mgr.bdd_level_traverse(f);
        assert_eq!(levels.len(), 2);
        // x AND y: root is x (level 0), then-child is y (level 1)
        assert_eq!(levels[0].len(), 1);
        assert_eq!(levels[1].len(), 1);
    }

    #[test]
    fn traverse_xor() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var(); // level 0
        let y = mgr.bdd_new_var(); // level 1
        let f = mgr.bdd_xor(x, y);

        let levels = mgr.bdd_level_traverse(f);
        assert_eq!(levels.len(), 2);
        assert_eq!(levels[0].len(), 1); // x at level 0
        assert_eq!(levels[1].len(), 1); // y at level 1 (shared via complement)
    }

    #[test]
    fn width_at_level() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y);

        assert_eq!(mgr.bdd_width_at_level(f, 0), 1);
        assert_eq!(mgr.bdd_width_at_level(f, 1), 1);
        assert_eq!(mgr.bdd_width_at_level(NodeId::ONE, 0), 0);
    }

    #[test]
    fn max_width_simple() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let z = mgr.bdd_new_var();

        // (x AND y) OR (x AND z) — may create wider BDD
        let xy = mgr.bdd_and(x, y);
        let xz = mgr.bdd_and(x, z);
        let f = mgr.bdd_or(xy, xz);

        let (level, width) = mgr.bdd_max_width(f);
        // Should have at least 1 node at the max-width level
        assert!(width >= 1);
        assert!(level < 3);
    }

    #[test]
    fn max_width_constant() {
        let mgr = Manager::new();
        let (level, width) = mgr.bdd_max_width(NodeId::ZERO);
        assert_eq!(level, 0);
        assert_eq!(width, 0);
    }

    #[test]
    fn traverse_shared_nodes() {
        let mut mgr = Manager::new();
        let a = mgr.bdd_new_var(); // level 0
        let b = mgr.bdd_new_var(); // level 1
        let c = mgr.bdd_new_var(); // level 2

        // Build f = (a AND b) OR (a AND c) — b and c may share structure
        let ab = mgr.bdd_and(a, b);
        let ac = mgr.bdd_and(a, c);
        let f = mgr.bdd_or(ab, ac);

        let levels = mgr.bdd_level_traverse(f);
        // Each node should appear exactly once (no duplicates)
        let total: usize = levels.iter().map(|l| l.len()).sum();
        let mut all_nodes: Vec<NodeId> = levels.into_iter().flatten().collect();
        all_nodes.sort_by_key(|n| n.raw_index());
        all_nodes.dedup();
        assert_eq!(total, all_nodes.len(), "no duplicate nodes in traversal");
    }
}
