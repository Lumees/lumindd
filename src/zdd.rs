// lumindd — ZDD (Zero-suppressed Decision Diagram) operations
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::HashMap;

use crate::computed_table::OpTag;
use crate::manager::Manager;
use crate::node::NodeId;

impl Manager {
    // ==================================================================
    // ZDD Set Operations
    // ==================================================================

    /// ZDD union (set union of families).
    pub fn zdd_union(&mut self, p: NodeId, q: NodeId) -> NodeId {
        // Terminal cases
        if p.is_zero() {
            return q;
        }
        if q.is_zero() {
            return p;
        }
        if p == q {
            return p;
        }
        if p.is_one() && q.is_one() {
            return NodeId::ONE;
        }

        // Normalize order for caching
        let (a, b) = if p.raw_index() > q.raw_index() {
            (q, p)
        } else {
            (p, q)
        };

        if let Some(result) = self.cache.lookup(OpTag::ZddUnion, a, b, NodeId::ZERO) {
            return result;
        }

        let a_level = self.zdd_level(a);
        let b_level = self.zdd_level(b);

        let result = if a_level < b_level {
            let a_var = self.var_index(a);
            let a_t = self.node(a).then_child();
            let a_e = self.node(a).else_child();
            let e = self.zdd_union(a_e, b);
            self.zdd_unique_inter(a_var, a_t, e)
        } else if a_level > b_level {
            let b_var = self.var_index(b);
            let b_t = self.node(b).then_child();
            let b_e = self.node(b).else_child();
            let e = self.zdd_union(a, b_e);
            self.zdd_unique_inter(b_var, b_t, e)
        } else {
            let a_var = self.var_index(a);
            let a_t = self.node(a).then_child();
            let a_e = self.node(a).else_child();
            let b_t = self.node(b).then_child();
            let b_e = self.node(b).else_child();
            let t = self.zdd_union(a_t, b_t);
            let e = self.zdd_union(a_e, b_e);
            self.zdd_unique_inter(a_var, t, e)
        };

        self.cache.insert(OpTag::ZddUnion, a, b, NodeId::ZERO, result);
        result
    }

    /// ZDD intersection (set intersection of families).
    pub fn zdd_intersect(&mut self, p: NodeId, q: NodeId) -> NodeId {
        if p.is_zero() || q.is_zero() {
            return NodeId::ZERO;
        }
        if p == q {
            return p;
        }
        if p.is_one() {
            return if q.is_one() { NodeId::ONE } else { NodeId::ZERO };
        }
        if q.is_one() {
            return NodeId::ZERO;
        }

        let (a, b) = if p.raw_index() > q.raw_index() {
            (q, p)
        } else {
            (p, q)
        };

        if let Some(result) = self.cache.lookup(OpTag::ZddIntersect, a, b, NodeId::ZERO) {
            return result;
        }

        let a_level = self.zdd_level(a);
        let b_level = self.zdd_level(b);

        let result = if a_level < b_level {
            let a_e = self.node(a).else_child();
            self.zdd_intersect(a_e, b)
        } else if a_level > b_level {
            let b_e = self.node(b).else_child();
            self.zdd_intersect(a, b_e)
        } else {
            let a_var = self.var_index(a);
            let a_t = self.node(a).then_child();
            let a_e = self.node(a).else_child();
            let b_t = self.node(b).then_child();
            let b_e = self.node(b).else_child();
            let t = self.zdd_intersect(a_t, b_t);
            let e = self.zdd_intersect(a_e, b_e);
            self.zdd_unique_inter(a_var, t, e)
        };

        self.cache.insert(OpTag::ZddIntersect, a, b, NodeId::ZERO, result);
        result
    }

    /// ZDD difference (set difference): P \ Q.
    pub fn zdd_diff(&mut self, p: NodeId, q: NodeId) -> NodeId {
        if p.is_zero() {
            return NodeId::ZERO;
        }
        if q.is_zero() {
            return p;
        }
        if p == q {
            return NodeId::ZERO;
        }

        if let Some(result) = self.cache.lookup(OpTag::ZddDiff, p, q, NodeId::ZERO) {
            return result;
        }

        let p_level = self.zdd_level(p);
        let q_level = self.zdd_level(q);

        let result = if p.is_one() {
            // p = {∅}. Check if q contains ∅ by following else-children to the bottom.
            // If q contains ∅, then {∅} \ q = ∅. Otherwise {∅} \ q = {∅}.
            if self.zdd_contains_empty(q) { NodeId::ZERO } else { NodeId::ONE }
        } else if p_level < q_level {
            let p_var = self.var_index(p);
            let p_t = self.node(p).then_child();
            let p_e = self.node(p).else_child();
            let e = self.zdd_diff(p_e, q);
            self.zdd_unique_inter(p_var, p_t, e)
        } else if p_level > q_level {
            let q_e = self.node(q).else_child();
            self.zdd_diff(p, q_e)
        } else {
            let p_var = self.var_index(p);
            let p_t = self.node(p).then_child();
            let p_e = self.node(p).else_child();
            let q_t = self.node(q).then_child();
            let q_e = self.node(q).else_child();
            let t = self.zdd_diff(p_t, q_t);
            let e = self.zdd_diff(p_e, q_e);
            self.zdd_unique_inter(p_var, t, e)
        };

        self.cache.insert(OpTag::ZddDiff, p, q, NodeId::ZERO, result);
        result
    }

    /// ZDD product (cross product of set families).
    pub fn zdd_product(&mut self, f: NodeId, g: NodeId) -> NodeId {
        if f.is_zero() || g.is_zero() {
            return NodeId::ZERO;
        }
        if f.is_one() {
            return g;
        }
        if g.is_one() {
            return f;
        }

        let (a, b) = if f.raw_index() > g.raw_index() {
            (g, f)
        } else {
            (f, g)
        };

        if let Some(result) = self.cache.lookup(OpTag::ZddProduct, a, b, NodeId::ZERO) {
            return result;
        }

        let a_level = self.zdd_level(a);
        let b_level = self.zdd_level(b);

        let result = if a_level < b_level {
            let a_var = self.var_index(a);
            let a_t = self.node(a).then_child();
            let a_e = self.node(a).else_child();
            let t = self.zdd_product(a_t, b);
            let e = self.zdd_product(a_e, b);
            self.zdd_unique_inter(a_var, t, e)
        } else if a_level > b_level {
            let b_var = self.var_index(b);
            let b_t = self.node(b).then_child();
            let b_e = self.node(b).else_child();
            let t = self.zdd_product(a, b_t);
            let e = self.zdd_product(a, b_e);
            self.zdd_unique_inter(b_var, t, e)
        } else {
            let a_var = self.var_index(a);
            let a_t = self.node(a).then_child();
            let a_e = self.node(a).else_child();
            let b_t = self.node(b).then_child();
            let b_e = self.node(b).else_child();
            // (a_t ∪ a_e) × (b_t ∪ b_e), but with same top variable
            let tt = self.zdd_product(a_t, b_t);
            let te = self.zdd_product(a_t, b_e);
            let et = self.zdd_product(a_e, b_t);
            let ee = self.zdd_product(a_e, b_e);
            let te_et = self.zdd_union(te, et);
            let t_part = self.zdd_union(tt, te_et);
            self.zdd_unique_inter(a_var, t_part, ee)
        };

        self.cache.insert(OpTag::ZddProduct, a, b, NodeId::ZERO, result);
        result
    }

    /// ZDD weak division: f / g.
    pub fn zdd_weak_div(&mut self, f: NodeId, g: NodeId) -> NodeId {
        if f.is_zero() {
            return NodeId::ZERO;
        }
        if g.is_zero() {
            return NodeId::ZERO;
        }
        if g.is_one() {
            return f;
        }
        if f == g {
            return NodeId::ONE;
        }

        if let Some(result) = self.cache.lookup(OpTag::ZddWeakDiv, f, g, NodeId::ZERO) {
            return result;
        }

        let f_level = self.zdd_level(f);
        let g_level = self.zdd_level(g);

        let result = if f_level < g_level {
            let f_e = self.node(f).else_child();
            self.zdd_weak_div(f_e, g)
        } else if f_level > g_level {
            NodeId::ZERO
        } else {
            // Same variable level
            let f_t = self.node(f).then_child();
            let f_e = self.node(f).else_child();
            let g_t = self.node(g).then_child();
            let g_e = self.node(g).else_child();

            if g_e.is_zero() {
                // g only has sets containing this var → divide f_t by g_t
                self.zdd_weak_div(f_t, g_t)
            } else if g_t.is_zero() {
                // g only has sets without this var → divide f_e by g_e
                self.zdd_weak_div(f_e, g_e)
            } else {
                let t = self.zdd_weak_div(f_t, g_t);
                let e = self.zdd_weak_div(f_e, g_e);
                self.zdd_intersect(t, e)
            }
        };

        self.cache.insert(OpTag::ZddWeakDiv, f, g, NodeId::ZERO, result);
        result
    }

    /// ZDD change: toggle variable membership in all sets.
    pub fn zdd_change(&mut self, p: NodeId, var: u16) -> NodeId {
        if p.is_zero() {
            return NodeId::ZERO;
        }

        let var_id = NodeId::from_raw(var as u32, false);
        if let Some(result) = self.cache.lookup(OpTag::ZddChange, p, var_id, NodeId::ZERO) {
            return result;
        }

        let result = if p.is_one() {
            // {∅} with var toggled = {{var}}
            self.zdd_unique_inter(var, NodeId::ONE, NodeId::ZERO)
        } else {
            let p_var = self.var_index(p);
            let p_level = self.zdd_perm[p_var as usize];
            let var_level = self.zdd_perm[var as usize];

            if p_level < var_level {
                let p_t = self.node(p).then_child();
                let p_e = self.node(p).else_child();
                let t = self.zdd_change(p_t, var);
                let e = self.zdd_change(p_e, var);
                self.zdd_unique_inter(p_var, t, e)
            } else if p_level > var_level {
                // Insert var above p
                self.zdd_unique_inter(var, p, NodeId::ZERO)
            } else {
                // Same variable — swap then and else children
                let p_t = self.node(p).then_child();
                let p_e = self.node(p).else_child();
                self.zdd_unique_inter(p_var, p_e, p_t)
            }
        };

        self.cache.insert(OpTag::ZddChange, p, var_id, NodeId::ZERO, result);
        result
    }

    /// ZDD subset1: positive cofactor (sets that contain var).
    pub fn zdd_subset1(&self, p: NodeId, var: u16) -> NodeId {
        if p.is_zero() || p.is_one() {
            return NodeId::ZERO;
        }
        let p_var = self.var_index(p);
        let p_level = self.zdd_perm[p_var as usize];
        let var_level = self.zdd_perm[var as usize];

        if p_level > var_level {
            NodeId::ZERO
        } else if p_level < var_level {
            // var is below p, recurse into else-child
            let p_e = self.node(p).else_child();
            self.zdd_subset1(p_e, var)
        } else {
            self.node(p).then_child()
        }
    }

    /// ZDD subset0: negative cofactor (sets that don't contain var).
    pub fn zdd_subset0(&self, p: NodeId, var: u16) -> NodeId {
        if p.is_zero() || p.is_one() {
            return p;
        }
        let p_var = self.var_index(p);
        let p_level = self.zdd_perm[p_var as usize];
        let var_level = self.zdd_perm[var as usize];

        if p_level > var_level {
            p
        } else if p_level < var_level {
            let p_e = self.node(p).else_child();
            self.zdd_subset0(p_e, var)
        } else {
            self.node(p).else_child()
        }
    }

    /// ZDD ITE.
    pub fn zdd_ite(&mut self, f: NodeId, g: NodeId, h: NodeId) -> NodeId {
        if f.is_one() {
            return g;
        }
        if f.is_zero() {
            return h;
        }
        if g == h {
            return g;
        }

        if let Some(result) = self.cache.lookup(OpTag::ZddIte, f, g, h) {
            return result;
        }

        let f_level = self.zdd_level(f);
        let g_level = self.zdd_level(g);
        let h_level = self.zdd_level(h);
        let top_level = f_level.min(g_level).min(h_level);
        let top_var = self.zdd_inv_perm[top_level as usize] as u16;

        let (f_t, f_e) = self.zdd_cofactors(f, top_var);
        let (g_t, g_e) = self.zdd_cofactors(g, top_var);
        let (h_t, h_e) = self.zdd_cofactors(h, top_var);

        let t = self.zdd_ite(f_t, g_t, h_t);
        let e = self.zdd_ite(f_e, g_e, h_e);

        let result = self.zdd_unique_inter(top_var, t, e);

        self.cache.insert(OpTag::ZddIte, f, g, h, result);
        result
    }

    /// ZDD cofactors with respect to a variable.
    pub(crate) fn zdd_cofactors(&self, f: NodeId, var_index: u16) -> (NodeId, NodeId) {
        if f.is_constant() {
            return (NodeId::ZERO, f);
        }
        let node_var = self.var_index(f);
        if node_var == var_index {
            (self.node(f).then_child(), self.node(f).else_child())
        } else {
            (NodeId::ZERO, f)
        }
    }

    // ==================================================================
    // ZDD ↔ BDD conversion
    // ==================================================================

    /// Convert a BDD to a ZDD.
    pub fn zdd_from_bdd(&mut self, f: NodeId) -> NodeId {
        if f.is_one() {
            return NodeId::ONE;
        }
        if f.is_zero() {
            return NodeId::ZERO;
        }

        let f_var = self.var_index(f.regular());
        let (f_t, f_e) = self.bdd_cofactors(f, f_var);

        let t = self.zdd_from_bdd(f_t);
        let e = self.zdd_from_bdd(f_e);

        // Ensure ZDD variable exists
        while self.num_zdd_vars <= f_var {
            self.zdd_new_var();
        }

        self.zdd_unique_inter(f_var, t, e)
    }

    /// Convert a ZDD to a BDD.
    pub fn zdd_to_bdd(&mut self, f: NodeId) -> NodeId {
        if f.is_one() {
            return NodeId::ONE;
        }
        if f.is_zero() {
            return NodeId::ZERO;
        }

        let f_var = self.var_index(f);
        let f_t = self.node(f).then_child();
        let f_e = self.node(f).else_child();

        let t = self.zdd_to_bdd(f_t);
        let e = self.zdd_to_bdd(f_e);

        // Ensure BDD variable exists
        while self.num_vars <= f_var {
            self.bdd_new_var();
        }

        self.unique_inter(f_var, t, e)
    }

    /// Count the number of sets in a ZDD family.
    pub fn zdd_count(&self, f: NodeId) -> u64 {
        let mut cache: HashMap<u32, u64> = HashMap::new();
        self.zdd_count_rec(f, &mut cache)
    }

    fn zdd_count_rec(&self, f: NodeId, cache: &mut HashMap<u32, u64>) -> u64 {
        if f.is_zero() {
            return 0;
        }
        if f.is_one() {
            return 1;
        }
        let key = f.raw_index();
        if let Some(&cached) = cache.get(&key) {
            return cached;
        }
        let t = self.node(f).then_child();
        let e = self.node(f).else_child();
        let result = self.zdd_count_rec(t, cache) + self.zdd_count_rec(e, cache);
        cache.insert(key, result);
        result
    }

    /// Check if a ZDD family contains the empty set.
    /// Follows the else-children chain to the terminal.
    fn zdd_contains_empty(&self, f: NodeId) -> bool {
        let mut current = f;
        loop {
            if current.is_one() {
                return true;
            }
            if current.is_zero() {
                return false;
            }
            current = self.node(current).else_child();
        }
    }
}
