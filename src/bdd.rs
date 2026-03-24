// lumindd — BDD (Binary Decision Diagram) operations
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use crate::computed_table::OpTag;
use crate::manager::Manager;
use crate::node::NodeId;

impl Manager {
    // ==================================================================
    // ITE (If-Then-Else) — the universal BDD operation
    // ==================================================================

    /// Compute `if f then g else h` (the ITE operation).
    ///
    /// This is the fundamental BDD operation from which AND, OR, XOR, etc.
    /// are all derived.
    pub fn bdd_ite(&mut self, f: NodeId, g: NodeId, h: NodeId) -> NodeId {
        // Terminal cases
        if f.is_one() {
            return g;
        }
        if f.is_zero() {
            return h;
        }
        if g.is_one() && h.is_zero() {
            return f; // ITE(f, 1, 0) = f
        }
        if g.is_zero() && h.is_one() {
            return f.not(); // ITE(f, 0, 1) = NOT f
        }
        if g == h {
            return g; // ITE(f, g, g) = g
        }
        if f == g {
            return self.bdd_ite(f, NodeId::ONE, h); // ITE(f, f, h) = ITE(f, 1, h)
        }
        if f == h {
            return self.bdd_ite(f, g, NodeId::ZERO); // ITE(f, g, f) = ITE(f, g, 0)
        }
        if f == g.not() {
            return self.bdd_ite(f, NodeId::ZERO, h); // ITE(f, !f, h) = ITE(f, 0, h)
        }
        if f == h.not() {
            return self.bdd_ite(f, g, NodeId::ONE); // ITE(f, g, !f) = ITE(f, g, 1)
        }

        // Normalize: if f is complemented, ITE(!f, g, h) = ITE(f, h, g)
        let (nf, ng, nh) = if f.is_complemented() {
            (f.not(), h, g)
        } else {
            (f, g, h)
        };

        // Check computed table
        if let Some(result) = self.cache.lookup(OpTag::BddIte, nf, ng, nh) {
            return result;
        }

        // Find the top variable (smallest level)
        let f_level = self.level(nf);
        let g_level = self.level(ng);
        let h_level = self.level(nh);
        let top_level = f_level.min(g_level).min(h_level);
        let top_var = self.inv_perm[top_level as usize] as u16;

        // Cofactor each operand with respect to the top variable
        let (f_t, f_e) = self.bdd_cofactors(nf, top_var);
        let (g_t, g_e) = self.bdd_cofactors(ng, top_var);
        let (h_t, h_e) = self.bdd_cofactors(nh, top_var);

        // Recurse
        let t = self.bdd_ite(f_t, g_t, h_t);
        let e = self.bdd_ite(f_e, g_e, h_e);

        // Build result
        let result = if t == e {
            t
        } else {
            self.unique_inter(top_var, t, e)
        };

        // Cache result
        self.cache.insert(OpTag::BddIte, nf, ng, nh, result);

        result
    }

    /// Get cofactors of f with respect to variable var_index.
    /// Returns (positive_cofactor, negative_cofactor).
    pub(crate) fn bdd_cofactors(&self, f: NodeId, var_index: u16) -> (NodeId, NodeId) {
        if f.is_constant() {
            return (f, f);
        }
        let node_var = self.var_index(f.regular());
        if node_var == var_index {
            let t = self.raw_then(f).not_cond(f.is_complemented());
            let e = self.raw_else(f).not_cond(f.is_complemented());
            (t, e)
        } else {
            // Variable is below this node, cofactors are both f
            (f, f)
        }
    }

    // ==================================================================
    // Basic Boolean operations (derived from ITE)
    // ==================================================================

    /// Compute `f AND g`.
    pub fn bdd_and(&mut self, f: NodeId, g: NodeId) -> NodeId {
        // Terminal cases for speed
        if f.is_one() {
            return g;
        }
        if g.is_one() {
            return f;
        }
        if f.is_zero() || g.is_zero() {
            return NodeId::ZERO;
        }
        if f == g {
            return f;
        }
        if f == g.not() {
            return NodeId::ZERO;
        }

        // Normalize operand order for better caching
        let (a, b) = if f.raw_index() > g.raw_index() {
            (g, f)
        } else {
            (f, g)
        };

        // Check computed table
        if let Some(result) = self.cache.lookup(OpTag::BddAnd, a, b, NodeId::ZERO) {
            return result;
        }

        // Find top variable
        let a_level = self.level(a);
        let b_level = self.level(b);
        let top_level = a_level.min(b_level);
        let top_var = self.inv_perm[top_level as usize] as u16;

        // Cofactor
        let (a_t, a_e) = self.bdd_cofactors(a, top_var);
        let (b_t, b_e) = self.bdd_cofactors(b, top_var);

        // Recurse
        let t = self.bdd_and(a_t, b_t);
        let e = self.bdd_and(a_e, b_e);

        let result = if t == e { t } else { self.unique_inter(top_var, t, e) };

        self.cache.insert(OpTag::BddAnd, a, b, NodeId::ZERO, result);
        result
    }

    /// Compute `NOT f` (O(1) — just flips the complement bit).
    #[inline]
    pub fn bdd_not(&self, f: NodeId) -> NodeId {
        f.not()
    }

    /// Compute `f OR g`.
    pub fn bdd_or(&mut self, f: NodeId, g: NodeId) -> NodeId {
        // NOT(NOT(f) AND NOT(g)) — De Morgan
        let nf = f.not();
        let ng = g.not();
        let and_result = self.bdd_and(nf, ng);
        and_result.not()
    }

    /// Compute `f XOR g`.
    pub fn bdd_xor(&mut self, f: NodeId, g: NodeId) -> NodeId {
        // Terminal cases
        if f == g {
            return NodeId::ZERO;
        }
        if f == g.not() {
            return NodeId::ONE;
        }
        if f.is_zero() {
            return g;
        }
        if f.is_one() {
            return g.not();
        }
        if g.is_zero() {
            return f;
        }
        if g.is_one() {
            return f.not();
        }

        // Normalize
        let (a, b) = if f.raw_index() > g.raw_index() {
            (g, f)
        } else {
            (f, g)
        };

        if let Some(result) = self.cache.lookup(OpTag::BddXor, a, b, NodeId::ZERO) {
            return result;
        }

        let a_level = self.level(a);
        let b_level = self.level(b);
        let top_level = a_level.min(b_level);
        let top_var = self.inv_perm[top_level as usize] as u16;

        let (a_t, a_e) = self.bdd_cofactors(a, top_var);
        let (b_t, b_e) = self.bdd_cofactors(b, top_var);

        let t = self.bdd_xor(a_t, b_t);
        let e = self.bdd_xor(a_e, b_e);

        let result = if t == e { t } else { self.unique_inter(top_var, t, e) };

        self.cache.insert(OpTag::BddXor, a, b, NodeId::ZERO, result);
        result
    }

    /// Compute `NOT(f AND g)`.
    pub fn bdd_nand(&mut self, f: NodeId, g: NodeId) -> NodeId {
        self.bdd_and(f, g).not()
    }

    /// Compute `NOT(f OR g)`.
    pub fn bdd_nor(&mut self, f: NodeId, g: NodeId) -> NodeId {
        self.bdd_or(f, g).not()
    }

    /// Compute `NOT(f XOR g)` (equivalence).
    pub fn bdd_xnor(&mut self, f: NodeId, g: NodeId) -> NodeId {
        self.bdd_xor(f, g).not()
    }

    /// Check if `f` implies `g` (f <= g).
    pub fn bdd_leq(&mut self, f: NodeId, g: NodeId) -> bool {
        let and_not = self.bdd_and(f, g.not());
        and_not.is_zero()
    }

    /// Check if f is the tautology (constant ONE).
    pub fn bdd_is_tautology(&self, f: NodeId) -> bool {
        f.is_one()
    }

    /// Check if f is unsatisfiable (constant ZERO).
    pub fn bdd_is_unsat(&self, f: NodeId) -> bool {
        f.is_zero()
    }

    // ==================================================================
    // Quantification (abstraction)
    // ==================================================================

    /// Existential abstraction: ∃ vars . f
    ///
    /// `cube` is a conjunction of variables to quantify over (built with
    /// `bdd_cube` or chained ANDs of variable projections).
    pub fn bdd_exist_abstract(&mut self, f: NodeId, cube: NodeId) -> NodeId {
        // Terminal cases
        if f.is_constant() {
            return f;
        }
        if cube.is_one() {
            return f;
        }

        if let Some(result) = self.cache.lookup(OpTag::BddExist, f, cube, NodeId::ZERO) {
            return result;
        }

        let f_level = self.level(f);
        let cube_level = self.level(cube);

        let result = if cube_level < f_level {
            // Cube variable is above f — skip it
            let next_cube = self.then_child(cube);
            self.bdd_exist_abstract(f, next_cube)
        } else if cube_level == f_level {
            // Quantify this variable
            let top_var = self.var_index(f.regular());
            let (f_t, f_e) = self.bdd_cofactors(f, top_var);
            let next_cube = self.then_child(cube);
            let t = self.bdd_exist_abstract(f_t, next_cube);
            let e = self.bdd_exist_abstract(f_e, next_cube);
            self.bdd_or(t, e)
        } else {
            // f variable is above cube — decompose f
            let top_var = self.var_index(f.regular());
            let (f_t, f_e) = self.bdd_cofactors(f, top_var);
            let t = self.bdd_exist_abstract(f_t, cube);
            let e = self.bdd_exist_abstract(f_e, cube);
            if t == e { t } else { self.unique_inter(top_var, t, e) }
        };

        self.cache.insert(OpTag::BddExist, f, cube, NodeId::ZERO, result);
        result
    }

    /// Universal abstraction: ∀ vars . f
    pub fn bdd_univ_abstract(&mut self, f: NodeId, cube: NodeId) -> NodeId {
        // ∀x.f = NOT(∃x.NOT(f))
        let nf = f.not();
        let exist = self.bdd_exist_abstract(nf, cube);
        exist.not()
    }

    /// Fused AND + existential abstraction: ∃ vars . (f AND g)
    pub fn bdd_and_abstract(&mut self, f: NodeId, g: NodeId, cube: NodeId) -> NodeId {
        // Terminal cases
        if f.is_zero() || g.is_zero() {
            return NodeId::ZERO;
        }
        if f.is_one() {
            return self.bdd_exist_abstract(g, cube);
        }
        if g.is_one() {
            return self.bdd_exist_abstract(f, cube);
        }
        if cube.is_one() {
            return self.bdd_and(f, g);
        }
        if f == g {
            return self.bdd_exist_abstract(f, cube);
        }

        // Normalize order
        let (a, b) = if f.raw_index() > g.raw_index() {
            (g, f)
        } else {
            (f, g)
        };

        if let Some(result) = self.cache.lookup(OpTag::BddAndAbstract, a, b, cube) {
            return result;
        }

        let a_level = self.level(a);
        let b_level = self.level(b);
        let c_level = self.level(cube);
        let top_level = a_level.min(b_level).min(c_level);
        let top_var = self.inv_perm[top_level as usize] as u16;

        let (a_t, a_e) = self.bdd_cofactors(a, top_var);
        let (b_t, b_e) = self.bdd_cofactors(b, top_var);
        let next_cube = if c_level == top_level {
            self.then_child(cube)
        } else {
            cube
        };

        let t = self.bdd_and_abstract(a_t, b_t, next_cube);
        let e = self.bdd_and_abstract(a_e, b_e, next_cube);

        let result = if c_level == top_level {
            // Quantify: OR the cofactors
            self.bdd_or(t, e)
        } else if t == e {
            t
        } else {
            self.unique_inter(top_var, t, e)
        };

        self.cache.insert(OpTag::BddAndAbstract, a, b, cube, result);
        result
    }

    // ==================================================================
    // Composition and cofactoring
    // ==================================================================

    /// Substitute g for variable v in f: f[v := g].
    pub fn bdd_compose(&mut self, f: NodeId, g: NodeId, v: u16) -> NodeId {
        if f.is_constant() {
            return f;
        }

        let f_var = self.var_index(f.regular());
        let f_level = self.perm[f_var as usize];
        let v_level = self.perm[v as usize];

        if f_level > v_level {
            // f does not depend on v
            return f;
        }

        if f_var == v {
            // Direct substitution
            let (f_t, f_e) = self.bdd_cofactors(f, v);
            return self.bdd_ite(g, f_t, f_e);
        }

        let cache_g = NodeId::from_raw(v as u32, g.is_complemented());
        if let Some(result) = self.cache.lookup(OpTag::BddCompose, f, g, cache_g) {
            return result;
        }

        let (f_t, f_e) = self.bdd_cofactors(f, f_var);
        let t = self.bdd_compose(f_t, g, v);
        let e = self.bdd_compose(f_e, g, v);

        let result = if t == e { t } else { self.unique_inter(f_var, t, e) };

        self.cache.insert(OpTag::BddCompose, f, g, cache_g, result);
        result
    }

    /// Generalized cofactor (restrict) of f by constraint c.
    ///
    /// Simplifies f assuming c is true. The result agrees with f
    /// on all assignments where c is true.
    pub fn bdd_restrict(&mut self, f: NodeId, c: NodeId) -> NodeId {
        if f.is_constant() {
            return f;
        }
        if c.is_one() {
            return f;
        }
        if c.is_zero() {
            return NodeId::ZERO;
        }

        if let Some(result) = self.cache.lookup(OpTag::BddRestrict, f, c, NodeId::ZERO) {
            return result;
        }

        let f_var = self.var_index(f.regular());
        let c_var = self.var_index(c.regular());
        let f_level = self.perm[f_var as usize];
        let c_level = self.perm[c_var as usize];

        let result = if f_level <= c_level {
            let (f_t, f_e) = self.bdd_cofactors(f, f_var);
            if f_level == c_level {
                let (c_t, c_e) = self.bdd_cofactors(c, c_var);
                if c_t.is_zero() {
                    self.bdd_restrict(f_e, c_e)
                } else if c_e.is_zero() {
                    self.bdd_restrict(f_t, c_t)
                } else {
                    let t = self.bdd_restrict(f_t, c_t);
                    let e = self.bdd_restrict(f_e, c_e);
                    if t == e { t } else { self.unique_inter(f_var, t, e) }
                }
            } else {
                let t = self.bdd_restrict(f_t, c);
                let e = self.bdd_restrict(f_e, c);
                if t == e { t } else { self.unique_inter(f_var, t, e) }
            }
        } else {
            // c variable above f — take the appropriate cofactor of c
            let (c_t, c_e) = self.bdd_cofactors(c, c_var);
            if c_t.is_zero() {
                self.bdd_restrict(f, c_e)
            } else if c_e.is_zero() {
                self.bdd_restrict(f, c_t)
            } else {
                self.bdd_restrict(f, c_t) // heuristic: take positive cofactor
            }
        };

        self.cache.insert(OpTag::BddRestrict, f, c, NodeId::ZERO, result);
        result
    }

    /// Constrain f by c (Coudert & Madre).
    pub fn bdd_constrain(&mut self, f: NodeId, c: NodeId) -> NodeId {
        if f.is_constant() {
            return f;
        }
        if c.is_one() {
            return f;
        }
        if c.is_zero() {
            return NodeId::ZERO;
        }
        if f == c {
            return NodeId::ONE;
        }
        if f == c.not() {
            return NodeId::ZERO;
        }

        if let Some(result) = self.cache.lookup(OpTag::BddConstrain, f, c, NodeId::ZERO) {
            return result;
        }

        let f_var = self.var_index(f.regular());
        let c_var = self.var_index(c.regular());
        let f_level = self.perm[f_var as usize];
        let c_level = self.perm[c_var as usize];

        let result = if f_level <= c_level {
            let top_var = f_var;
            let (f_t, f_e) = self.bdd_cofactors(f, top_var);
            if f_level == c_level {
                let (c_t, c_e) = self.bdd_cofactors(c, top_var);
                if c_t.is_zero() {
                    self.bdd_constrain(f_e, c_e)
                } else if c_e.is_zero() {
                    self.bdd_constrain(f_t, c_t)
                } else {
                    let t = self.bdd_constrain(f_t, c_t);
                    let e = self.bdd_constrain(f_e, c_e);
                    if t == e { t } else { self.unique_inter(top_var, t, e) }
                }
            } else {
                let t = self.bdd_constrain(f_t, c);
                let e = self.bdd_constrain(f_e, c);
                if t == e { t } else { self.unique_inter(top_var, t, e) }
            }
        } else {
            let (c_t, c_e) = self.bdd_cofactors(c, c_var);
            if c_t.is_zero() {
                self.bdd_constrain(f, c_e)
            } else if c_e.is_zero() {
                self.bdd_constrain(f, c_t)
            } else {
                let t = self.bdd_constrain(f, c_t);
                let e = self.bdd_constrain(f, c_e);
                if t == e { t } else { self.unique_inter(c_var, t, e) }
            }
        };

        self.cache.insert(OpTag::BddConstrain, f, c, NodeId::ZERO, result);
        result
    }

    // ==================================================================
    // Cube construction helpers
    // ==================================================================

    /// Build a cube (conjunction) from a set of variable indices.
    pub fn bdd_cube(&mut self, vars: &[u16]) -> NodeId {
        let mut result = NodeId::ONE;
        // Build bottom-up (reverse order by level for efficiency)
        let mut sorted: Vec<u16> = vars.to_vec();
        sorted.sort_by(|a, b| self.perm[*b as usize].cmp(&self.perm[*a as usize]));
        for &v in &sorted {
            let var_node = self.bdd_ith_var(v);
            result = self.bdd_and(var_node, result);
        }
        result
    }

    /// Build a cube from variables with specified polarities.
    /// `phase[i]` is true for positive literal, false for negative.
    pub fn bdd_cube_with_phase(&mut self, vars: &[u16], phase: &[bool]) -> NodeId {
        assert_eq!(vars.len(), phase.len());
        let mut result = NodeId::ONE;
        let mut pairs: Vec<(u16, bool)> = vars.iter().copied().zip(phase.iter().copied()).collect();
        pairs.sort_by(|a, b| self.perm[b.0 as usize].cmp(&self.perm[a.0 as usize]));
        for (v, p) in pairs {
            let var_node = self.bdd_ith_var(v);
            let lit = if p { var_node } else { var_node.not() };
            result = self.bdd_and(lit, result);
        }
        result
    }
}
