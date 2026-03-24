// lumindd — Additional BDD operations for CUDD parity
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

//! Extra BDD operations: fused XOR-exist, Boolean difference, prime implicant
//! expansion, minterm picking, set splitting, intersection, comparison
//! predicates, adjacent variable permutation, and Lucky Image compaction.

use crate::manager::Manager;
use crate::node::NodeId;
use crate::unique_table::UniqueSubtable;

impl Manager {
    // ==================================================================
    // 1. Fused XOR + existential abstraction
    // ==================================================================

    /// Fused XOR + existential abstraction: `∃vars.(f XOR g)`.
    ///
    /// Computes `bdd_xor(f, g)` then existentially abstracts the variables
    /// encoded in `cube`.
    pub fn bdd_xor_exist_abstract(
        &mut self,
        f: NodeId,
        g: NodeId,
        cube: NodeId,
    ) -> NodeId {
        let xor = self.bdd_xor(f, g);
        self.bdd_exist_abstract(xor, cube)
    }

    // ==================================================================
    // 2. Boolean difference
    // ==================================================================

    /// Boolean difference (derivative) of `f` with respect to variable `var`:
    ///
    ///   `df/dxi = cofactor_pos(f, xi) XOR cofactor_neg(f, xi)`
    ///
    /// The result is ONE for every assignment where flipping `var` changes
    /// the value of `f`.
    pub fn bdd_boolean_diff(&mut self, f: NodeId, var: u16) -> NodeId {
        let (f_pos, f_neg) = self.bdd_cofactors(f, var);
        self.bdd_xor(f_pos, f_neg)
    }

    // ==================================================================
    // 3. New variable at a specific level
    // ==================================================================

    /// Create a new BDD variable and insert it at the specified `level`.
    ///
    /// All existing variables whose level is `>= level` are shifted down by one.
    /// The `perm`, `inv_perm`, and `unique_tables` arrays are updated accordingly.
    /// Returns the projection function for the new variable.
    pub fn bdd_new_var_at_level(&mut self, level: u32) -> NodeId {
        let new_var = self.num_vars;
        let level = level.min(new_var as u32); // clamp to valid range
        self.num_vars += 1;

        // Shift every existing variable at level >= target down by 1
        for v in 0..new_var {
            if self.perm[v as usize] >= level {
                self.perm[v as usize] += 1;
            }
        }

        // Assign the new variable to the requested level
        self.perm.push(level);

        // Rebuild inv_perm from scratch (level -> var)
        let n = self.num_vars as usize;
        self.inv_perm.resize(n, 0);
        for v in 0..n {
            let lv = self.perm[v] as usize;
            self.inv_perm[lv] = v as u32;
        }

        // Insert a new unique sub-table at the requested level,
        // shifting existing tables down.
        self.unique_tables.insert(level as usize, UniqueSubtable::new());

        // Create the projection function: if new_var then ONE else ZERO
        let node = self.unique_inter(new_var, NodeId::ONE, NodeId::ZERO);
        self.ref_node(node);
        node
    }

    // ==================================================================
    // 4. Make prime implicant
    // ==================================================================

    /// Expand a cube (minterm) to a prime implicant of `f`.
    ///
    /// For each literal in the cube, try removing it (replacing it with
    /// don't-care). If the resulting larger cube still implies `f`, keep
    /// the removal; otherwise restore the literal.
    ///
    /// Both `cube` and `f` are BDDs; `cube` must imply `f`.
    /// Returns a BDD representing the prime implicant (a conjunction of
    /// literals).
    pub fn bdd_make_prime(&mut self, cube: NodeId, f: NodeId) -> NodeId {
        if cube.is_zero() || f.is_zero() {
            return NodeId::ZERO;
        }
        if f.is_one() {
            // Everything implies tautology — the prime is just TRUE
            return NodeId::ONE;
        }

        // Collect the literals present in the cube.
        // A cube BDD is a chain: each internal node has one child = ZERO
        // (or complemented ONE) and the other child continues the chain.
        let mut literals: Vec<(u16, bool)> = Vec::new();
        self.collect_cube_literals(cube, &mut literals);

        // Sort by level (top-first) for a canonical processing order
        literals.sort_by_key(|&(v, _)| self.perm[v as usize]);

        // Try removing each literal
        let mut result = cube;
        for &(var, _phase) in &literals {
            // Remove this literal: existentially abstract the variable
            let var_cube = self.bdd_ith_var(var);
            let expanded = self.bdd_exist_abstract(result, var_cube);

            // Check: does the expanded cube still imply f?
            // expanded => f  iff  expanded AND NOT(f) == ZERO
            let check = self.bdd_and(expanded, f.not());
            if check.is_zero() {
                result = expanded;
            }
        }

        result
    }

    /// Helper: collect literals from a cube BDD.
    ///
    /// Walks the cube top-down. At each internal node whose variable is `v`:
    /// - if then-child leads to the rest and else-child is ZERO: positive literal
    /// - if else-child leads to the rest and then-child is ZERO: negative literal
    /// (with complemented edges the detection is a bit more nuanced)
    fn collect_cube_literals(&self, cube: NodeId, out: &mut Vec<(u16, bool)>) {
        let mut current = cube;
        while !current.is_constant() {
            let var = self.var_index(current.regular());
            let t = self.then_child(current);
            let e = self.else_child(current);

            if e.is_zero() {
                // Positive literal: var = 1
                out.push((var, true));
                current = t;
            } else if t.is_zero() {
                // Negative literal: var = 0
                out.push((var, false));
                current = e;
            } else {
                // Not a simple cube — take then-branch as positive literal
                out.push((var, true));
                current = t;
            }
        }
    }

    // ==================================================================
    // 5. Pick one minterm as a BDD cube
    // ==================================================================

    /// Pick one minterm of `f` as a BDD cube over the given `vars`.
    ///
    /// Returns a conjunction of literals (one per variable in `vars`)
    /// representing a single satisfying assignment of `f` restricted to
    /// those variables. Returns ZERO if `f` is unsatisfiable.
    pub fn bdd_pick_one_minterm(
        &mut self,
        f: NodeId,
        vars: &[u16],
    ) -> NodeId {
        if f.is_zero() {
            return NodeId::ZERO;
        }

        // Find a satisfying assignment by walking the BDD
        let mut assignment: Vec<(u16, bool)> = Vec::new();
        let mut current = f;

        // Sort vars by level for ordered traversal
        let mut sorted_vars: Vec<u16> = vars.to_vec();
        sorted_vars.sort_by_key(|&v| self.perm[v as usize]);

        for &v in &sorted_vars {
            if current.is_constant() {
                // Remaining variables are don't-care; pick false
                assignment.push((v, false));
                continue;
            }

            let node_var = self.var_index(current.regular());
            let node_level = self.perm[node_var as usize];
            let v_level = self.perm[v as usize];

            if v_level < node_level {
                // Variable v is above the current node — it's don't-care,
                // pick false
                assignment.push((v, false));
            } else if node_var == v {
                // This node decides on v
                let t = self.then_child(current);
                let e = self.else_child(current);
                if !t.is_zero() {
                    assignment.push((v, true));
                    current = t;
                } else {
                    assignment.push((v, false));
                    current = e;
                }
            } else {
                // v_level > node_level: current node's variable is above v
                // in the ordering; we need to pick a branch of the current
                // node and continue looking for v.
                // First, descend through variables above v
                while !current.is_constant() {
                    let cv = self.var_index(current.regular());
                    let cl = self.perm[cv as usize];
                    if cl >= v_level {
                        break;
                    }
                    // Pick the then-branch if it is not zero, else the else-branch
                    let t = self.then_child(current);
                    if !t.is_zero() {
                        current = t;
                    } else {
                        current = self.else_child(current);
                    }
                }
                // Now handle v at the current position
                if current.is_constant() {
                    assignment.push((v, false));
                } else {
                    let cv = self.var_index(current.regular());
                    if cv == v {
                        let t = self.then_child(current);
                        let e = self.else_child(current);
                        if !t.is_zero() {
                            assignment.push((v, true));
                            current = t;
                        } else {
                            assignment.push((v, false));
                            current = e;
                        }
                    } else {
                        // v is not in the BDD path — don't-care
                        assignment.push((v, false));
                    }
                }
            }
        }

        // Build the cube BDD from the assignment (bottom-up by level)
        assignment.sort_by(|a, b| self.perm[b.0 as usize].cmp(&self.perm[a.0 as usize]));
        let mut result = NodeId::ONE;
        for &(v, phase) in &assignment {
            let var_node = self.bdd_ith_var(v);
            let lit = if phase { var_node } else { var_node.not() };
            result = self.bdd_and(lit, result);
        }

        result
    }

    // ==================================================================
    // 6. Split set
    // ==================================================================

    /// Split `f` into two parts `(g, h)` such that `g OR h = f`,
    /// `g AND h = ZERO`, and `g` has approximately `n` minterms
    /// (counted over `vars`).
    ///
    /// Uses the top variable (in the current ordering among `vars`) to
    /// partition: the positive cofactor forms part of `g` and the
    /// negative cofactor forms part of `h`, recursing as needed.
    pub fn bdd_split_set(
        &mut self,
        f: NodeId,
        vars: &[u16],
        n: f64,
    ) -> (NodeId, NodeId) {
        if f.is_zero() || n <= 0.0 {
            return (NodeId::ZERO, f);
        }
        let total = self.bdd_count_minterm(f, vars.len() as u32);
        if n >= total {
            return (f, NodeId::ZERO);
        }

        // Find the top variable of f among the given vars
        let f_level = self.level(f);
        let mut top_var = None;
        let mut top_level = u32::MAX;
        for &v in vars {
            let vl = self.perm[v as usize];
            if vl < top_level && vl >= f_level {
                top_level = vl;
                top_var = Some(v);
            }
        }

        let top_var = match top_var {
            Some(v) => v,
            None => return (f, NodeId::ZERO),
        };

        let (f_t, f_e) = self.bdd_cofactors(f, top_var);
        let var_node = self.bdd_ith_var(top_var);

        // Count minterms in the positive cofactor over (vars.len() - 1) variables
        let nv = if vars.len() > 1 { vars.len() as u32 - 1 } else { 0 };
        let count_t = self.bdd_count_minterm(f_t, nv);

        if count_t <= n {
            // Take all of the positive cofactor, split the negative cofactor
            let remaining = n - count_t;
            let (g_e, h_e) = self.bdd_split_set(f_e, vars, remaining);
            let g_part = self.bdd_and(var_node, f_t);
            let g = self.bdd_or(g_part, g_e);
            let h = self.bdd_and(var_node.not(), h_e);
            (g, h)
        } else {
            // Split within the positive cofactor
            let (g_t, h_t) = self.bdd_split_set(f_t, vars, n);
            let g = self.bdd_and(var_node, g_t);
            let h_part = self.bdd_and(var_node, h_t);
            let h_neg = self.bdd_and(var_node.not(), f_e);
            let h = self.bdd_or(h_part, h_neg);
            (g, h)
        }
    }

    // ==================================================================
    // 7. Intersect (find a function between f and g)
    // ==================================================================

    /// Find a function `h` such that `f AND g` is satisfied — essentially
    /// computes `f AND g` but with early termination optimizations.
    ///
    /// In CUDD, `Cudd_bddIntersect` finds a function between `f` and `g`
    /// that is potentially simpler. Here we implement a recursive version
    /// that eagerly terminates when one operand implies the other.
    pub fn bdd_intersect(&mut self, f: NodeId, g: NodeId) -> NodeId {
        // Terminal cases
        if f.is_zero() || g.is_zero() {
            return NodeId::ZERO;
        }
        if f.is_one() {
            return g;
        }
        if g.is_one() {
            return f;
        }
        if f == g {
            return f;
        }
        if f == g.not() {
            return NodeId::ZERO;
        }

        // Optimization: if f implies g, return f; if g implies f, return g
        // (this makes the result potentially smaller than f AND g)
        let f_and_not_g = self.bdd_and(f, g.not());
        if f_and_not_g.is_zero() {
            return f; // f implies g
        }
        let g_and_not_f = self.bdd_and(g, f.not());
        if g_and_not_f.is_zero() {
            return g; // g implies f
        }

        // Fall back to AND
        self.bdd_and(f, g)
    }

    // ==================================================================
    // 8. BDD for x == y (bitwise equality)
    // ==================================================================

    /// Build a BDD for the predicate `x == y`, where `x_vars` and `y_vars`
    /// are vectors of variable indices representing the bits of `x` and `y`
    /// respectively (MSB first).
    ///
    /// The result is the conjunction `(x0 XNOR y0) AND (x1 XNOR y1) AND ...`.
    pub fn bdd_xeqy(&mut self, x_vars: &[u16], y_vars: &[u16]) -> NodeId {
        assert_eq!(
            x_vars.len(),
            y_vars.len(),
            "x_vars and y_vars must have the same length"
        );
        let mut result = NodeId::ONE;
        // Build bottom-up for better BDD structure
        for i in (0..x_vars.len()).rev() {
            let xi = self.bdd_ith_var(x_vars[i]);
            let yi = self.bdd_ith_var(y_vars[i]);
            let eq_i = self.bdd_xnor(xi, yi); // xi XNOR yi = NOT(xi XOR yi)
            result = self.bdd_and(eq_i, result);
        }
        result
    }

    // ==================================================================
    // 9. BDD for x > y (unsigned comparison)
    // ==================================================================

    /// Build a BDD for the predicate `x > y` (unsigned), where `x_vars` and
    /// `y_vars` are vectors of variable indices (MSB first).
    ///
    /// Built from MSB to LSB using the recurrence:
    ///   `x > y` iff `(x_msb > y_msb) OR (x_msb == y_msb AND x_rest > y_rest)`
    pub fn bdd_xgty(&mut self, x_vars: &[u16], y_vars: &[u16]) -> NodeId {
        assert_eq!(
            x_vars.len(),
            y_vars.len(),
            "x_vars and y_vars must have the same length"
        );
        if x_vars.is_empty() {
            return NodeId::ZERO; // no bits => cannot be greater
        }

        // Build from LSB to MSB
        let mut gt = NodeId::ZERO; // accumulated "x > y" for lower bits
        for i in (0..x_vars.len()).rev() {
            let xi = self.bdd_ith_var(x_vars[i]);
            let yi = self.bdd_ith_var(y_vars[i]);

            // xi > yi at this bit: xi=1 AND yi=0
            let xi_gt_yi = self.bdd_and(xi, yi.not());

            // xi == yi at this bit: xi XNOR yi
            let xi_eq_yi = self.bdd_xnor(xi, yi);

            // x > y = (xi > yi) OR (xi == yi AND previous_gt)
            let eq_and_prev = self.bdd_and(xi_eq_yi, gt);
            gt = self.bdd_or(xi_gt_yi, eq_and_prev);
        }

        gt
    }

    // ==================================================================
    // 10. Adjacent variable permutation (swap with next lower level)
    // ==================================================================

    /// Swap variable `var` with the variable at the adjacent lower level
    /// in the BDD `f`.
    ///
    /// If `var` is at level `L`, then it is swapped with the variable at
    /// level `L + 1`. This is equivalent to `bdd_swap_variables` restricted
    /// to two adjacent variables.
    ///
    /// Returns the modified BDD. Panics if `var` is at the bottom level.
    pub fn bdd_adj_permute_x(&mut self, f: NodeId, var: u16) -> NodeId {
        let level = self.perm[var as usize];
        let max_level = self.num_vars as u32 - 1;
        assert!(
            level < max_level,
            "variable {} is at level {} (bottom); no adjacent lower level",
            var,
            level
        );
        let adj_var = self.inv_perm[(level + 1) as usize] as u16;
        self.bdd_swap_variables(f, &[var], &[adj_var])
    }

    // ==================================================================
    // 11. Lucky Image compaction
    // ==================================================================

    /// Lucky Image (LI) compaction: simplify `f` given care set `c`.
    ///
    /// Produces a function that agrees with `f` wherever `c` is true and
    /// may differ elsewhere. The goal is a smaller BDD than `f` by
    /// exploiting the don't-care space (`NOT c`).
    ///
    /// Algorithm: recursively walk `f` and `c` together. When the top
    /// variable of `c` is above `f`, choose the non-zero cofactor of `c`
    /// to descend into (collapsing a don't-care variable). Otherwise,
    /// decompose both by their top variable and recurse.
    pub fn bdd_li_compaction(&mut self, f: NodeId, c: NodeId) -> NodeId {
        // Terminal cases
        if c.is_zero() {
            // No care set — return any function (ZERO is simplest)
            return NodeId::ZERO;
        }
        if c.is_one() {
            return f;
        }
        if f.is_constant() {
            return f;
        }
        if f == c {
            return NodeId::ONE;
        }
        if f == c.not() {
            return NodeId::ZERO;
        }

        let f_var = self.var_index(f.regular());
        let c_var = self.var_index(c.regular());
        let f_level = self.perm[f_var as usize];
        let c_level = self.perm[c_var as usize];

        if c_level < f_level {
            // c's variable is above f — c splits before f does.
            // Take the non-zero cofactor of c to continue.
            let (c_t, c_e) = self.bdd_cofactors(c, c_var);
            if c_t.is_zero() {
                self.bdd_li_compaction(f, c_e)
            } else if c_e.is_zero() {
                self.bdd_li_compaction(f, c_t)
            } else {
                // Both cofactors are non-zero; pick the positive one
                // (heuristic — could also try both and pick smaller result)
                self.bdd_li_compaction(f, c_t)
            }
        } else if f_level < c_level {
            // f's variable is above c
            let (f_t, f_e) = self.bdd_cofactors(f, f_var);
            let t = self.bdd_li_compaction(f_t, c);
            let e = self.bdd_li_compaction(f_e, c);
            if t == e {
                t
            } else {
                self.unique_inter(f_var, t, e)
            }
        } else {
            // Same variable
            let (f_t, f_e) = self.bdd_cofactors(f, f_var);
            let (c_t, c_e) = self.bdd_cofactors(c, c_var);

            // If one cofactor of c is zero, we only need the other
            if c_t.is_zero() {
                return self.bdd_li_compaction(f_e, c_e);
            }
            if c_e.is_zero() {
                return self.bdd_li_compaction(f_t, c_t);
            }

            let t = self.bdd_li_compaction(f_t, c_t);
            let e = self.bdd_li_compaction(f_e, c_e);
            if t == e {
                t
            } else {
                self.unique_inter(f_var, t, e)
            }
        }
    }
}

// ======================================================================
// Tests
// ======================================================================

#[cfg(test)]
mod tests {
    use crate::Manager;
    use crate::NodeId;

    // ------------------------------------------------------------------
    // 1. bdd_xor_exist_abstract
    // ------------------------------------------------------------------

    #[test]
    fn test_xor_exist_abstract_basic() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var(); // x0
        let y = mgr.bdd_new_var(); // x1

        // ∃x0.(x0 XOR x1)
        // x0 XOR x1 is true when x0 != x1
        // ∃x0 means: for any x1, is there an x0 making it true? Yes, always.
        let cube = mgr.bdd_ith_var(0);
        let result = mgr.bdd_xor_exist_abstract(x, y, cube);
        assert!(result.is_one(), "∃x0.(x0 XOR x1) should be tautology");
    }

    #[test]
    fn test_xor_exist_abstract_same() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let _y = mgr.bdd_new_var();

        // ∃x0.(x0 XOR x0) = ∃x0.0 = 0
        let cube = mgr.bdd_ith_var(0);
        let result = mgr.bdd_xor_exist_abstract(x, x, cube);
        assert!(result.is_zero());
    }

    // ------------------------------------------------------------------
    // 2. bdd_boolean_diff
    // ------------------------------------------------------------------

    #[test]
    fn test_boolean_diff_single_var() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var(); // x0

        // df/dx0 where f = x0: cofactor_pos = 1, cofactor_neg = 0 => 1 XOR 0 = 1
        let diff = mgr.bdd_boolean_diff(x, 0);
        assert!(diff.is_one(), "Boolean diff of x0 w.r.t. x0 should be 1");
    }

    #[test]
    fn test_boolean_diff_independent() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var(); // x0
        let _y = mgr.bdd_new_var(); // x1

        // df/dx1 where f = x0: cofactors of x0 w.r.t. x1 are both x0
        // => x0 XOR x0 = 0
        let diff = mgr.bdd_boolean_diff(x, 1);
        assert!(diff.is_zero(), "Boolean diff of x0 w.r.t. x1 should be 0");
    }

    #[test]
    fn test_boolean_diff_and() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var(); // x0
        let y = mgr.bdd_new_var(); // x1
        let f = mgr.bdd_and(x, y); // f = x0 AND x1

        // df/dx0 = cofactor_pos(f, x0) XOR cofactor_neg(f, x0)
        //        = x1 XOR 0 = x1
        let diff = mgr.bdd_boolean_diff(f, 0);
        assert_eq!(diff, y, "Boolean diff of (x0 AND x1) w.r.t. x0 should be x1");
    }

    #[test]
    fn test_boolean_diff_xor() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var(); // x0
        let y = mgr.bdd_new_var(); // x1
        let f = mgr.bdd_xor(x, y); // f = x0 XOR x1

        // df/dx0 = cofactor_pos(x0 XOR x1, x0) XOR cofactor_neg(x0 XOR x1, x0)
        //        = NOT(x1) XOR x1 = 1
        let diff = mgr.bdd_boolean_diff(f, 0);
        assert!(
            diff.is_one(),
            "Boolean diff of (x0 XOR x1) w.r.t. x0 should be 1"
        );
    }

    // ------------------------------------------------------------------
    // 3. bdd_new_var_at_level
    // ------------------------------------------------------------------

    #[test]
    fn test_new_var_at_level_top() {
        let mut mgr = Manager::new();
        let _x0 = mgr.bdd_new_var(); // var 0, level 0
        let _x1 = mgr.bdd_new_var(); // var 1, level 1

        // Insert new var at level 0 (top)
        let x2 = mgr.bdd_new_var_at_level(0);
        assert_eq!(mgr.num_vars(), 3);

        // New variable (index 2) should be at level 0
        assert_eq!(mgr.perm[2], 0, "new var should be at level 0");
        // Old var 0 should have shifted to level 1
        assert_eq!(mgr.perm[0], 1, "var 0 should now be at level 1");
        // Old var 1 should have shifted to level 2
        assert_eq!(mgr.perm[1], 2, "var 1 should now be at level 2");

        // inv_perm should be consistent
        assert_eq!(mgr.inv_perm[0], 2);
        assert_eq!(mgr.inv_perm[1], 0);
        assert_eq!(mgr.inv_perm[2], 1);

        // The projection function should not be constant
        assert!(!x2.is_constant());
    }

    #[test]
    fn test_new_var_at_level_middle() {
        let mut mgr = Manager::new();
        let _x0 = mgr.bdd_new_var(); // var 0, level 0
        let _x1 = mgr.bdd_new_var(); // var 1, level 1
        let _x2 = mgr.bdd_new_var(); // var 2, level 2

        // Insert new var at level 1 (middle)
        let _x3 = mgr.bdd_new_var_at_level(1);
        assert_eq!(mgr.num_vars(), 4);

        assert_eq!(mgr.perm[0], 0, "var 0 stays at level 0");
        assert_eq!(mgr.perm[3], 1, "new var 3 at level 1");
        assert_eq!(mgr.perm[1], 2, "var 1 shifted to level 2");
        assert_eq!(mgr.perm[2], 3, "var 2 shifted to level 3");
    }

    #[test]
    fn test_new_var_at_level_bottom() {
        let mut mgr = Manager::new();
        let _x0 = mgr.bdd_new_var(); // var 0, level 0
        let _x1 = mgr.bdd_new_var(); // var 1, level 1

        // Insert at level 2 (bottom — same as bdd_new_var)
        let _x2 = mgr.bdd_new_var_at_level(2);
        assert_eq!(mgr.num_vars(), 3);

        assert_eq!(mgr.perm[0], 0);
        assert_eq!(mgr.perm[1], 1);
        assert_eq!(mgr.perm[2], 2);
    }

    // ------------------------------------------------------------------
    // 4. bdd_make_prime
    // ------------------------------------------------------------------

    #[test]
    fn test_make_prime_basic() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var(); // x0
        let y = mgr.bdd_new_var(); // x1
        let z = mgr.bdd_new_var(); // x2

        // f = x0 OR x1
        let f = mgr.bdd_or(x, y);

        // cube = x0 AND x1 AND NOT(x2) — a minterm of f
        let nz = mgr.bdd_not(z);
        let ynz = mgr.bdd_and(y, nz);
        let cube = mgr.bdd_and(x, ynz);

        let prime = mgr.bdd_make_prime(cube, f);

        // The prime should imply f
        let check = mgr.bdd_and(prime, f.not());
        assert!(check.is_zero(), "prime must imply f");

        // The prime should have fewer literals than the original cube.
        // x0 alone implies (x0 OR x1), so the prime could be just x0.
        let prime_support = mgr.bdd_support(prime);
        assert!(
            prime_support.len() <= 2,
            "prime should have at most 2 literals, got {}",
            prime_support.len()
        );
    }

    #[test]
    fn test_make_prime_tautology() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();

        // f = ONE (tautology). Any cube's prime should be ONE.
        let prime = mgr.bdd_make_prime(x, NodeId::ONE);
        assert!(prime.is_one());
    }

    #[test]
    fn test_make_prime_zero() {
        let mut mgr = Manager::new();
        let _x = mgr.bdd_new_var();

        let prime = mgr.bdd_make_prime(NodeId::ZERO, NodeId::ONE);
        assert!(prime.is_zero());
    }

    // ------------------------------------------------------------------
    // 5. bdd_pick_one_minterm
    // ------------------------------------------------------------------

    #[test]
    fn test_pick_one_minterm_and() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var(); // x0
        let y = mgr.bdd_new_var(); // x1
        let f = mgr.bdd_and(x, y); // f = x0 AND x1

        let vars = vec![0, 1];
        let minterm = mgr.bdd_pick_one_minterm(f, &vars);

        // The minterm should imply f
        let check = mgr.bdd_and(minterm, f.not());
        assert!(check.is_zero(), "minterm must imply f");

        // Should have exactly 1 satisfying assignment over 2 vars
        let count = mgr.bdd_count_minterm(minterm, 2);
        assert!(
            (count - 1.0).abs() < 1e-10,
            "minterm should have exactly 1 minterm, got {}",
            count
        );
    }

    #[test]
    fn test_pick_one_minterm_or() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_or(x, y);

        let vars = vec![0, 1];
        let minterm = mgr.bdd_pick_one_minterm(f, &vars);

        // Must imply f
        let check = mgr.bdd_and(minterm, f.not());
        assert!(check.is_zero());

        // Exactly 1 minterm
        let count = mgr.bdd_count_minterm(minterm, 2);
        assert!((count - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_pick_one_minterm_zero() {
        let mut mgr = Manager::new();
        let _x = mgr.bdd_new_var();
        let result = mgr.bdd_pick_one_minterm(NodeId::ZERO, &[0]);
        assert!(result.is_zero());
    }

    #[test]
    fn test_pick_one_minterm_tautology() {
        let mut mgr = Manager::new();
        let _x = mgr.bdd_new_var();
        let _y = mgr.bdd_new_var();

        let vars = vec![0, 1];
        let minterm = mgr.bdd_pick_one_minterm(NodeId::ONE, &vars);

        // Should have exactly 1 minterm
        let count = mgr.bdd_count_minterm(minterm, 2);
        assert!((count - 1.0).abs() < 1e-10);
    }

    // ------------------------------------------------------------------
    // 6. bdd_split_set
    // ------------------------------------------------------------------

    #[test]
    fn test_split_set_basic() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_or(x, y); // 3 minterms out of 4

        let vars = vec![0, 1];
        let (g, h) = mgr.bdd_split_set(f, &vars, 1.0);

        // g OR h should equal f
        let union = mgr.bdd_or(g, h);
        assert_eq!(union, f, "g | h should equal f");

        // g AND h should be ZERO (disjoint split)
        let inter = mgr.bdd_and(g, h);
        assert!(inter.is_zero(), "g & h should be empty");
    }

    #[test]
    fn test_split_set_all_in_first() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let _y = mgr.bdd_new_var();

        let (g, h) = mgr.bdd_split_set(x, &[0, 1], 100.0);
        assert_eq!(g, x);
        assert!(h.is_zero());
    }

    #[test]
    fn test_split_set_none_in_first() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let _y = mgr.bdd_new_var();

        let (g, h) = mgr.bdd_split_set(x, &[0, 1], 0.0);
        assert!(g.is_zero());
        assert_eq!(h, x);
    }

    // ------------------------------------------------------------------
    // 7. bdd_intersect
    // ------------------------------------------------------------------

    #[test]
    fn test_intersect_basic() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_or(x, y);
        let g = mgr.bdd_and(x, y);

        // g implies f, so intersect should return g
        let h = mgr.bdd_intersect(f, g);
        // h must satisfy: h implies f AND h implies g ... or at minimum f AND g
        let check_f = mgr.bdd_and(h, f.not());
        assert!(check_f.is_zero(), "h must imply f");
    }

    #[test]
    fn test_intersect_same() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let h = mgr.bdd_intersect(x, x);
        assert_eq!(h, x);
    }

    #[test]
    fn test_intersect_complement() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let h = mgr.bdd_intersect(x, x.not());
        assert!(h.is_zero());
    }

    // ------------------------------------------------------------------
    // 8. bdd_xeqy
    // ------------------------------------------------------------------

    #[test]
    fn test_xeqy_single_bit() {
        let mut mgr = Manager::new();
        let _x = mgr.bdd_new_var(); // var 0
        let _y = mgr.bdd_new_var(); // var 1

        let eq = mgr.bdd_xeqy(&[0], &[1]);

        // x0 == x1: true when both 0 or both 1 => 2 minterms out of 4
        let count = mgr.bdd_count_minterm(eq, 2);
        assert!(
            (count - 2.0).abs() < 1e-10,
            "x0 == x1 should have 2 minterms, got {}",
            count
        );

        // Verify: x0=0,x1=0 -> true; x0=1,x1=1 -> true
        assert!(mgr.bdd_eval(eq, &[false, false]));
        assert!(mgr.bdd_eval(eq, &[true, true]));
        assert!(!mgr.bdd_eval(eq, &[true, false]));
        assert!(!mgr.bdd_eval(eq, &[false, true]));
    }

    #[test]
    fn test_xeqy_two_bits() {
        let mut mgr = Manager::new();
        for _ in 0..4 {
            mgr.bdd_new_var(); // vars 0..3
        }

        // x = (var0, var1), y = (var2, var3)
        let eq = mgr.bdd_xeqy(&[0, 1], &[2, 3]);

        // 4 possible x values, 4 possible y values, 4 matching pairs out of 16
        let count = mgr.bdd_count_minterm(eq, 4);
        assert!(
            (count - 4.0).abs() < 1e-10,
            "2-bit x==y should have 4 minterms, got {}",
            count
        );

        // x=01, y=01 => true
        assert!(mgr.bdd_eval(eq, &[false, true, false, true]));
        // x=01, y=10 => false
        assert!(!mgr.bdd_eval(eq, &[false, true, true, false]));
    }

    // ------------------------------------------------------------------
    // 9. bdd_xgty
    // ------------------------------------------------------------------

    #[test]
    fn test_xgty_single_bit() {
        let mut mgr = Manager::new();
        let _x = mgr.bdd_new_var(); // var 0
        let _y = mgr.bdd_new_var(); // var 1

        let gt = mgr.bdd_xgty(&[0], &[1]);

        // x0 > x1 (unsigned): only when x0=1, x1=0 => 1 minterm out of 4
        let count = mgr.bdd_count_minterm(gt, 2);
        assert!(
            (count - 1.0).abs() < 1e-10,
            "1-bit x>y should have 1 minterm, got {}",
            count
        );

        assert!(mgr.bdd_eval(gt, &[true, false]));   // 1 > 0
        assert!(!mgr.bdd_eval(gt, &[false, true]));   // 0 > 1 = false
        assert!(!mgr.bdd_eval(gt, &[true, true]));     // 1 > 1 = false
        assert!(!mgr.bdd_eval(gt, &[false, false]));   // 0 > 0 = false
    }

    #[test]
    fn test_xgty_two_bits() {
        let mut mgr = Manager::new();
        for _ in 0..4 {
            mgr.bdd_new_var(); // vars 0..3
        }

        // x = (var0, var1), y = (var2, var3), MSB first
        let gt = mgr.bdd_xgty(&[0, 1], &[2, 3]);

        // Count: out of 16 assignments, x > y for 6 pairs:
        // (1,0),(1,1),(1,2),(1,3),(2,2),(2,3)... let's count properly:
        // x\y: 0 1 2 3
        //   0: N N N N
        //   1: Y N N N
        //   2: Y Y N N
        //   3: Y Y Y N
        // Total: 0+1+2+3 = 6 minterms
        let count = mgr.bdd_count_minterm(gt, 4);
        assert!(
            (count - 6.0).abs() < 1e-10,
            "2-bit x>y should have 6 minterms, got {}",
            count
        );

        // x=11 (3), y=01 (1) => 3 > 1 = true
        assert!(mgr.bdd_eval(gt, &[true, true, false, true]));
        // x=01 (1), y=10 (2) => 1 > 2 = false
        assert!(!mgr.bdd_eval(gt, &[false, true, true, false]));
        // x=10 (2), y=10 (2) => 2 > 2 = false
        assert!(!mgr.bdd_eval(gt, &[true, false, true, false]));
    }

    // ------------------------------------------------------------------
    // 10. bdd_adj_permute_x
    // ------------------------------------------------------------------

    #[test]
    fn test_adj_permute_basic() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var(); // var 0, level 0
        let y = mgr.bdd_new_var(); // var 1, level 1

        // f = x0 AND x1
        let f = mgr.bdd_and(x, y);

        // Swap var 0 (level 0) with var 1 (level 1)
        let g = mgr.bdd_adj_permute_x(f, 0);

        // Result should be x1 AND x0 = same function (AND is commutative)
        // but with variables swapped. The truth table is the same.
        for a in [false, true] {
            for b in [false, true] {
                let orig = mgr.bdd_eval(f, &[a, b]);
                // g has var0 and var1 swapped, so eval with swapped inputs
                let swapped = mgr.bdd_eval(g, &[b, a]);
                assert_eq!(orig, swapped, "a={}, b={}", a, b);
            }
        }
    }

    // ------------------------------------------------------------------
    // 11. bdd_li_compaction
    // ------------------------------------------------------------------

    #[test]
    fn test_li_compaction_full_care() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y);

        // Care set = ONE: result must equal f
        let result = mgr.bdd_li_compaction(f, NodeId::ONE);
        assert_eq!(result, f);
    }

    #[test]
    fn test_li_compaction_no_care() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let _y = mgr.bdd_new_var();

        // Care set = ZERO: result can be anything (we return ZERO)
        let result = mgr.bdd_li_compaction(x, NodeId::ZERO);
        assert!(result.is_zero());
    }

    #[test]
    fn test_li_compaction_agrees_on_care() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var(); // x0
        let y = mgr.bdd_new_var(); // x1

        // f = x0 XOR x1
        let f = mgr.bdd_xor(x, y);
        // care = x0 (only care about assignments where x0 = 1)
        let care = x;

        let result = mgr.bdd_li_compaction(f, care);

        // On the care set (x0=1), result must agree with f:
        // f(1, x1) = NOT(x1), so result(1, x1) should be NOT(x1)
        // Check: result AND care should equal f AND care
        let rc = mgr.bdd_and(result, care);
        let fc = mgr.bdd_and(f, care);
        assert_eq!(rc, fc, "result must agree with f on the care set");
    }

    #[test]
    fn test_li_compaction_f_eq_c() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();

        // f == c should return ONE
        let result = mgr.bdd_li_compaction(x, x);
        assert!(result.is_one());
    }

    #[test]
    fn test_li_compaction_f_eq_not_c() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();

        // f == NOT(c) should return ZERO
        let result = mgr.bdd_li_compaction(x, x.not());
        assert!(result.is_zero());
    }
}
