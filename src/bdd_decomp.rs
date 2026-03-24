// lumindd — BDD decomposition and equation solving
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::HashSet;

use crate::manager::Manager;
use crate::node::NodeId;

impl Manager {
    // ==================================================================
    // Support computation helper
    // ==================================================================

    /// Collect the set of variable indices that appear in the BDD rooted at `f`.
    fn bdd_support_set(&self, f: NodeId) -> HashSet<u16> {
        let mut support = HashSet::new();
        let mut visited = HashSet::new();
        self.bdd_support_rec(f, &mut support, &mut visited);
        support
    }

    fn bdd_support_rec(
        &self,
        f: NodeId,
        support: &mut HashSet<u16>,
        visited: &mut HashSet<u32>,
    ) {
        if f.is_constant() {
            return;
        }
        let raw = f.raw_index();
        if !visited.insert(raw) {
            return;
        }
        let var = self.var_index(f.regular());
        support.insert(var);
        let (t, e) = self.bdd_cofactors(f, var);
        self.bdd_support_rec(t, support, visited);
        self.bdd_support_rec(e, support, visited);
    }

    /// Count nodes in a BDD (for decomposition heuristics).
    fn bdd_size(&self, f: NodeId) -> usize {
        let mut visited = HashSet::new();
        self.bdd_size_rec(f, &mut visited);
        visited.len()
    }

    fn bdd_size_rec(&self, f: NodeId, visited: &mut HashSet<u32>) {
        if f.is_constant() {
            return;
        }
        if !visited.insert(f.raw_index()) {
            return;
        }
        let var = self.var_index(f.regular());
        let (t, e) = self.bdd_cofactors(f, var);
        self.bdd_size_rec(t, visited);
        self.bdd_size_rec(e, visited);
    }

    // ==================================================================
    // Conjunctive decomposition
    // ==================================================================

    /// Decompose `f` into `g AND h` where neither `g` nor `h` is trivial
    /// (i.e., neither is constant ONE), attempting to minimize `max(|g|, |h|)`.
    ///
    /// If `f` cannot be decomposed (it is a single variable or constant),
    /// returns `(f, ONE)`.
    ///
    /// The approach: for each variable at the top levels, check if one cofactor
    /// implies the other. If `f|_v=0` implies `f|_v=1`, then
    /// `f = (v OR f|_v=0) AND f|_v=1`.
    pub fn bdd_conjunctive_decomp(&mut self, f: NodeId) -> (NodeId, NodeId) {
        if f.is_constant() {
            return (f, NodeId::ONE);
        }

        let support = self.bdd_support_set(f);
        if support.len() <= 1 {
            return (f, NodeId::ONE);
        }

        // Sort variables by level (top first)
        let mut vars: Vec<u16> = support.into_iter().collect();
        vars.sort_by_key(|&v| self.perm[v as usize]);

        let f_size = self.bdd_size(f);
        let mut best_g = f;
        let mut best_h = NodeId::ONE;
        let mut best_max = f_size;

        // Try decomposition at each of the top variables
        let limit = vars.len().min(8); // limit search depth
        for i in 0..limit {
            let v = vars[i];
            let (f_t, f_e) = self.bdd_cofactors(f, v);

            // Check if f_e implies f_t: then f = (v OR f_e) AND f_t
            let check_e_implies_t = self.bdd_and(f_e, f_t.not());
            if check_e_implies_t.is_zero() {
                let v_node = self.bdd_ith_var(v);
                let g = self.bdd_or(v_node, f_e);
                let h = f_t;

                if !g.is_one() && !h.is_one() {
                    let g_size = self.bdd_size(g);
                    let h_size = self.bdd_size(h);
                    let max_size = g_size.max(h_size);
                    if max_size < best_max {
                        best_g = g;
                        best_h = h;
                        best_max = max_size;
                    }
                }
            }

            // Check if f_t implies f_e: then f = (NOT v OR f_t) AND f_e
            let check_t_implies_e = self.bdd_and(f_t, f_e.not());
            if check_t_implies_e.is_zero() {
                let v_node = self.bdd_ith_var(v);
                let g = self.bdd_or(v_node.not(), f_t);
                let h = f_e;

                if !g.is_one() && !h.is_one() {
                    let g_size = self.bdd_size(g);
                    let h_size = self.bdd_size(h);
                    let max_size = g_size.max(h_size);
                    if max_size < best_max {
                        best_g = g;
                        best_h = h;
                        best_max = max_size;
                    }
                }
            }

            // General decomposition: split support
            // g = exist_abstract(f, cube of vars not containing v)
            // This is expensive so we only try for the top variable
            if i == 0 && best_h.is_one() {
                // Try splitting the support in half
                let mid = vars.len() / 2;
                if mid > 0 && mid < vars.len() {
                    let cube_vars: Vec<u16> = vars[mid..].to_vec();
                    let cube = self.bdd_cube(&cube_vars);
                    let g = self.bdd_exist_abstract(f, cube);

                    if !g.is_one() && !g.is_zero() {
                        // h = constrain(f, g)
                        let h = self.bdd_constrain(f, g);
                        // Verify: g AND h should equal f
                        let prod = self.bdd_and(g, h);
                        if prod == f {
                            let g_size = self.bdd_size(g);
                            let h_size = self.bdd_size(h);
                            let max_size = g_size.max(h_size);
                            if max_size < best_max {
                                best_g = g;
                                best_h = h;
                                best_max = max_size;
                            }
                        }
                    }
                }
            }
        }

        (best_g, best_h)
    }

    // ==================================================================
    // Disjunctive decomposition
    // ==================================================================

    /// Decompose `f` into `g OR h` where neither is trivial.
    ///
    /// Uses duality: `f = g OR h` iff `NOT f = (NOT g) AND (NOT h)`.
    pub fn bdd_disjunctive_decomp(&mut self, f: NodeId) -> (NodeId, NodeId) {
        let nf = f.not();
        let (ng, nh) = self.bdd_conjunctive_decomp(nf);
        (ng.not(), nh.not())
    }

    // ==================================================================
    // Iterative conjunctive decomposition
    // ==================================================================

    /// Iteratively decompose `f` into a conjunction of up to `max_parts` parts.
    ///
    /// Returns a vector `[g1, g2, ..., gk]` such that `f = g1 AND g2 AND ... AND gk`
    /// where `k <= max_parts`.
    pub fn bdd_iterative_conjunctive_decomp(
        &mut self,
        f: NodeId,
        max_parts: usize,
    ) -> Vec<NodeId> {
        if f.is_constant() || max_parts <= 1 {
            return vec![f];
        }

        let mut parts: Vec<NodeId> = Vec::new();
        let mut remaining = f;

        while parts.len() < max_parts - 1 {
            let (g, h) = self.bdd_conjunctive_decomp(remaining);

            if h.is_one() {
                // Cannot decompose further
                break;
            }

            parts.push(g);
            remaining = h;
        }

        parts.push(remaining);
        parts
    }

    // ==================================================================
    // Equation solving
    // ==================================================================

    /// Solve `f(x, var) = 0` for `var`.
    ///
    /// Returns `(particular_solution, care_set)` where:
    /// - `particular_solution`: a BDD `g` such that `f[var := g]` is satisfiable
    /// - `care_set`: the set of assignments to the remaining variables where a
    ///   solution exists (the projection of f's solution set onto non-var variables)
    ///
    /// The general solution for var is: `(particular AND care) OR (anything AND NOT care)`.
    pub fn bdd_solve_eqn(
        &mut self,
        f: NodeId,
        var: u16,
    ) -> (NodeId, NodeId) {
        // f(var=1) and f(var=0) are the positive and negative cofactors
        let (f_pos, f_neg) = self.bdd_cofactors(f, var);

        // For f to be satisfiable, we need f_pos=1 OR f_neg=1 for each assignment.
        // The care set is where at least one cofactor is zero (solution exists).
        // Actually: care = NOT(f_pos AND f_neg) = at least one cofactor is 0 (f=0 achievable)
        //
        // Wait -- we want f=0. f(var) = 0 means: if var=1 then f_pos=0, if var=0 then f_neg=0.
        // We need to find var = g(other_vars) such that f[var:=g] = 0.
        //
        // f[var:=g] = ITE(g, f_pos, f_neg) = 0
        // This means: where g=1, f_pos must be 0; where g=0, f_neg must be 0.
        // So g must imply NOT(f_pos), and NOT(g) must imply NOT(f_neg).
        // g <= NOT(f_pos) and (NOT g) <= NOT(f_neg)
        // g <= NOT(f_pos) and f_neg <= g
        // So f_neg <= g <= NOT(f_pos)
        //
        // This is solvable iff f_neg <= NOT(f_pos), i.e., f_neg AND f_pos = 0.
        // The care set = NOT(f_neg AND f_pos) -- where the equation is solvable.
        // Actually the care set is the set of assignments where solution exists:
        // = NOT(f_pos) OR NOT(f_neg) = NOT(f_pos AND f_neg)

        let care = self.bdd_nand(f_pos, f_neg);

        // Particular solution: pick g = NOT(f_pos) on the care set.
        // Where f_pos = 0, set var=1 (g=1) works since f_pos=0 means f=0 when var=1.
        // Where f_pos = 1, we need var=0 (g=0), and f_neg must be 0 there.
        let particular = f_pos.not();

        // Restrict to care set for a cleaner particular solution
        let particular = self.bdd_and(particular, care);

        (particular, care)
    }

    /// Verify that solutions satisfy the equation `f = 0`.
    ///
    /// For each variable `vars[i]`, substitute `solutions[i]` into `f`.
    /// Returns true if the result is the zero function (on the care set).
    pub fn bdd_verify_sol(
        &mut self,
        f: NodeId,
        vars: &[u16],
        solutions: &[NodeId],
    ) -> bool {
        assert_eq!(
            vars.len(),
            solutions.len(),
            "vars and solutions must have the same length"
        );

        let mut result = f;
        for i in 0..vars.len() {
            result = self.bdd_compose(result, solutions[i], vars[i]);
        }

        result.is_zero()
    }

    // ==================================================================
    // Essential variables
    // ==================================================================

    /// Find variables that appear on every path from root to ONE.
    ///
    /// A variable is "essential" if both its positive and negative cofactors
    /// are strictly smaller than the original function (in terms of the
    /// satisfying set). Formally, variable `v` is essential in `f` if
    /// `f|_{v=0}` and `f|_{v=1}` are both strictly less than `f`.
    ///
    /// Equivalently, v is essential iff f depends on v and neither cofactor
    /// equals f (the variable is not vacuous and cannot be removed).
    pub fn bdd_essential_vars(&self, f: NodeId) -> Vec<u16> {
        if f.is_constant() {
            return Vec::new();
        }

        let support = self.bdd_support_set(f);
        let mut essential = Vec::new();

        for &v in &support {
            let (f_t, f_e) = self.bdd_cofactors(f, v);
            // Variable is essential if both cofactors differ from f
            // (f truly depends on it, and it cannot be projected away trivially)
            if f_t != f && f_e != f {
                essential.push(v);
            }
        }

        // Sort by variable level for consistent output
        essential.sort_by_key(|&v| self.perm[v as usize]);
        essential
    }

    // ==================================================================
    // Compatible projection
    // ==================================================================

    /// Project `f` onto the variables in `cube` (existential abstraction
    /// over all variables NOT in the cube), ensuring compatibility.
    ///
    /// `cube` is a conjunction of the variables to keep. All other variables
    /// in the support of `f` are existentially quantified away.
    pub fn bdd_compatible_projection(
        &mut self,
        f: NodeId,
        cube: NodeId,
    ) -> NodeId {
        if f.is_constant() {
            return f;
        }
        if cube.is_one() {
            // Keep no variables -- existentially abstract everything
            // Result is ONE if f is satisfiable, ZERO otherwise
            return if f.is_zero() { NodeId::ZERO } else { NodeId::ONE };
        }

        // Collect the variables in the cube
        let cube_vars = self.bdd_support_set(cube);

        // Collect the support of f
        let f_support = self.bdd_support_set(f);

        // Variables to quantify: those in f's support but not in cube
        let quant_vars: Vec<u16> = f_support
            .iter()
            .filter(|v| !cube_vars.contains(v))
            .copied()
            .collect();

        if quant_vars.is_empty() {
            return f;
        }

        // Build quantification cube
        let quant_cube = self.bdd_cube_from_vars(&quant_vars);

        // Existential abstraction
        self.bdd_exist_abstract(f, quant_cube)
    }

    /// Helper: build a cube from a Vec of variable indices.
    /// Unlike bdd_cube which takes a slice, this works with owned data
    /// to avoid borrow checker issues.
    fn bdd_cube_from_vars(&mut self, vars: &[u16]) -> NodeId {
        let mut sorted = vars.to_vec();
        sorted.sort_by(|a, b| self.perm[*b as usize].cmp(&self.perm[*a as usize]));
        let mut result = NodeId::ONE;
        for &v in &sorted {
            let var_node = self.bdd_ith_var(v);
            result = self.bdd_and(var_node, result);
        }
        result
    }
}
