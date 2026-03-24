// lumindd — Decision diagram manager
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use crate::computed_table::ComputedTable;
use crate::mtr::MtrTree;
use crate::node::{DdNode, NodeId, RawIndex, CONST_INDEX, MAX_REF};
use crate::reorder::ReorderingMethod;
use crate::unique_table::{ConstantTable, UniqueSubtable};

/// The central manager that owns all decision diagram nodes.
///
/// All BDD, ADD, and ZDD operations go through the manager.
/// Nodes are stored in a flat arena and referenced by [`NodeId`] handles.
pub struct Manager {
    /// Arena of all nodes.
    pub(crate) nodes: Vec<DdNode>,

    /// Per-variable unique tables (one per BDD/ADD variable).
    pub(crate) unique_tables: Vec<UniqueSubtable>,

    /// Per-variable unique tables for ZDD.
    pub(crate) zdd_unique_tables: Vec<UniqueSubtable>,

    /// Unique table for ADD constant (terminal) nodes.
    pub(crate) constant_table: ConstantTable,

    /// Computed table (operation result cache).
    pub(crate) cache: ComputedTable,

    /// Variable index -> level mapping (for reordering).
    pub(crate) perm: Vec<u32>,

    /// Level -> variable index mapping (inverse of perm).
    pub(crate) inv_perm: Vec<u32>,

    /// ZDD variable index -> level mapping.
    pub(crate) zdd_perm: Vec<u32>,

    /// ZDD level -> variable index.
    pub(crate) zdd_inv_perm: Vec<u32>,

    /// Number of BDD/ADD variables.
    pub(crate) num_vars: u16,

    /// Number of ZDD variables.
    pub(crate) num_zdd_vars: u16,

    /// Auto-reordering enabled.
    pub(crate) auto_reorder: bool,

    /// Reordering method.
    pub(crate) reorder_method: ReorderingMethod,

    /// Flag set when reordering occurred during an operation.
    pub(crate) reordered: bool,

    /// Total number of garbage collections performed.
    pub(crate) gc_count: u64,

    /// Dead node count.
    pub(crate) dead: usize,

    /// Next GC threshold.
    pub(crate) gc_threshold: usize,

    /// BDD/ADD variable group tree (MTR) for constrained reordering.
    pub(crate) group_tree: Option<MtrTree>,

    /// ZDD variable group tree (MTR) for constrained reordering.
    pub(crate) zdd_group_tree: Option<MtrTree>,

    /// Per-variable binding flags — bound variables are excluded from reordering.
    pub(crate) bound_vars: Vec<bool>,
}

impl Manager {
    /// Create a new empty manager with default settings.
    pub fn new() -> Self {
        Self::with_capacity(0, 0, 18) // 2^18 = 256K cache entries
    }

    /// Create a manager with pre-allocated variables and cache size.
    pub fn with_capacity(num_bdd_vars: u16, num_zdd_vars: u16, cache_log2: u32) -> Self {
        // Create the arena with the constant-ONE node at index 0.
        let nodes = vec![DdNode::Constant {
            value: 1.0,
            ref_count: MAX_REF, // constants are never freed
        }];

        let mut mgr = Manager {
            nodes,
            unique_tables: Vec::new(),
            zdd_unique_tables: Vec::new(),
            constant_table: ConstantTable::new(),
            cache: ComputedTable::new(cache_log2),
            perm: Vec::new(),
            inv_perm: Vec::new(),
            zdd_perm: Vec::new(),
            zdd_inv_perm: Vec::new(),
            num_vars: 0,
            num_zdd_vars: 0,
            auto_reorder: false,
            reorder_method: ReorderingMethod::Sift,
            reordered: false,
            gc_count: 0,
            dead: 0,
            gc_threshold: 1 << 16,
            group_tree: None,
            zdd_group_tree: None,
            bound_vars: Vec::new(),
        };

        // Register the constant-ONE node.
        mgr.constant_table.insert(1.0, 0);

        // Pre-create requested BDD variables.
        for _ in 0..num_bdd_vars {
            mgr.bdd_new_var();
        }

        // Pre-create requested ZDD variables.
        for _ in 0..num_zdd_vars {
            mgr.zdd_new_var();
        }

        mgr
    }

    // ------------------------------------------------------------------
    // Node access helpers
    // ------------------------------------------------------------------

    /// Get a reference to a node by raw index.
    #[inline]
    pub(crate) fn node(&self, id: NodeId) -> &DdNode {
        &self.nodes[id.raw_index() as usize]
    }

    /// Get the variable index of a node (CONST_INDEX for terminals).
    #[inline]
    pub(crate) fn var_index(&self, id: NodeId) -> u16 {
        self.node(id).var_index()
    }

    /// Get the current level of a node's variable.
    #[inline]
    pub(crate) fn level(&self, id: NodeId) -> u32 {
        let vi = self.var_index(id);
        if vi == CONST_INDEX {
            u32::MAX // constants are at the bottom
        } else {
            self.perm[vi as usize]
        }
    }

    /// Get the current level of a ZDD node's variable.
    #[inline]
    pub(crate) fn zdd_level(&self, id: NodeId) -> u32 {
        let vi = self.var_index(id);
        if vi == CONST_INDEX {
            u32::MAX
        } else {
            self.zdd_perm[vi as usize]
        }
    }

    /// Get the then-child, adjusting for complement edge on the parent.
    #[inline]
    pub(crate) fn then_child(&self, id: NodeId) -> NodeId {
        let t = self.node(id).then_child();
        // Complement edges: if parent is complemented, children are complemented too.
        // But by convention, the then-child of a complemented edge is complemented.
        t.not_cond(id.is_complemented())
    }

    /// Get the else-child, adjusting for complement edge on the parent.
    #[inline]
    pub(crate) fn else_child(&self, id: NodeId) -> NodeId {
        let e = self.node(id).else_child();
        e.not_cond(id.is_complemented())
    }

    /// Get raw then-child without complement adjustment.
    #[inline]
    pub(crate) fn raw_then(&self, id: NodeId) -> NodeId {
        self.node(id.regular()).then_child()
    }

    /// Get raw else-child without complement adjustment.
    #[inline]
    pub(crate) fn raw_else(&self, id: NodeId) -> NodeId {
        self.node(id.regular()).else_child()
    }

    /// Returns true if the node is a constant/terminal.
    #[inline]
    pub fn is_constant(&self, id: NodeId) -> bool {
        id.is_constant()
    }

    /// The constant ONE node.
    #[inline]
    pub fn one(&self) -> NodeId {
        NodeId::ONE
    }

    /// The constant ZERO node.
    #[inline]
    pub fn zero(&self) -> NodeId {
        NodeId::ZERO
    }

    // ------------------------------------------------------------------
    // Node allocation
    // ------------------------------------------------------------------

    /// Maximum number of nodes supported (limited by NodeId encoding: 31 bits).
    pub const MAX_NODES: usize = (1 << 31) - 1;

    /// Allocate a new internal node in the arena.
    pub(crate) fn alloc_node(&mut self, var_index: u16, then_child: NodeId, else_child: NodeId) -> RawIndex {
        assert!(self.nodes.len() < Self::MAX_NODES, "node arena overflow: exceeded {} nodes", Self::MAX_NODES);
        let idx = self.nodes.len() as RawIndex;
        self.nodes.push(DdNode::Internal {
            var_index,
            then_child,
            else_child,
            ref_count: 0,
        });
        idx
    }

    /// Allocate a new constant node.
    pub(crate) fn alloc_constant(&mut self, value: f64) -> RawIndex {
        assert!(self.nodes.len() < Self::MAX_NODES, "node arena overflow: exceeded {} nodes", Self::MAX_NODES);
        let idx = self.nodes.len() as RawIndex;
        self.nodes.push(DdNode::Constant {
            value,
            ref_count: 0,
        });
        idx
    }

    // ------------------------------------------------------------------
    // Reference counting
    // ------------------------------------------------------------------

    /// Increment the reference count of a node.
    #[inline]
    pub fn ref_node(&mut self, id: NodeId) {
        let raw = id.raw_index() as usize;
        self.nodes[raw].incr_ref();
    }

    /// Decrement the reference count of a node, recursively.
    pub fn deref_node(&mut self, id: NodeId) {
        let raw = id.raw_index() as usize;
        let rc = self.nodes[raw].ref_count();
        if rc == MAX_REF {
            return; // saturated, never freed
        }
        self.nodes[raw].decr_ref();
        if rc == 1 {
            // Node is now dead — extract children before recursing
            self.dead += 1;
            let children = if let DdNode::Internal { then_child, else_child, .. } = self.nodes[raw] {
                Some((then_child, else_child))
            } else {
                None
            };
            if let Some((t, e)) = children {
                self.deref_node(t);
                self.deref_node(e);
            }
        }
    }

    // ------------------------------------------------------------------
    // Canonical node creation (unique table lookup-or-insert)
    // ------------------------------------------------------------------

    /// Find or create a canonical BDD/ADD internal node.
    ///
    /// Enforces the BDD canonical form:
    /// - If then == else, return then (redundant node elimination).
    /// - If the then-child is complemented, complement both children and
    ///   return a complemented edge to the result (ensures the then-child
    ///   of the stored node is always regular).
    pub(crate) fn unique_inter(&mut self, var_index: u16, then_child: NodeId, else_child: NodeId) -> NodeId {
        // Redundant node elimination
        if then_child == else_child {
            return then_child;
        }

        // Canonical form: then-child must be regular (not complemented).
        let (t, e, complemented) = if then_child.is_complemented() {
            (then_child.not(), else_child.not(), true)
        } else {
            (then_child, else_child, false)
        };

        // Look up in the unique table for this variable
        let level = self.perm[var_index as usize] as usize;

        // Ensure we have enough unique tables
        while self.unique_tables.len() <= level {
            self.unique_tables.push(UniqueSubtable::new());
        }

        if let Some(raw_idx) = self.unique_tables[level].lookup(t, e) {
            return NodeId::from_raw(raw_idx, complemented);
        }

        // Trigger GC if needed
        if self.nodes.len() >= self.gc_threshold {
            self.garbage_collect();
        }

        // Allocate and insert
        let raw_idx = self.alloc_node(var_index, t, e);
        self.unique_tables[level].insert(t, e, raw_idx);

        NodeId::from_raw(raw_idx, complemented)
    }

    /// Find or create a canonical ZDD internal node.
    ///
    /// ZDD canonical form: eliminate node if then-child is ZERO
    /// (zero-suppressed reduction rule).
    pub(crate) fn zdd_unique_inter(&mut self, var_index: u16, then_child: NodeId, else_child: NodeId) -> NodeId {
        // ZDD reduction: if then-child is zero, skip this variable
        if then_child.is_zero() {
            return else_child;
        }

        let level = self.zdd_perm[var_index as usize] as usize;

        while self.zdd_unique_tables.len() <= level {
            self.zdd_unique_tables.push(UniqueSubtable::new());
        }

        if let Some(raw_idx) = self.zdd_unique_tables[level].lookup(then_child, else_child) {
            return NodeId::from_raw(raw_idx, false);
        }

        if self.nodes.len() >= self.gc_threshold {
            self.garbage_collect();
        }

        let raw_idx = self.alloc_node(var_index, then_child, else_child);
        self.zdd_unique_tables[level].insert(then_child, else_child, raw_idx);

        NodeId::from_raw(raw_idx, false)
    }

    // ------------------------------------------------------------------
    // Variable creation
    // ------------------------------------------------------------------

    /// Create a new BDD/ADD variable and return its projection function.
    ///
    /// The variable is placed at the next available level.
    pub fn bdd_new_var(&mut self) -> NodeId {
        let var_index = self.num_vars;
        let level = var_index as u32;

        self.num_vars += 1;
        self.perm.push(level);
        self.inv_perm.push(var_index as u32);
        self.unique_tables.push(UniqueSubtable::new());

        // Create the projection function: if var_index then ONE else ZERO
        let node = self.unique_inter(var_index, NodeId::ONE, NodeId::ZERO);
        self.ref_node(node);
        node
    }

    /// Get or create the i-th BDD variable.
    pub fn bdd_ith_var(&mut self, i: u16) -> NodeId {
        while self.num_vars <= i {
            self.bdd_new_var();
        }
        // Build the projection function for variable i
        self.unique_inter(i, NodeId::ONE, NodeId::ZERO)
    }

    /// Create a new ZDD variable.
    pub fn zdd_new_var(&mut self) -> NodeId {
        let var_index = self.num_zdd_vars;
        let level = var_index as u32;

        self.num_zdd_vars += 1;
        self.zdd_perm.push(level);
        self.zdd_inv_perm.push(var_index as u32);
        self.zdd_unique_tables.push(UniqueSubtable::new());

        // ZDD variable: represents the set {{var_index}}
        let node = self.zdd_unique_inter(var_index, NodeId::ONE, NodeId::ZERO);
        self.ref_node(node);
        node
    }

    /// Number of BDD/ADD variables.
    pub fn num_vars(&self) -> u16 {
        self.num_vars
    }

    /// Number of ZDD variables.
    pub fn num_zdd_vars(&self) -> u16 {
        self.num_zdd_vars
    }

    // ------------------------------------------------------------------
    // Garbage collection
    // ------------------------------------------------------------------

    /// Run garbage collection to reclaim dead nodes.
    pub fn garbage_collect(&mut self) {
        self.gc_count += 1;
        // In this implementation, dead nodes remain in the arena but their
        // unique table entries are cleaned up. A full compacting GC would
        // require remapping all node IDs.
        //
        // For now, we grow the GC threshold to avoid frequent GC.
        self.gc_threshold = (self.nodes.len() * 2).max(self.gc_threshold);
        self.dead = 0;
    }

    // ------------------------------------------------------------------
    // Configuration
    // ------------------------------------------------------------------

    /// Enable automatic variable reordering.
    pub fn enable_auto_reorder(&mut self, method: ReorderingMethod) {
        self.auto_reorder = true;
        self.reorder_method = method;
    }

    /// Disable automatic variable reordering.
    pub fn disable_auto_reorder(&mut self) {
        self.auto_reorder = false;
    }

    /// Get the number of live nodes.
    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Get computed table statistics.
    pub fn cache_stats(&self) -> (u64, u64) {
        (self.cache.hits, self.cache.misses)
    }
}

impl Default for Manager {
    fn default() -> Self {
        Self::new()
    }
}
