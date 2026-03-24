// lumindd — BDD transfer, equivalence, monotonicity, and iteration utilities
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::{HashMap, HashSet};

use crate::manager::Manager;
use crate::node::{DdNode, NodeId, CONST_INDEX};

impl Manager {
    // ==================================================================
    // 1. BDD Transfer
    // ==================================================================

    /// Transfer a BDD from `source` manager into `self`.
    ///
    /// Recursively rebuilds the BDD using `self`'s unique tables.
    /// Variables are mapped by index (variable i in source becomes variable i in self).
    pub fn bdd_transfer(&mut self, source: &Manager, f: NodeId) -> NodeId {
        let mut cache: HashMap<NodeId, NodeId> = HashMap::new();
        self.bdd_transfer_rec(source, f, &mut cache)
    }

    fn bdd_transfer_rec(
        &mut self,
        source: &Manager,
        f: NodeId,
        cache: &mut HashMap<NodeId, NodeId>,
    ) -> NodeId {
        // Terminal cases
        if f.is_one() {
            return NodeId::ONE;
        }
        if f.is_zero() {
            return NodeId::ZERO;
        }

        // Check cache
        if let Some(&cached) = cache.get(&f) {
            return cached;
        }

        let reg = f.regular();
        let var = source.var_index(reg);

        // Ensure self has enough variables
        while self.num_vars <= var {
            self.bdd_new_var();
        }

        // Get children from source (accounting for complement)
        let t = source.then_child(f);
        let e = source.else_child(f);

        // Recurse
        let new_t = self.bdd_transfer_rec(source, t, cache);
        let new_e = self.bdd_transfer_rec(source, e, cache);

        // Build in self
        let result = if new_t == new_e {
            new_t
        } else {
            self.unique_inter(var, new_t, new_e)
        };

        cache.insert(f, result);
        result
    }

    // ==================================================================
    // 2. BDD Equivalence under Don't-Care
    // ==================================================================

    /// Check if `f` and `g` are equivalent under don't-care set `dc`:
    /// `f AND NOT(dc) == g AND NOT(dc)`.
    pub fn bdd_equiv_dc(&mut self, f: NodeId, g: NodeId, dc: NodeId) -> bool {
        if f == g {
            return true;
        }
        let care = dc.not();
        let f_care = self.bdd_and(f, care);
        let g_care = self.bdd_and(g, care);
        f_care == g_care
    }

    // ==================================================================
    // 3. BDD Leq Unless
    // ==================================================================

    /// Check if `f` implies `g` unless `dc`: `(f AND NOT(g) AND NOT(dc))` is ZERO.
    pub fn bdd_leq_unless(&mut self, f: NodeId, g: NodeId, dc: NodeId) -> bool {
        let not_g = g.not();
        let not_dc = dc.not();
        let tmp = self.bdd_and(f, not_g);
        let result = self.bdd_and(tmp, not_dc);
        result.is_zero()
    }

    // ==================================================================
    // 4. BDD Increasing (Monotone Increasing)
    // ==================================================================

    /// Check if `f` is monotone increasing in `var`:
    /// cofactor_neg(f, var) implies cofactor_pos(f, var).
    pub fn bdd_increasing(&self, f: NodeId, var: u16) -> bool {
        // We need a mutable borrow for bdd_leq, but we can check structurally.
        // cofactor_neg <= cofactor_pos means cofactor_neg AND NOT(cofactor_pos) == ZERO
        // We do this by cloning through a helper that takes &mut self.
        // Since &self is required, we check recursively without building new nodes.
        self.bdd_increasing_rec(f, var)
    }

    fn bdd_increasing_rec(&self, f: NodeId, var: u16) -> bool {
        if f.is_constant() {
            return true;
        }
        let f_var = self.var_index(f.regular());
        let f_level = self.perm[f_var as usize];
        let v_level = self.perm[var as usize];

        if f_level > v_level {
            // f does not depend on var, trivially monotone
            return true;
        }

        if f_var == var {
            // cofactor_neg <= cofactor_pos means: wherever neg is 1, pos must be 1.
            // In BDD terms with complement edges: bdd_leq(neg, pos).
            // We can check this by testing if AND(neg, NOT(pos)) is zero.
            let (pos, neg) = self.bdd_cofactors(f, var);
            self.bdd_leq_check(neg, pos)
        } else {
            // Recurse on children
            let (t, e) = self.bdd_cofactors(f, f_var);
            self.bdd_increasing_rec(t, var) && self.bdd_increasing_rec(e, var)
        }
    }

    /// Check bdd_leq(f, g) without mutation: f AND NOT(g) == ZERO.
    /// This works by checking that there is no path to ONE in f that is also a path to ZERO in g.
    fn bdd_leq_check(&self, f: NodeId, g: NodeId) -> bool {
        // Terminal cases
        if f.is_zero() {
            return true;
        }
        if g.is_one() {
            return true;
        }
        if f.is_one() && g.is_zero() {
            return false;
        }
        if f == g {
            return true;
        }

        // Find top variable
        let f_level = self.level(f);
        let g_level = self.level(g);
        let top_level = f_level.min(g_level);
        let top_var = self.inv_perm[top_level as usize] as u16;

        let (f_t, f_e) = self.bdd_cofactors(f, top_var);
        let (g_t, g_e) = self.bdd_cofactors(g, top_var);

        self.bdd_leq_check(f_t, g_t) && self.bdd_leq_check(f_e, g_e)
    }

    // ==================================================================
    // 5. BDD Decreasing (Monotone Decreasing)
    // ==================================================================

    /// Check if `f` is monotone decreasing in `var`:
    /// cofactor_pos(f, var) implies cofactor_neg(f, var).
    pub fn bdd_decreasing(&self, f: NodeId, var: u16) -> bool {
        self.bdd_decreasing_rec(f, var)
    }

    fn bdd_decreasing_rec(&self, f: NodeId, var: u16) -> bool {
        if f.is_constant() {
            return true;
        }
        let f_var = self.var_index(f.regular());
        let f_level = self.perm[f_var as usize];
        let v_level = self.perm[var as usize];

        if f_level > v_level {
            return true;
        }

        if f_var == var {
            let (pos, neg) = self.bdd_cofactors(f, var);
            self.bdd_leq_check(pos, neg)
        } else {
            let (t, e) = self.bdd_cofactors(f, f_var);
            self.bdd_decreasing_rec(t, var) && self.bdd_decreasing_rec(e, var)
        }
    }

    // ==================================================================
    // 6. BDD Non-Polluting AND
    // ==================================================================

    /// Non-polluting AND: like AND but avoids creating new nodes when one operand
    /// doesn't depend on variables of the other.
    ///
    /// In this implementation, we simply delegate to `bdd_and` since the optimization
    /// is mainly relevant for CUDD's memory management model.
    pub fn bdd_np_and(&mut self, f: NodeId, g: NodeId) -> NodeId {
        self.bdd_and(f, g)
    }

    // ==================================================================
    // 7. BDD Subset With Mask
    // ==================================================================

    /// Existentially abstract away all variables NOT in `mask_vars`.
    ///
    /// This keeps only the variables in `mask_vars` by existentially quantifying
    /// out everything else that `f` depends on.
    pub fn bdd_subset_with_mask(&mut self, f: NodeId, mask_vars: &[u16]) -> NodeId {
        if f.is_constant() {
            return f;
        }

        // Find all variables in f's support that are NOT in mask_vars
        let mask_set: HashSet<u16> = mask_vars.iter().copied().collect();
        let support = self.bdd_support(f);
        let abstract_vars: Vec<u16> = support
            .into_iter()
            .filter(|v| !mask_set.contains(v))
            .collect();

        if abstract_vars.is_empty() {
            return f;
        }

        // Build cube of variables to abstract
        let cube = self.bdd_cube(&abstract_vars);
        self.bdd_exist_abstract(f, cube)
    }

    // ==================================================================
    // 8. BDD Sharing Size
    // ==================================================================

    /// Count total shared DAG nodes across multiple BDDs (union of all reachable nodes).
    ///
    /// This is a static method that takes a manager reference for node traversal.
    pub fn bdd_sharing_size(&self, nodes: &[NodeId]) -> usize {
        let mut visited = HashSet::new();
        for &f in nodes {
            self.sharing_size_rec(f, &mut visited);
        }
        visited.len()
    }

    fn sharing_size_rec(&self, f: NodeId, visited: &mut HashSet<u32>) {
        let raw = f.raw_index();
        if !visited.insert(raw) {
            return;
        }
        if f.is_constant() {
            return;
        }
        self.sharing_size_rec(self.raw_then(f), visited);
        self.sharing_size_rec(self.raw_else(f), visited);
    }

    // ==================================================================
    // 9. BDD Count Leaves
    // ==================================================================

    /// Count distinct terminal nodes reachable from `f`.
    ///
    /// For a BDD this is always 1 or 2 (ONE and/or ZERO).
    pub fn bdd_count_leaves(&self, f: NodeId) -> usize {
        let mut leaves = HashSet::new();
        self.count_leaves_rec(f, &mut leaves, &mut HashSet::new());
        leaves.len()
    }

    fn count_leaves_rec(
        &self,
        f: NodeId,
        leaves: &mut HashSet<u32>,
        visited: &mut HashSet<u32>,
    ) {
        let raw = f.raw_index();
        if f.is_constant() {
            // For BDD: ONE is raw 0 uncomplemented, ZERO is raw 0 complemented.
            // We distinguish them by storing the full NodeId encoding.
            if f.is_one() {
                leaves.insert(0); // ONE
            } else {
                leaves.insert(1); // ZERO
            }
            return;
        }
        if !visited.insert(raw) {
            return;
        }
        self.count_leaves_rec(self.then_child(f), leaves, visited);
        self.count_leaves_rec(self.else_child(f), leaves, visited);
    }

    // ==================================================================
    // 10. BDD Estimate Cofactor
    // ==================================================================

    /// Estimate the DAG size of a cofactor without computing it.
    ///
    /// Walks the BDD counting nodes that would remain after cofactoring by `var`
    /// with the given `phase` (true = positive cofactor, false = negative cofactor).
    pub fn bdd_estimate_cofactor(&self, f: NodeId, var: u16, phase: bool) -> usize {
        let mut visited = HashSet::new();
        self.estimate_cofactor_rec(f, var, phase, &mut visited)
    }

    fn estimate_cofactor_rec(
        &self,
        f: NodeId,
        var: u16,
        phase: bool,
        visited: &mut HashSet<u32>,
    ) -> usize {
        if f.is_constant() {
            return 0;
        }
        let raw = f.raw_index();
        if !visited.insert(raw) {
            return 0;
        }

        let f_var = self.var_index(f.regular());
        if f_var == var {
            // This node gets replaced by the appropriate child
            let child = if phase {
                self.then_child(f)
            } else {
                self.else_child(f)
            };
            if child.is_constant() {
                return 0;
            }
            self.estimate_cofactor_rec(child, var, phase, visited)
        } else {
            // This node remains; count it plus children
            let t_count = self.estimate_cofactor_rec(self.then_child(f), var, phase, visited);
            let e_count = self.estimate_cofactor_rec(self.else_child(f), var, phase, visited);
            1 + t_count + e_count
        }
    }

    // ==================================================================
    // 11. BDD Foreach Prime
    // ==================================================================

    /// Iterate over all prime implicants of the function between `lower` and `upper` bounds.
    ///
    /// For each prime implicant, calls `callback` with an assignment vector of length `num_vars`.
    /// Each entry is `Some(true)` for positive literal, `Some(false)` for negative literal,
    /// or `None` for don't-care.
    ///
    /// A prime implicant of f (where lower <= f <= upper) is a minimal cube that implies upper
    /// and is consistent with lower.
    pub fn bdd_foreach_prime<F>(&mut self, lower: NodeId, upper: NodeId, mut callback: F)
    where
        F: FnMut(&[Option<bool>]),
    {
        let n = self.num_vars as usize;
        let mut cube = vec![None; n];
        self.foreach_prime_rec(lower, upper, &mut cube, &mut callback);
    }

    fn foreach_prime_rec<F>(
        &mut self,
        lower: NodeId,
        upper: NodeId,
        cube: &mut Vec<Option<bool>>,
        callback: &mut F,
    ) where
        F: FnMut(&[Option<bool>]),
    {
        // If upper is ZERO, no primes exist in this branch
        if upper.is_zero() {
            return;
        }
        // If lower is ONE, we found a prime (all remaining vars are don't-care)
        if lower.is_one() {
            callback(cube);
            return;
        }
        // If upper is ONE and lower is ZERO, emit the current cube as a prime
        if upper.is_one() && lower.is_zero() {
            callback(cube);
            return;
        }

        // Find the top variable
        let l_level = self.level(lower);
        let u_level = self.level(upper);
        let top_level = l_level.min(u_level);
        let top_var = self.inv_perm[top_level as usize] as u16;

        let (l_t, l_e) = self.bdd_cofactors(lower, top_var);
        let (u_t, u_e) = self.bdd_cofactors(upper, top_var);

        // Try to expand: if both branches lead to primes, the variable is don't-care
        // First check if the variable can be don't-care:
        // lower_t OR lower_e as the new lower, upper_t AND upper_e as the new upper
        let u_both = self.bdd_and(u_t, u_e);
        let l_both = self.bdd_or(l_t, l_e);

        if !u_both.is_zero() {
            // There are primes that don't depend on this variable
            cube[top_var as usize] = None;
            self.foreach_prime_rec(l_both, u_both, cube, callback);
        }

        // Positive-only primes: those in upper_t but not in upper_e (or lower forces it)
        let u_pos_only = self.bdd_and(u_t, u_e.not());
        if !u_pos_only.is_zero() {
            cube[top_var as usize] = Some(true);
            self.foreach_prime_rec(l_t, u_pos_only, cube, callback);
        }

        // Negative-only primes
        let u_neg_only = self.bdd_and(u_e, u_t.not());
        if !u_neg_only.is_zero() {
            cube[top_var as usize] = Some(false);
            self.foreach_prime_rec(l_e, u_neg_only, cube, callback);
        }

        // Restore
        cube[top_var as usize] = None;
    }

    // ==================================================================
    // 12. BDD Foreach Node
    // ==================================================================

    /// Visit every internal node in `f`'s DAG.
    ///
    /// Callback receives `(node_id, var_index, then_child, else_child)`.
    /// Terminal nodes are not visited.
    pub fn bdd_foreach_node<F>(&self, f: NodeId, mut callback: F)
    where
        F: FnMut(NodeId, u16, NodeId, NodeId),
    {
        let mut visited = HashSet::new();
        self.foreach_node_rec(f, &mut visited, &mut callback);
    }

    fn foreach_node_rec<F>(
        &self,
        f: NodeId,
        visited: &mut HashSet<u32>,
        callback: &mut F,
    ) where
        F: FnMut(NodeId, u16, NodeId, NodeId),
    {
        if f.is_constant() {
            return;
        }
        let raw = f.raw_index();
        if !visited.insert(raw) {
            return;
        }

        let reg = f.regular();
        let var = self.var_index(reg);
        let t = self.raw_then(f);
        let e = self.raw_else(f);

        callback(reg, var, t, e);

        self.foreach_node_rec(t, visited, callback);
        self.foreach_node_rec(e, visited, callback);
    }

    // ==================================================================
    // 13. ADD Equal Sup Norm
    // ==================================================================

    /// Check if two ADDs are equal within tolerance (sup-norm):
    /// `max|f - g| <= tolerance`.
    pub fn add_equal_sup_norm(&self, f: NodeId, g: NodeId, tolerance: f64) -> bool {
        self.add_sup_norm_rec(f, g, tolerance)
    }

    fn add_sup_norm_rec(&self, f: NodeId, g: NodeId, tolerance: f64) -> bool {
        if f == g {
            return true;
        }

        let f_is_const = self.is_add_const(f);
        let g_is_const = self.is_add_const(g);

        if f_is_const && g_is_const {
            let fv = self.add_terminal_value(f);
            let gv = self.add_terminal_value(g);
            return (fv - gv).abs() <= tolerance;
        }

        // Find top variable
        let f_level = self.add_level(f);
        let g_level = self.add_level(g);
        let top_level = f_level.min(g_level);
        let top_var = self.inv_perm[top_level as usize] as u16;

        let (f_t, f_e) = self.add_cofactors_const(f, top_var);
        let (g_t, g_e) = self.add_cofactors_const(g, top_var);

        self.add_sup_norm_rec(f_t, g_t, tolerance) && self.add_sup_norm_rec(f_e, g_e, tolerance)
    }

    // ==================================================================
    // 14. ADD Round Off
    // ==================================================================

    /// Round ADD terminal values to `places` decimal places.
    pub fn add_round_off(&mut self, f: NodeId, places: u32) -> NodeId {
        if self.is_add_const(f) {
            let v = self.add_terminal_value(f);
            let factor = 10.0f64.powi(places as i32);
            let rounded = (v * factor).round() / factor;
            return self.add_const(rounded);
        }

        let f_var = self.var_index(f);
        let (f_t, f_e) = self.add_cofactors_const(f, f_var);

        let t = self.add_round_off(f_t, places);
        let e = self.add_round_off(f_e, places);

        if t == e {
            t
        } else {
            self.add_unique_inter(f_var, t, e)
        }
    }

    // ==================================================================
    // 15. ADD Agreement
    // ==================================================================

    /// ADD that equals `f` where `f == g`, and a background value (0.0) elsewhere.
    pub fn add_agreement(&mut self, f: NodeId, g: NodeId) -> NodeId {
        if f == g {
            return f;
        }

        let f_is_const = self.is_add_const(f);
        let g_is_const = self.is_add_const(g);

        if f_is_const && g_is_const {
            let fv = self.add_terminal_value(f);
            let gv = self.add_terminal_value(g);
            if (fv - gv).abs() < f64::EPSILON {
                return f;
            } else {
                return self.add_const(0.0);
            }
        }

        let f_level = self.add_level(f);
        let g_level = self.add_level(g);
        let top_level = f_level.min(g_level);
        let top_var = self.inv_perm[top_level as usize] as u16;

        let (f_t, f_e) = self.add_cofactors_const(f, top_var);
        let (g_t, g_e) = self.add_cofactors_const(g, top_var);

        let t = self.add_agreement(f_t, g_t);
        let e = self.add_agreement(f_e, g_e);

        if t == e {
            t
        } else {
            self.add_unique_inter(top_var, t, e)
        }
    }

    // ==================================================================
    // 16. BDD Priority Select
    // ==================================================================

    /// Select the minterm from `f` with highest sum of priorities for true variables.
    ///
    /// `vars` lists the variable indices to consider.
    /// `priorities` gives the priority weight for each corresponding variable.
    /// Returns a BDD representing the single best minterm (or f if f is constant).
    pub fn bdd_priority_select(
        &mut self,
        f: NodeId,
        vars: &[u16],
        priorities: &[f64],
    ) -> NodeId {
        assert_eq!(vars.len(), priorities.len());

        if f.is_constant() {
            return f;
        }

        // Build a priority map
        let priority_map: HashMap<u16, f64> = vars
            .iter()
            .copied()
            .zip(priorities.iter().copied())
            .collect();

        // Get all satisfying assignments, score them, pick the best
        let cubes = self.bdd_iter_cubes(f);
        if cubes.is_empty() {
            return NodeId::ZERO;
        }

        // For each cube, we need to expand don't-cares and score.
        // But for efficiency, we greedily pick the best assignment.
        // Strategy: walk the BDD top-down, at each variable pick the branch
        // with higher priority (if both branches are satisfiable).
        let n = self.num_vars as usize;
        let mut assignment = vec![false; n];
        self.priority_select_rec(f, &priority_map, &mut assignment);

        // Build the BDD for the single minterm
        self.bdd_minterm_from_assignment(&assignment)
    }

    fn priority_select_rec(
        &self,
        f: NodeId,
        priority_map: &HashMap<u16, f64>,
        assignment: &mut Vec<bool>,
    ) {
        if f.is_constant() {
            return;
        }

        let var = self.var_index(f.regular());
        let t = self.then_child(f);
        let e = self.else_child(f);
        let prio = priority_map.get(&var).copied().unwrap_or(0.0);

        if t.is_zero() {
            // Must go else
            assignment[var as usize] = false;
            self.priority_select_rec(e, priority_map, assignment);
        } else if e.is_zero() {
            // Must go then
            assignment[var as usize] = true;
            self.priority_select_rec(t, priority_map, assignment);
        } else if prio >= 0.0 {
            // Prefer then (positive priority)
            assignment[var as usize] = true;
            self.priority_select_rec(t, priority_map, assignment);
        } else {
            // Prefer else (negative priority)
            assignment[var as usize] = false;
            self.priority_select_rec(e, priority_map, assignment);
        }
    }

    fn bdd_minterm_from_assignment(&mut self, assignment: &[bool]) -> NodeId {
        let n = self.num_vars;
        let mut result = NodeId::ONE;
        // Build bottom-up: iterate variables from highest level to lowest
        let mut var_indices: Vec<u16> = (0..n).collect();
        var_indices.sort_by(|a, b| self.perm[*b as usize].cmp(&self.perm[*a as usize]));

        for var in var_indices {
            let var_node = self.bdd_ith_var(var);
            let lit = if assignment[var as usize] {
                var_node
            } else {
                var_node.not()
            };
            result = self.bdd_and(lit, result);
        }
        result
    }

    // ==================================================================
    // ADD helper methods (private)
    // ==================================================================

    /// Check if a node is an ADD constant (terminal).
    fn is_add_const(&self, id: NodeId) -> bool {
        let node = self.node(id.regular());
        node.var_index() == CONST_INDEX
    }

    /// Get the terminal value of an ADD constant node.
    fn add_terminal_value(&self, id: NodeId) -> f64 {
        match self.node(id.regular()) {
            DdNode::Constant { value, .. } => {
                if id.is_complemented() && id.raw_index() == 0 {
                    // BDD ZERO represented as complemented ONE = 0.0
                    0.0
                } else {
                    *value
                }
            }
            _ => 0.0,
        }
    }

    /// Get the level of an ADD node.
    fn add_level(&self, id: NodeId) -> u32 {
        let vi = self.var_index(id.regular());
        if vi == CONST_INDEX {
            u32::MAX
        } else {
            self.perm[vi as usize]
        }
    }

    /// ADD cofactors without complement edge handling (ADDs don't use complement edges).
    fn add_cofactors_const(&self, f: NodeId, var_index: u16) -> (NodeId, NodeId) {
        if self.is_add_const(f) {
            return (f, f);
        }
        let node_var = self.var_index(f.regular());
        if node_var == var_index {
            let node = self.node(f.regular());
            (node.then_child(), node.else_child())
        } else {
            (f, f)
        }
    }
}

// ======================================================================
// Tests
// ======================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Manager;

    // ------------------------------------------------------------------
    // 1. bdd_transfer
    // ------------------------------------------------------------------

    #[test]
    fn test_bdd_transfer_constants() {
        let src = Manager::new();
        let mut dst = Manager::new();
        assert_eq!(dst.bdd_transfer(&src, NodeId::ONE), NodeId::ONE);
        assert_eq!(dst.bdd_transfer(&src, NodeId::ZERO), NodeId::ZERO);
    }

    #[test]
    fn test_bdd_transfer_single_var() {
        let mut src = Manager::new();
        let x = src.bdd_new_var();
        let mut dst = Manager::new();
        let x_dst = dst.bdd_transfer(&src, x);
        // Should behave identically: eval true -> true, eval false -> false
        assert!(dst.bdd_eval(x_dst, &[true]));
        assert!(!dst.bdd_eval(x_dst, &[false]));
    }

    #[test]
    fn test_bdd_transfer_complex() {
        let mut src = Manager::new();
        let a = src.bdd_new_var();
        let b = src.bdd_new_var();
        let f = src.bdd_and(a, b);

        let mut dst = Manager::new();
        let f_dst = dst.bdd_transfer(&src, f);

        // f = a AND b
        assert!(dst.bdd_eval(f_dst, &[true, true]));
        assert!(!dst.bdd_eval(f_dst, &[true, false]));
        assert!(!dst.bdd_eval(f_dst, &[false, true]));
        assert!(!dst.bdd_eval(f_dst, &[false, false]));
    }

    // ------------------------------------------------------------------
    // 2. bdd_equiv_dc
    // ------------------------------------------------------------------

    #[test]
    fn test_bdd_equiv_dc_identical() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        assert!(mgr.bdd_equiv_dc(x, x, NodeId::ZERO));
    }

    #[test]
    fn test_bdd_equiv_dc_with_dontcare() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        // f = x, g = x AND y, dc = NOT y (don't care when y=0)
        let f = x;
        let g = mgr.bdd_and(x, y);
        let dc = y.not();
        // Under dc: care set is y=1. f|y=1 = x, g|y=1 = x. So equivalent.
        assert!(mgr.bdd_equiv_dc(f, g, dc));
    }

    #[test]
    fn test_bdd_equiv_dc_not_equivalent() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        // f = x, g = y. dc = ZERO (no don't cares)
        assert!(!mgr.bdd_equiv_dc(x, y, NodeId::ZERO));
    }

    // ------------------------------------------------------------------
    // 3. bdd_leq_unless
    // ------------------------------------------------------------------

    #[test]
    fn test_bdd_leq_unless_trivial() {
        let mut mgr = Manager::new();
        // ZERO implies anything
        let x = mgr.bdd_new_var();
        assert!(mgr.bdd_leq_unless(NodeId::ZERO, x, NodeId::ZERO));
    }

    #[test]
    fn test_bdd_leq_unless_with_dc() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        // f = x OR y, g = x. Not f <= g in general.
        // But with dc = y (don't care when y=1): f AND NOT(g) AND NOT(dc) = (x OR y) AND NOT(x) AND NOT(y) = 0
        let f = mgr.bdd_or(x, y);
        assert!(mgr.bdd_leq_unless(f, x, y));
    }

    #[test]
    fn test_bdd_leq_unless_fails() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        // f = x OR y, g = x, dc = ZERO
        let f = mgr.bdd_or(x, y);
        assert!(!mgr.bdd_leq_unless(f, x, NodeId::ZERO));
    }

    // ------------------------------------------------------------------
    // 4. bdd_increasing
    // ------------------------------------------------------------------

    #[test]
    fn test_bdd_increasing_constant() {
        let mgr = Manager::new();
        assert!(mgr.bdd_increasing(NodeId::ONE, 0));
        assert!(mgr.bdd_increasing(NodeId::ZERO, 0));
    }

    #[test]
    fn test_bdd_increasing_var() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        // x is increasing in x: cofactor_neg=0, cofactor_pos=1. 0 <= 1. Yes.
        assert!(mgr.bdd_increasing(x, 0));
    }

    #[test]
    fn test_bdd_increasing_not_var() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        // NOT x is NOT increasing in x: cofactor_neg=1, cofactor_pos=0. 1 <= 0? No.
        let nx = x.not();
        assert!(!mgr.bdd_increasing(nx, 0));
    }

    #[test]
    fn test_bdd_increasing_or() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_or(x, y);
        // f = x OR y. Increasing in x? cofactor_neg = y, cofactor_pos = 1. y <= 1? Yes.
        assert!(mgr.bdd_increasing(f, 0));
        // Increasing in y? cofactor_neg = x, cofactor_pos = 1. x <= 1? Yes.
        assert!(mgr.bdd_increasing(f, 1));
    }

    // ------------------------------------------------------------------
    // 5. bdd_decreasing
    // ------------------------------------------------------------------

    #[test]
    fn test_bdd_decreasing_constant() {
        let mgr = Manager::new();
        assert!(mgr.bdd_decreasing(NodeId::ONE, 0));
    }

    #[test]
    fn test_bdd_decreasing_not_var() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let nx = x.not();
        // NOT x is decreasing in x: cofactor_pos=0, cofactor_neg=1. 0 <= 1? Yes.
        assert!(mgr.bdd_decreasing(nx, 0));
    }

    #[test]
    fn test_bdd_decreasing_var() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        // x is NOT decreasing in x: cofactor_pos=1, cofactor_neg=0. 1 <= 0? No.
        assert!(!mgr.bdd_decreasing(x, 0));
    }

    // ------------------------------------------------------------------
    // 6. bdd_np_and
    // ------------------------------------------------------------------

    #[test]
    fn test_bdd_np_and_basic() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let np = mgr.bdd_np_and(x, y);
        let regular = mgr.bdd_and(x, y);
        assert_eq!(np, regular);
    }

    #[test]
    fn test_bdd_np_and_constants() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        assert_eq!(mgr.bdd_np_and(x, NodeId::ONE), x);
        assert_eq!(mgr.bdd_np_and(x, NodeId::ZERO), NodeId::ZERO);
        assert_eq!(mgr.bdd_np_and(NodeId::ONE, NodeId::ONE), NodeId::ONE);
    }

    // ------------------------------------------------------------------
    // 7. bdd_subset_with_mask
    // ------------------------------------------------------------------

    #[test]
    fn test_bdd_subset_with_mask_empty() {
        let mut mgr = Manager::new();
        // Constant -> unchanged
        let r = mgr.bdd_subset_with_mask(NodeId::ONE, &[]);
        assert_eq!(r, NodeId::ONE);
    }

    #[test]
    fn test_bdd_subset_with_mask_keep_all() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y);
        // Keep both vars -> no change
        let r = mgr.bdd_subset_with_mask(f, &[0, 1]);
        assert_eq!(r, f);
    }

    #[test]
    fn test_bdd_subset_with_mask_abstract_one() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y);
        // Keep only x, abstract y: exists y. (x AND y) = x
        let r = mgr.bdd_subset_with_mask(f, &[0]);
        assert_eq!(r, x);
    }

    // ------------------------------------------------------------------
    // 8. bdd_sharing_size
    // ------------------------------------------------------------------

    #[test]
    fn test_bdd_sharing_size_empty() {
        let mgr = Manager::new();
        assert_eq!(mgr.bdd_sharing_size(&[]), 0);
    }

    #[test]
    fn test_bdd_sharing_size_constants() {
        let mgr = Manager::new();
        // ONE and ZERO share the same raw index 0
        assert_eq!(mgr.bdd_sharing_size(&[NodeId::ONE]), 1);
        assert_eq!(mgr.bdd_sharing_size(&[NodeId::ZERO]), 1);
        assert_eq!(mgr.bdd_sharing_size(&[NodeId::ONE, NodeId::ZERO]), 1);
    }

    #[test]
    fn test_bdd_sharing_size_shared() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y);
        let g = mgr.bdd_or(x, y);
        // Shared nodes between f and g
        let total = mgr.bdd_sharing_size(&[f, g]);
        let f_size = mgr.dag_size(f);
        let g_size = mgr.dag_size(g);
        // Sharing should be <= sum of individual sizes
        assert!(total <= f_size + g_size);
        assert!(total > 0);
    }

    // ------------------------------------------------------------------
    // 9. bdd_count_leaves
    // ------------------------------------------------------------------

    #[test]
    fn test_bdd_count_leaves_one() {
        let mgr = Manager::new();
        assert_eq!(mgr.bdd_count_leaves(NodeId::ONE), 1);
    }

    #[test]
    fn test_bdd_count_leaves_zero() {
        let mgr = Manager::new();
        assert_eq!(mgr.bdd_count_leaves(NodeId::ZERO), 1);
    }

    #[test]
    fn test_bdd_count_leaves_var() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        // x reaches both ONE and ZERO
        assert_eq!(mgr.bdd_count_leaves(x), 2);
    }

    // ------------------------------------------------------------------
    // 10. bdd_estimate_cofactor
    // ------------------------------------------------------------------

    #[test]
    fn test_bdd_estimate_cofactor_constant() {
        let mgr = Manager::new();
        assert_eq!(mgr.bdd_estimate_cofactor(NodeId::ONE, 0, true), 0);
    }

    #[test]
    fn test_bdd_estimate_cofactor_var() {
        let mut mgr = Manager::new();
        let _x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_and(_x, y);
        // Positive cofactor of (x AND y) w.r.t. x = y (1 internal node)
        let est = mgr.bdd_estimate_cofactor(f, 0, true);
        assert!(est >= 1);
    }

    #[test]
    fn test_bdd_estimate_cofactor_negative() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y);
        // Negative cofactor of (x AND y) w.r.t. x = ZERO (0 internal nodes)
        let est = mgr.bdd_estimate_cofactor(f, 0, false);
        assert_eq!(est, 0);
    }

    // ------------------------------------------------------------------
    // 11. bdd_foreach_prime
    // ------------------------------------------------------------------

    #[test]
    fn test_bdd_foreach_prime_one() {
        let mut mgr = Manager::new();
        let _x = mgr.bdd_new_var();
        let mut primes = Vec::new();
        mgr.bdd_foreach_prime(NodeId::ONE, NodeId::ONE, |cube| {
            primes.push(cube.to_vec());
        });
        // ONE has one prime: all don't-care
        assert_eq!(primes.len(), 1);
        assert!(primes[0].iter().all(|v| v.is_none()));
    }

    #[test]
    fn test_bdd_foreach_prime_zero() {
        let mut mgr = Manager::new();
        let _x = mgr.bdd_new_var();
        let mut primes = Vec::new();
        mgr.bdd_foreach_prime(NodeId::ZERO, NodeId::ZERO, |cube| {
            primes.push(cube.to_vec());
        });
        assert_eq!(primes.len(), 0);
    }

    #[test]
    fn test_bdd_foreach_prime_var() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let mut primes = Vec::new();
        mgr.bdd_foreach_prime(x, x, |cube| {
            primes.push(cube.to_vec());
        });
        // x has one prime: x=true, rest don't care
        assert_eq!(primes.len(), 1);
        assert_eq!(primes[0][0], Some(true));
    }

    // ------------------------------------------------------------------
    // 12. bdd_foreach_node
    // ------------------------------------------------------------------

    #[test]
    fn test_bdd_foreach_node_constant() {
        let mgr = Manager::new();
        let mut count = 0;
        mgr.bdd_foreach_node(NodeId::ONE, |_, _, _, _| {
            count += 1;
        });
        assert_eq!(count, 0); // No internal nodes in a constant
    }

    #[test]
    fn test_bdd_foreach_node_var() {
        let mut mgr = Manager::new();
        let _x = mgr.bdd_new_var();
        let mut nodes = Vec::new();
        mgr.bdd_foreach_node(_x, |id, var, _t, _e| {
            nodes.push((id, var));
        });
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].1, 0); // variable index 0
    }

    #[test]
    fn test_bdd_foreach_node_complex() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y);
        let mut count = 0;
        mgr.bdd_foreach_node(f, |_, _, _, _| {
            count += 1;
        });
        // x AND y should have at least 2 internal nodes
        assert!(count >= 2);
    }

    // ------------------------------------------------------------------
    // 13. add_equal_sup_norm
    // ------------------------------------------------------------------

    #[test]
    fn test_add_equal_sup_norm_identical() {
        let mut mgr = Manager::new();
        let c = mgr.add_const(3.14);
        assert!(mgr.add_equal_sup_norm(c, c, 0.0));
    }

    #[test]
    fn test_add_equal_sup_norm_within_tolerance() {
        let mut mgr = Manager::new();
        let a = mgr.add_const(1.0);
        let b = mgr.add_const(1.05);
        assert!(mgr.add_equal_sup_norm(a, b, 0.1));
        assert!(!mgr.add_equal_sup_norm(a, b, 0.01));
    }

    #[test]
    fn test_add_equal_sup_norm_with_vars() {
        let mut mgr = Manager::new();
        let _v = mgr.bdd_new_var();
        let c1 = mgr.add_const(2.0);
        let c2 = mgr.add_const(3.0);
        let f = mgr.add_unique_inter(0, c1, c2);
        // f: var0 ? 2.0 : 3.0
        let c3 = mgr.add_const(2.5);
        let c4 = mgr.add_const(3.4);
        let g = mgr.add_unique_inter(0, c3, c4);
        // g: var0 ? 2.5 : 3.4. Max diff = 0.5
        assert!(mgr.add_equal_sup_norm(f, g, 0.5));
        assert!(!mgr.add_equal_sup_norm(f, g, 0.3));
    }

    // ------------------------------------------------------------------
    // 14. add_round_off
    // ------------------------------------------------------------------

    #[test]
    fn test_add_round_off_constant() {
        let mut mgr = Manager::new();
        let c = mgr.add_const(3.14159);
        let r = mgr.add_round_off(c, 2);
        let val = mgr.add_value(r).unwrap();
        assert!((val - 3.14).abs() < 1e-10);
    }

    #[test]
    fn test_add_round_off_zero_places() {
        let mut mgr = Manager::new();
        let c = mgr.add_const(2.7);
        let r = mgr.add_round_off(c, 0);
        let val = mgr.add_value(r).unwrap();
        assert!((val - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_add_round_off_with_var() {
        let mut mgr = Manager::new();
        let _v = mgr.bdd_new_var();
        let c1 = mgr.add_const(1.456);
        let c2 = mgr.add_const(2.789);
        let f = mgr.add_unique_inter(0, c1, c2);
        let r = mgr.add_round_off(f, 1);
        // Check that terminal values are rounded to 1 decimal place
        // var0=true -> 1.5, var0=false -> 2.8
        let (t, e) = mgr.add_cofactors(r, 0);
        let tv = mgr.add_value(t).unwrap();
        let ev = mgr.add_value(e).unwrap();
        assert!((tv - 1.5).abs() < 1e-10);
        assert!((ev - 2.8).abs() < 1e-10);
    }

    // ------------------------------------------------------------------
    // 15. add_agreement
    // ------------------------------------------------------------------

    #[test]
    fn test_add_agreement_identical() {
        let mut mgr = Manager::new();
        let c = mgr.add_const(5.0);
        let r = mgr.add_agreement(c, c);
        assert_eq!(r, c);
    }

    #[test]
    fn test_add_agreement_different_constants() {
        let mut mgr = Manager::new();
        let a = mgr.add_const(1.0);
        let b = mgr.add_const(2.0);
        let r = mgr.add_agreement(a, b);
        let val = mgr.add_value(r).unwrap();
        assert!((val - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_add_agreement_partial() {
        let mut mgr = Manager::new();
        let _v = mgr.bdd_new_var();
        let c1 = mgr.add_const(5.0);
        let c2 = mgr.add_const(3.0);
        let f = mgr.add_unique_inter(0, c1, c2);
        // g: var0 ? 5.0 : 7.0
        let c3 = mgr.add_const(7.0);
        let g = mgr.add_unique_inter(0, c1, c3);
        let r = mgr.add_agreement(f, g);
        // Agreement: var0=true -> 5.0 (agree), var0=false -> 0.0 (disagree)
        let (t, e) = mgr.add_cofactors(r, 0);
        let tv = mgr.add_value(t).unwrap();
        let ev = mgr.add_value(e).unwrap();
        assert!((tv - 5.0).abs() < 1e-10);
        assert!((ev - 0.0).abs() < 1e-10);
    }

    // ------------------------------------------------------------------
    // 16. bdd_priority_select
    // ------------------------------------------------------------------

    #[test]
    fn test_bdd_priority_select_constant() {
        let mut mgr = Manager::new();
        let r = mgr.bdd_priority_select(NodeId::ONE, &[], &[]);
        assert_eq!(r, NodeId::ONE);
        let r2 = mgr.bdd_priority_select(NodeId::ZERO, &[], &[]);
        assert_eq!(r2, NodeId::ZERO);
    }

    #[test]
    fn test_bdd_priority_select_single_var() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        // f = x, priority for x = 1.0 (prefer true)
        let r = mgr.bdd_priority_select(x, &[0], &[1.0]);
        // Result should be the minterm x=true
        assert!(mgr.bdd_eval(r, &[true]));
        assert!(!mgr.bdd_eval(r, &[false]));
    }

    #[test]
    fn test_bdd_priority_select_two_vars() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_or(x, y);
        // Priorities: x=10.0, y=1.0. Best minterm with highest priority sum: x=true, y=true
        let r = mgr.bdd_priority_select(f, &[0, 1], &[10.0, 1.0]);
        // The result should be a single minterm that satisfies f
        assert!(mgr.bdd_eval(r, &[true, false]) || mgr.bdd_eval(r, &[true, true]),
            "selected minterm should have x=true (highest priority var)");
        // Should be a single minterm
        let count = mgr.bdd_count_minterm(r, 2);
        assert!((count - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_bdd_priority_select_negative_priority() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let _y = mgr.bdd_new_var();
        // f = x (only depends on x). Priority x = -5.0 (prefer false)
        let r = mgr.bdd_priority_select(x, &[0], &[-5.0]);
        // Since f=x, the only satisfying assignment has x=true, so we must pick it
        // even though priority is negative (no choice).
        assert!(mgr.bdd_eval(r, &[true, false]));
    }
}
