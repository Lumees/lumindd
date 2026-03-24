// lumindd — Additional ADD operations for CUDD parity
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::{HashMap, HashSet};

use crate::manager::Manager;
use crate::node::{NodeId, CONST_INDEX};

impl Manager {
    // ==================================================================
    // ADD Compose (single-variable substitution)
    // ==================================================================

    /// Single-variable substitution for ADDs: compute f[var := g].
    ///
    /// Replaces variable `var` in ADD `f` with ADD `g`. This is analogous
    /// to `bdd_compose` but operates on ADDs (no complemented edges).
    pub fn add_compose(&mut self, f: NodeId, g: NodeId, var: u16) -> NodeId {
        if self.is_add_terminal(f) {
            return f;
        }

        let f_var = self.var_index(f);
        let f_level = self.perm[f_var as usize];
        let v_level = self.perm[var as usize];

        if f_level > v_level {
            // f does not depend on var
            return f;
        }

        if f_var == var {
            // Direct substitution: ITE(g, f_then, f_else)
            let (f_t, f_e) = self.add_cofactors(f, var);
            return self.add_ite(g, f_t, f_e);
        }

        // Recurse on children
        let (f_t, f_e) = self.add_cofactors(f, f_var);
        let t = self.add_compose(f_t, g, var);
        let e = self.add_compose(f_e, g, var);

        if t == e { t } else { self.add_unique_inter(f_var, t, e) }
    }

    // ==================================================================
    // ADD Find Min / Max terminal values
    // ==================================================================

    /// Find the minimum terminal value in an ADD.
    ///
    /// Traverses all terminal nodes reachable from `f` and returns
    /// the smallest value found.
    pub fn add_find_min(&self, f: NodeId) -> f64 {
        let mut min_val = f64::INFINITY;
        let mut visited = HashSet::new();
        self.add_find_extremum_rec(f, &mut min_val, true, &mut visited);
        min_val
    }

    /// Find the maximum terminal value in an ADD.
    ///
    /// Traverses all terminal nodes reachable from `f` and returns
    /// the largest value found.
    pub fn add_find_max(&self, f: NodeId) -> f64 {
        let mut max_val = f64::NEG_INFINITY;
        let mut visited = HashSet::new();
        self.add_find_extremum_rec(f, &mut max_val, false, &mut visited);
        max_val
    }

    /// Recursive helper that finds min (is_min=true) or max (is_min=false).
    fn add_find_extremum_rec(
        &self,
        f: NodeId,
        extremum: &mut f64,
        is_min: bool,
        visited: &mut HashSet<u32>,
    ) {
        if self.is_add_terminal(f) {
            let v = self.add_value(f).unwrap_or(0.0);
            if is_min {
                if v < *extremum { *extremum = v; }
            } else {
                if v > *extremum { *extremum = v; }
            }
            return;
        }

        let raw = f.raw_index();
        if !visited.insert(raw) {
            return;
        }

        let node = self.node(f);
        let t = node.then_child();
        let e = node.else_child();
        self.add_find_extremum_rec(t, extremum, is_min, visited);
        self.add_find_extremum_rec(e, extremum, is_min, visited);
    }

    // ==================================================================
    // ADD Scalar Inverse
    // ==================================================================

    /// Compute 1/f for each terminal value in an ADD.
    ///
    /// Terminal values of 0.0 map to `f64::INFINITY`.
    pub fn add_scalar_inverse(&mut self, f: NodeId) -> NodeId {
        if self.is_add_terminal(f) {
            let v = self.add_value(f).unwrap_or(0.0);
            return if v == 0.0 {
                self.add_const(f64::INFINITY)
            } else {
                self.add_const(1.0 / v)
            };
        }

        let f_var = self.var_index(f);
        let (f_t, f_e) = self.add_cofactors(f, f_var);

        let t = self.add_scalar_inverse(f_t);
        let e = self.add_scalar_inverse(f_e);

        if t == e { t } else { self.add_unique_inter(f_var, t, e) }
    }

    // ==================================================================
    // ADD → BDD interval conversion
    // ==================================================================

    /// Convert ADD to BDD: result is 1 where lower <= ADD_value <= upper.
    ///
    /// For each path through the ADD, if the terminal value `v` satisfies
    /// `lower <= v <= upper`, the corresponding BDD path leads to ONE.
    pub fn add_bdd_interval(&mut self, f: NodeId, lower: f64, upper: f64) -> NodeId {
        if self.is_add_terminal(f) {
            let v = self.add_value(f).unwrap_or(0.0);
            return if v >= lower && v <= upper { NodeId::ONE } else { NodeId::ZERO };
        }

        let f_var = self.var_index(f);
        let (f_t, f_e) = self.add_cofactors(f, f_var);

        let t = self.add_bdd_interval(f_t, lower, upper);
        let e = self.add_bdd_interval(f_e, lower, upper);

        if t == e { t } else { self.unique_inter(f_var, t, e) }
    }

    // ==================================================================
    // ADD → BDD strict threshold
    // ==================================================================

    /// Convert ADD to BDD with strict threshold: result is 1 where ADD_value > threshold.
    ///
    /// This differs from `add_bdd_threshold` which uses `>=`.
    pub fn add_bdd_strict_threshold(&mut self, f: NodeId, threshold: f64) -> NodeId {
        if self.is_add_terminal(f) {
            let v = self.add_value(f).unwrap_or(0.0);
            return if v > threshold { NodeId::ONE } else { NodeId::ZERO };
        }

        let f_var = self.var_index(f);
        let (f_t, f_e) = self.add_cofactors(f, f_var);

        let t = self.add_bdd_strict_threshold(f_t, threshold);
        let e = self.add_bdd_strict_threshold(f_e, threshold);

        if t == e { t } else { self.unique_inter(f_var, t, e) }
    }

    // ==================================================================
    // ADD count paths to non-zero terminals
    // ==================================================================

    /// Count paths from root to non-zero terminals in an ADD.
    ///
    /// Each path through the ADD that reaches a terminal with value != 0.0
    /// contributes 1 to the count.
    pub fn add_count_paths_to_nonzero(&self, f: NodeId) -> f64 {
        let mut cache: HashMap<u32, f64> = HashMap::new();
        self.add_count_nonzero_rec(f, &mut cache)
    }

    fn add_count_nonzero_rec(&self, f: NodeId, cache: &mut HashMap<u32, f64>) -> f64 {
        if self.is_add_terminal(f) {
            let v = self.add_value(f).unwrap_or(0.0);
            return if v != 0.0 { 1.0 } else { 0.0 };
        }

        let raw = f.raw_index();
        if let Some(&cached) = cache.get(&raw) {
            return cached;
        }

        let node = self.node(f);
        let t = node.then_child();
        let e = node.else_child();
        let result = self.add_count_nonzero_rec(t, cache) + self.add_count_nonzero_rec(e, cache);
        cache.insert(raw, result);
        result
    }

    // ==================================================================
    // BDD vector support
    // ==================================================================

    /// Compute the union of support variables across multiple BDDs.
    ///
    /// Returns a sorted vector of variable indices that appear in at
    /// least one of the given BDD functions.
    pub fn bdd_vector_support(&self, funcs: &[NodeId]) -> Vec<u16> {
        let mut support = HashSet::new();
        for &f in funcs {
            for v in self.bdd_support(f) {
                support.insert(v);
            }
        }
        let mut result: Vec<u16> = support.into_iter().collect();
        result.sort();
        result
    }

    // ==================================================================
    // BDD classify support
    // ==================================================================

    /// Classify variables into three groups relative to two BDDs.
    ///
    /// Returns `(only_in_f, in_both, only_in_g)`:
    /// - `only_in_f`: variables that appear in `f` but not in `g`
    /// - `in_both`: variables that appear in both `f` and `g`
    /// - `only_in_g`: variables that appear in `g` but not in `f`
    pub fn bdd_classify_support(&self, f: NodeId, g: NodeId) -> (Vec<u16>, Vec<u16>, Vec<u16>) {
        let f_sup: HashSet<u16> = self.bdd_support(f).into_iter().collect();
        let g_sup: HashSet<u16> = self.bdd_support(g).into_iter().collect();

        let mut only_f: Vec<u16> = f_sup.difference(&g_sup).copied().collect();
        let mut both: Vec<u16> = f_sup.intersection(&g_sup).copied().collect();
        let mut only_g: Vec<u16> = g_sup.difference(&f_sup).copied().collect();

        only_f.sort();
        both.sort();
        only_g.sort();

        (only_f, both, only_g)
    }

    // ==================================================================
    // Private helpers
    // ==================================================================

    /// Check if a node is an ADD terminal (constant) node.
    fn is_add_terminal(&self, id: NodeId) -> bool {
        let node = self.node(id.regular());
        node.var_index() == CONST_INDEX
    }
}

// ==================================================================
// Tests
// ==================================================================

#[cfg(test)]
mod tests {
    use crate::manager::Manager;
    use crate::node::NodeId;

    #[test]
    fn test_add_compose() {
        let mut mgr = Manager::new();
        let _x0 = mgr.bdd_new_var(); // var 0
        let _x1 = mgr.bdd_new_var(); // var 1

        // Build ADD: if x0 then 3.0 else 1.0, directly via unique_inter
        let c3 = mgr.add_const(3.0);
        let c1 = NodeId::ONE; // 1.0
        let f = mgr.add_unique_inter(0, c3, c1);

        // g = ADD variable for x1
        let g = mgr.add_ith_var(1);

        // f[x0 := x1]: should give if x1 then 3.0 else 1.0
        let result = mgr.add_compose(f, g, 0);

        // The result should not depend on x0 anymore
        assert!(!result.is_constant());
    }

    #[test]
    fn test_add_find_min_max() {
        let mut mgr = Manager::new();
        let _x0 = mgr.bdd_new_var();

        // Build ADD: if x0 then 5.0 else 2.0
        let c5 = mgr.add_const(5.0);
        let c2 = mgr.add_const(2.0);
        let x0_add = mgr.add_ith_var(0);
        let f = mgr.add_ite(x0_add, c5, c2);

        assert_eq!(mgr.add_find_min(f), 2.0);
        assert_eq!(mgr.add_find_max(f), 5.0);
    }

    #[test]
    fn test_add_find_min_max_constant() {
        let mut mgr = Manager::new();
        let c42 = mgr.add_const(42.0);
        assert_eq!(mgr.add_find_min(c42), 42.0);
        assert_eq!(mgr.add_find_max(c42), 42.0);
    }

    #[test]
    fn test_add_scalar_inverse() {
        let mut mgr = Manager::new();
        let _x0 = mgr.bdd_new_var();

        // Build ADD: if x0 then 4.0 else 2.0
        let c4 = mgr.add_const(4.0);
        let c2 = mgr.add_const(2.0);
        let x0_add = mgr.add_ith_var(0);
        let f = mgr.add_ite(x0_add, c4, c2);

        let inv = mgr.add_scalar_inverse(f);

        // Result should be: if x0 then 0.25 else 0.5
        let min_val = mgr.add_find_min(inv);
        let max_val = mgr.add_find_max(inv);
        assert!((min_val - 0.25).abs() < 1e-10);
        assert!((max_val - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_add_scalar_inverse_zero() {
        let mut mgr = Manager::new();
        let c0 = mgr.add_const(0.0);
        let inv = mgr.add_scalar_inverse(c0);
        let v = mgr.add_value(inv).unwrap();
        assert!(v.is_infinite() && v > 0.0);
    }

    #[test]
    fn test_add_bdd_interval() {
        let mut mgr = Manager::new();
        let _x0 = mgr.bdd_new_var();

        // ADD: if x0 then 5.0 else 2.0
        let c5 = mgr.add_const(5.0);
        let c2 = mgr.add_const(2.0);
        let x0_add = mgr.add_ith_var(0);
        let f = mgr.add_ite(x0_add, c5, c2);

        // Interval [1.0, 3.0]: only x0=0 path (value 2.0) qualifies
        let bdd = mgr.add_bdd_interval(f, 1.0, 3.0);
        // x0=0 should be true, x0=1 should be false
        assert!(mgr.bdd_eval(bdd, &[false]));
        assert!(!mgr.bdd_eval(bdd, &[true]));

        // Interval [4.0, 6.0]: only x0=1 path (value 5.0) qualifies
        let bdd2 = mgr.add_bdd_interval(f, 4.0, 6.0);
        assert!(!mgr.bdd_eval(bdd2, &[false]));
        assert!(mgr.bdd_eval(bdd2, &[true]));
    }

    #[test]
    fn test_add_bdd_strict_threshold() {
        let mut mgr = Manager::new();
        let _x0 = mgr.bdd_new_var();

        // ADD: if x0 then 5.0 else 5.0 (constant 5)
        let c5 = mgr.add_const(5.0);

        // strict threshold 5.0: value must be > 5.0, so result is ZERO
        let bdd = mgr.add_bdd_strict_threshold(c5, 5.0);
        assert!(bdd.is_zero());

        // strict threshold 4.9: 5.0 > 4.9, so result is ONE
        let bdd2 = mgr.add_bdd_strict_threshold(c5, 4.9);
        assert!(bdd2.is_one());
    }

    #[test]
    fn test_add_count_paths_to_nonzero() {
        let mut mgr = Manager::new();
        let _x0 = mgr.bdd_new_var();

        // ADD: if x0 then 3.0 else 0.0
        let c3 = mgr.add_const(3.0);
        let c0 = mgr.add_const(0.0);
        let x0_add = mgr.add_ith_var(0);
        let f = mgr.add_ite(x0_add, c3, c0);

        // One path to non-zero (x0=1 -> 3.0), one path to zero (x0=0 -> 0.0)
        let count = mgr.add_count_paths_to_nonzero(f);
        assert!((count - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_add_count_paths_to_nonzero_all_nonzero() {
        let mut mgr = Manager::new();
        let _x0 = mgr.bdd_new_var();

        let c3 = mgr.add_const(3.0);
        let c7 = mgr.add_const(7.0);
        let x0_add = mgr.add_ith_var(0);
        let f = mgr.add_ite(x0_add, c7, c3);

        // Both paths lead to non-zero
        let count = mgr.add_count_paths_to_nonzero(f);
        assert!((count - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_bdd_vector_support() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var(); // var 0
        let x1 = mgr.bdd_new_var(); // var 1
        let x2 = mgr.bdd_new_var(); // var 2

        let f = mgr.bdd_and(x0, x1); // depends on 0, 1
        let g = mgr.bdd_and(x1, x2); // depends on 1, 2

        let support = mgr.bdd_vector_support(&[f, g]);
        assert_eq!(support, vec![0, 1, 2]);
    }

    #[test]
    fn test_bdd_vector_support_disjoint() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var();
        let _x1 = mgr.bdd_new_var();
        let x2 = mgr.bdd_new_var();

        // f depends only on x0, g depends only on x2
        let support = mgr.bdd_vector_support(&[x0, x2]);
        assert_eq!(support, vec![0, 2]);
    }

    #[test]
    fn test_bdd_classify_support() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var();
        let x1 = mgr.bdd_new_var();
        let x2 = mgr.bdd_new_var();

        let f = mgr.bdd_and(x0, x1); // depends on 0, 1
        let g = mgr.bdd_and(x1, x2); // depends on 1, 2

        let (only_f, both, only_g) = mgr.bdd_classify_support(f, g);
        assert_eq!(only_f, vec![0]);
        assert_eq!(both, vec![1]);
        assert_eq!(only_g, vec![2]);
    }

    #[test]
    fn test_bdd_classify_support_same() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var();
        let x1 = mgr.bdd_new_var();

        let f = mgr.bdd_and(x0, x1);
        let g = mgr.bdd_or(x0, x1);

        let (only_f, both, only_g) = mgr.bdd_classify_support(f, g);
        assert!(only_f.is_empty());
        assert_eq!(both, vec![0, 1]);
        assert!(only_g.is_empty());
    }
}
