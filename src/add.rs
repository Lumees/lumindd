// lumindd — ADD (Algebraic Decision Diagram) operations
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use crate::computed_table::OpTag;
use crate::manager::Manager;
use crate::node::{DdNode, NodeId, CONST_INDEX};

/// ADD binary operators for use with `add_apply`.
#[derive(Clone, Copy, Debug)]
pub enum AddOp {
    Plus,
    Times,
    Minus,
    Divide,
    Minimum,
    Maximum,
    Or,
    And,
    Xor,
    Nand,
    Nor,
    Agree, // returns f if f == g, else ∞
}

impl AddOp {
    fn tag(self) -> u8 {
        self as u8
    }

    fn apply(self, a: f64, b: f64) -> f64 {
        match self {
            AddOp::Plus => a + b,
            AddOp::Times => a * b,
            AddOp::Minus => a - b,
            AddOp::Divide => {
                if b == 0.0 {
                    f64::INFINITY
                } else {
                    a / b
                }
            }
            AddOp::Minimum => a.min(b),
            AddOp::Maximum => a.max(b),
            AddOp::Or => {
                if a != 0.0 || b != 0.0 {
                    1.0
                } else {
                    0.0
                }
            }
            AddOp::And => {
                if a != 0.0 && b != 0.0 {
                    1.0
                } else {
                    0.0
                }
            }
            AddOp::Xor => {
                if (a != 0.0) ^ (b != 0.0) {
                    1.0
                } else {
                    0.0
                }
            }
            AddOp::Nand => {
                if a != 0.0 && b != 0.0 {
                    0.0
                } else {
                    1.0
                }
            }
            AddOp::Nor => {
                if a != 0.0 || b != 0.0 {
                    0.0
                } else {
                    1.0
                }
            }
            AddOp::Agree => {
                if (a - b).abs() < f64::EPSILON {
                    a
                } else {
                    f64::INFINITY
                }
            }
        }
    }
}

/// ADD monadic (unary) operators.
#[derive(Clone, Copy, Debug)]
pub enum AddMonadicOp {
    Log,
    Negate,
    Complement, // 1 - x
    Abs,
    Floor,
    Ceil,
}

impl AddMonadicOp {
    fn tag(self) -> u8 {
        self as u8
    }

    fn apply(self, a: f64) -> f64 {
        match self {
            AddMonadicOp::Log => a.ln(),
            AddMonadicOp::Negate => -a,
            AddMonadicOp::Complement => 1.0 - a,
            AddMonadicOp::Abs => a.abs(),
            AddMonadicOp::Floor => a.floor(),
            AddMonadicOp::Ceil => a.ceil(),
        }
    }
}

impl Manager {
    // ==================================================================
    // ADD Constants
    // ==================================================================

    /// Get or create an ADD constant (terminal) node with the given value.
    pub fn add_const(&mut self, value: f64) -> NodeId {
        if value == 1.0 {
            return NodeId::ONE;
        }

        if let Some(raw_idx) = self.constant_table.lookup(value) {
            return NodeId::from_raw(raw_idx, false);
        }

        let raw_idx = self.alloc_constant(value);
        self.constant_table.insert(value, raw_idx);
        NodeId::from_raw(raw_idx, false)
    }

    /// Get the ADD zero constant (0.0, distinct from BDD ZERO).
    pub fn add_zero(&mut self) -> NodeId {
        self.add_const(0.0)
    }

    /// Get the value of an ADD terminal node.
    ///
    /// ADDs do not use complemented edges. If a complemented NodeId is passed,
    /// this returns `None` to signal an invalid ADD reference.
    pub fn add_value(&self, id: NodeId) -> Option<f64> {
        if id.is_complemented() && id.raw_index() != 0 {
            // Complemented edges are only valid for BDD ONE/ZERO (raw index 0).
            // Any other complemented ADD node is an error.
            return None;
        }
        let node = self.node(id.regular());
        match node {
            DdNode::Constant { value, .. } => Some(*value),
            _ => None,
        }
    }

    // ==================================================================
    // ADD ITE
    // ==================================================================

    /// ADD If-Then-Else: selects g where f > 0, h otherwise.
    pub fn add_ite(&mut self, f: NodeId, g: NodeId, h: NodeId) -> NodeId {
        // Terminal: f is a constant
        if self.is_add_constant(f) {
            let fv = self.add_value(f).unwrap_or(0.0);
            return if fv != 0.0 { g } else { h };
        }
        if g == h {
            return g;
        }

        if let Some(result) = self.cache.lookup(OpTag::AddIte, f, g, h) {
            return result;
        }

        let f_level = self.level(f);
        let g_level = self.level(g);
        let h_level = self.level(h);
        let top_level = f_level.min(g_level).min(h_level);
        let top_var = self.inv_perm[top_level as usize] as u16;

        let (f_t, f_e) = self.add_cofactors(f, top_var);
        let (g_t, g_e) = self.add_cofactors(g, top_var);
        let (h_t, h_e) = self.add_cofactors(h, top_var);

        let t = self.add_ite(f_t, g_t, h_t);
        let e = self.add_ite(f_e, g_e, h_e);

        let result = if t == e {
            t
        } else {
            self.add_unique_inter(top_var, t, e)
        };

        self.cache.insert(OpTag::AddIte, f, g, h, result);
        result
    }

    /// ADD cofactors (no complemented edges for ADD).
    /// Get cofactors of an ADD node with respect to a variable.
    pub fn add_cofactors(&self, f: NodeId, var_index: u16) -> (NodeId, NodeId) {
        if f.is_constant() {
            return (f, f);
        }
        let node_var = self.var_index(f);
        if node_var == var_index {
            let node = self.node(f);
            (node.then_child(), node.else_child())
        } else {
            (f, f)
        }
    }

    /// ADD unique table lookup/insert (no complemented edges).
    pub(crate) fn add_unique_inter(&mut self, var_index: u16, then_child: NodeId, else_child: NodeId) -> NodeId {
        if then_child == else_child {
            return then_child;
        }

        let level = self.perm[var_index as usize] as usize;
        while self.unique_tables.len() <= level {
            self.unique_tables.push(crate::unique_table::UniqueSubtable::new());
        }

        if let Some(raw_idx) = self.unique_tables[level].lookup(then_child, else_child) {
            return NodeId::from_raw(raw_idx, false);
        }

        if self.nodes.len() >= self.gc_threshold {
            self.garbage_collect();
        }

        let raw_idx = self.alloc_node(var_index, then_child, else_child);
        self.unique_tables[level].insert(then_child, else_child, raw_idx);
        NodeId::from_raw(raw_idx, false)
    }

    // ==================================================================
    // ADD Apply (generic binary)
    // ==================================================================

    /// Apply a binary operator to two ADDs.
    pub fn add_apply(&mut self, op: AddOp, f: NodeId, g: NodeId) -> NodeId {
        // Terminal case: both operands are constants
        if self.is_add_constant(f) && self.is_add_constant(g) {
            let fv = self.add_value(f).unwrap_or(0.0);
            let gv = self.add_value(g).unwrap_or(0.0);
            return self.add_const(op.apply(fv, gv));
        }

        // Commutativity optimization for commutative ops
        let (a, b) = match op {
            AddOp::Plus | AddOp::Times | AddOp::Minimum | AddOp::Maximum
            | AddOp::Or | AddOp::And | AddOp::Xor | AddOp::Agree => {
                if f.raw_index() > g.raw_index() {
                    (g, f)
                } else {
                    (f, g)
                }
            }
            _ => (f, g),
        };

        let op_tag = OpTag::AddApply(op.tag());
        if let Some(result) = self.cache.lookup(op_tag, a, b, NodeId::ZERO) {
            return result;
        }

        let a_level = self.level(a);
        let b_level = self.level(b);
        let top_level = a_level.min(b_level);
        let top_var = self.inv_perm[top_level as usize] as u16;

        let (a_t, a_e) = self.add_cofactors(a, top_var);
        let (b_t, b_e) = self.add_cofactors(b, top_var);

        let t = self.add_apply(op, a_t, b_t);
        let e = self.add_apply(op, a_e, b_e);

        let result = if t == e { t } else { self.add_unique_inter(top_var, t, e) };

        self.cache.insert(op_tag, a, b, NodeId::ZERO, result);
        result
    }

    // ==================================================================
    // ADD Monadic Apply (unary)
    // ==================================================================

    /// Apply a unary operator to an ADD.
    pub fn add_monadic_apply(&mut self, op: AddMonadicOp, f: NodeId) -> NodeId {
        if self.is_add_constant(f) {
            let fv = self.add_value(f).unwrap_or(0.0);
            return self.add_const(op.apply(fv));
        }

        let op_tag = OpTag::AddMonadic(op.tag());
        if let Some(result) = self.cache.lookup(op_tag, f, NodeId::ZERO, NodeId::ZERO) {
            return result;
        }

        let f_var = self.var_index(f);
        let (f_t, f_e) = self.add_cofactors(f, f_var);

        let t = self.add_monadic_apply(op, f_t);
        let e = self.add_monadic_apply(op, f_e);

        let result = if t == e { t } else { self.add_unique_inter(f_var, t, e) };

        self.cache.insert(op_tag, f, NodeId::ZERO, NodeId::ZERO, result);
        result
    }

    // ==================================================================
    // ADD ↔ BDD conversion
    // ==================================================================

    /// Convert a BDD to an ADD (0/1 terminal valued).
    pub fn bdd_to_add(&mut self, f: NodeId) -> NodeId {
        if f.is_one() {
            return NodeId::ONE; // ADD 1.0
        }
        if f.is_zero() {
            return self.add_zero(); // ADD 0.0
        }

        let f_var = self.var_index(f.regular());
        let (f_t, f_e) = self.bdd_cofactors(f, f_var);

        let t = self.bdd_to_add(f_t);
        let e = self.bdd_to_add(f_e);

        if t == e { t } else { self.add_unique_inter(f_var, t, e) }
    }

    /// Convert an ADD to a BDD by thresholding: result is 1 where ADD value > threshold.
    pub fn add_bdd_threshold(&mut self, f: NodeId, threshold: f64) -> NodeId {
        if self.is_add_constant(f) {
            let v = self.add_value(f).unwrap_or(0.0);
            return if v > threshold { NodeId::ONE } else { NodeId::ZERO };
        }

        let f_var = self.var_index(f);
        let (f_t, f_e) = self.add_cofactors(f, f_var);

        let t = self.add_bdd_threshold(f_t, threshold);
        let e = self.add_bdd_threshold(f_e, threshold);

        if t == e { t } else { self.unique_inter(f_var, t, e) }
    }

    /// Convert an ADD to a BDD: result is 1 where ADD value != 0.
    pub fn add_bdd_pattern(&mut self, f: NodeId) -> NodeId {
        if self.is_add_constant(f) {
            let v = self.add_value(f).unwrap_or(0.0);
            return if v != 0.0 { NodeId::ONE } else { NodeId::ZERO };
        }

        let f_var = self.var_index(f);
        let (f_t, f_e) = self.add_cofactors(f, f_var);

        let t = self.add_bdd_pattern(f_t);
        let e = self.add_bdd_pattern(f_e);

        if t == e { t } else { self.unique_inter(f_var, t, e) }
    }

    // ==================================================================
    // Helpers
    // ==================================================================

    /// Check if a node is an ADD constant.
    fn is_add_constant(&self, id: NodeId) -> bool {
        let node = self.node(id.regular());
        node.var_index() == CONST_INDEX
    }

    // ==================================================================
    // Convenience arithmetic operations
    // ==================================================================

    pub fn add_plus(&mut self, f: NodeId, g: NodeId) -> NodeId {
        self.add_apply(AddOp::Plus, f, g)
    }

    pub fn add_times(&mut self, f: NodeId, g: NodeId) -> NodeId {
        self.add_apply(AddOp::Times, f, g)
    }

    pub fn add_minus(&mut self, f: NodeId, g: NodeId) -> NodeId {
        self.add_apply(AddOp::Minus, f, g)
    }

    pub fn add_divide(&mut self, f: NodeId, g: NodeId) -> NodeId {
        self.add_apply(AddOp::Divide, f, g)
    }

    pub fn add_min(&mut self, f: NodeId, g: NodeId) -> NodeId {
        self.add_apply(AddOp::Minimum, f, g)
    }

    pub fn add_max(&mut self, f: NodeId, g: NodeId) -> NodeId {
        self.add_apply(AddOp::Maximum, f, g)
    }

    pub fn add_negate(&mut self, f: NodeId) -> NodeId {
        self.add_monadic_apply(AddMonadicOp::Negate, f)
    }

    pub fn add_log(&mut self, f: NodeId) -> NodeId {
        self.add_monadic_apply(AddMonadicOp::Log, f)
    }

    /// ADD variable: creates an ADD node for the i-th variable
    /// (1.0 when variable is true, 0.0 when false).
    pub fn add_ith_var(&mut self, i: u16) -> NodeId {
        while self.num_vars <= i {
            self.bdd_new_var();
        }
        let add_one = NodeId::ONE;
        let add_zero = self.add_zero();
        self.add_unique_inter(i, add_one, add_zero)
    }
}
