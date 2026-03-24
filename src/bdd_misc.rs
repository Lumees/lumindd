// lumindd — Miscellaneous BDD utility functions
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

//! Miscellaneous BDD operations: essential variable detection, closest cube,
//! largest cube, shortest path, density, random minterms, and cofactor ratio.

use std::collections::HashMap;

use crate::manager::Manager;
use crate::node::NodeId;

impl Manager {
    // ==================================================================
    // Essential variable
    // ==================================================================

    /// Check whether `var` is essential in `f`.
    ///
    /// A variable is *essential* if it appears on **every** path from the root
    /// to the ONE terminal. This is stronger than being in the support (which
    /// only requires appearing on *some* path).
    ///
    /// Equivalently, `var` is essential iff `f` differs from at least one of
    /// its cofactors and the other cofactor is identically ZERO:
    ///   essential(var) <=> (f_pos == ZERO) || (f_neg == ZERO)
    /// where f_pos = cofactor(f, var=1), f_neg = cofactor(f, var=0).
    ///
    /// If f_neg is ZERO, then every satisfying assignment must have var=1
    /// (so var appears on every root-to-1 path with polarity true).
    /// If f_pos is ZERO, then every satisfying assignment must have var=0.
    pub fn bdd_is_var_essential(&mut self, f: NodeId, var: u16) -> bool {
        if f.is_constant() {
            return false;
        }
        // Compute full cofactor (not just top-level): restrict f to var=1 and var=0
        let var_node = self.bdd_ith_var(var);
        let pos_cof = self.bdd_restrict(f, var_node);
        let neg_cof = self.bdd_restrict(f, var_node.not());
        pos_cof.is_zero() || neg_cof.is_zero()
    }

    // ==================================================================
    // Closest cube
    // ==================================================================

    /// Find the cube in `f` that is closest (minimum Hamming distance) to any
    /// cube in `g`. Returns `(cube, distance)`.
    ///
    /// Uses a BDD-based algorithm: XOR the two functions variable-by-variable,
    /// counting disagreements along the best path.
    pub fn bdd_closest_cube(&mut self, f: NodeId, g: NodeId) -> (NodeId, u32) {
        if f.is_zero() {
            // No cube in f — return (ZERO, MAX)
            return (NodeId::ZERO, u32::MAX);
        }
        if g.is_zero() {
            // No cube in g — any cube in f has infinite distance
            return (NodeId::ZERO, u32::MAX);
        }

        let mut dist_cache: HashMap<(u32, u32), (NodeId, u32)> = HashMap::new();
        self.closest_cube_rec(f, g, &mut dist_cache)
    }

    fn closest_cube_rec(
        &mut self,
        f: NodeId,
        g: NodeId,
        cache: &mut HashMap<(u32, u32), (NodeId, u32)>,
    ) -> (NodeId, u32) {
        // Terminal cases
        if f.is_zero() {
            return (NodeId::ZERO, u32::MAX);
        }
        if g.is_zero() {
            return (NodeId::ZERO, u32::MAX);
        }
        if f.is_one() && g.is_one() {
            return (NodeId::ONE, 0);
        }
        if f.is_one() {
            // f is tautology — every assignment satisfies f.
            // Distance is 0 since g has at least one satisfying assignment
            // and that assignment also satisfies f.
            return (NodeId::ONE, 0);
        }

        let f_key = f.regular().raw_index() | if f.is_complemented() { 1 << 31 } else { 0 };
        let g_key = g.regular().raw_index() | if g.is_complemented() { 1 << 31 } else { 0 };
        let key = (f_key, g_key);
        if let Some(&result) = cache.get(&key) {
            return result;
        }

        // Find top variable
        let f_level = self.level(f);
        let g_level = self.level(g);
        let top_level = f_level.min(g_level);
        let top_var = self.inv_perm[top_level as usize] as u16;

        let (f_t, f_e) = self.bdd_cofactors(f, top_var);
        let (g_t, g_e) = self.bdd_cofactors(g, top_var);

        // Try both branches and pick the one with smaller distance
        let (cube_t, dist_t) = self.closest_cube_rec(f_t, g_t, cache);
        let (cube_e, dist_e) = self.closest_cube_rec(f_e, g_e, cache);

        let (best_cube, best_dist, var_val) = if dist_t <= dist_e {
            (cube_t, dist_t, true)
        } else {
            (cube_e, dist_e, false)
        };

        // If the best is still MAX, try cross-cofactors (f_t with g_e and vice versa)
        // adding 1 to the distance for the disagreement on top_var
        let (cross_cube, cross_dist, cross_val) = {
            let (c_te, d_te) = self.closest_cube_rec(f_t, g_e, cache);
            let (c_et, d_et) = self.closest_cube_rec(f_e, g_t, cache);
            let d_te_adj = d_te.saturating_add(1);
            let d_et_adj = d_et.saturating_add(1);
            if d_te_adj <= d_et_adj {
                (c_te, d_te_adj, true)
            } else {
                (c_et, d_et_adj, false)
            }
        };

        let (final_cube, final_dist, final_val) = if best_dist <= cross_dist {
            (best_cube, best_dist, var_val)
        } else {
            (cross_cube, cross_dist, cross_val)
        };

        // Build result cube: top_var = final_val AND final_cube
        let result_cube = if final_cube.is_zero() || final_dist == u32::MAX {
            NodeId::ZERO
        } else {
            let var_node = self.bdd_ith_var(top_var);
            let lit = if final_val { var_node } else { var_node.not() };
            self.bdd_and(lit, final_cube)
        };

        let result = (result_cube, final_dist);
        cache.insert(key, result);
        result
    }

    // ==================================================================
    // Largest cube
    // ==================================================================

    /// Find the largest cube (fewest literals) implied by `f`.
    ///
    /// Returns `(cube, num_literals)`. The cube is a conjunction of literals
    /// that implies `f` and has the minimum number of literals among all such
    /// cubes (i.e., it covers the most minterms).
    pub fn bdd_largest_cube(&mut self, f: NodeId) -> (NodeId, u32) {
        if f.is_zero() {
            return (NodeId::ZERO, u32::MAX);
        }
        if f.is_one() {
            return (NodeId::ONE, 0);
        }
        let mut cache: HashMap<u32, (NodeId, u32)> = HashMap::new();
        self.largest_cube_rec(f, &mut cache)
    }

    fn largest_cube_rec(
        &mut self,
        f: NodeId,
        cache: &mut HashMap<u32, (NodeId, u32)>,
    ) -> (NodeId, u32) {
        if f.is_one() {
            return (NodeId::ONE, 0);
        }
        if f.is_zero() {
            return (NodeId::ZERO, u32::MAX);
        }

        let reg = f.regular();
        let key = reg.raw_index() | if f.is_complemented() { 1 << 31 } else { 0 };
        if let Some(&result) = cache.get(&key) {
            return result;
        }

        let var = self.var_index(reg);
        let t = self.then_child(f);
        let e = self.else_child(f);

        let (cube_t, len_t) = self.largest_cube_rec(t, cache);
        let (cube_e, len_e) = self.largest_cube_rec(e, cache);

        // Pick the branch with fewer literals
        let (best_cube, best_len, use_then) = if len_t <= len_e {
            (cube_t, len_t, true)
        } else {
            (cube_e, len_e, false)
        };

        // Build result: add the literal for this variable
        let result = if best_cube.is_zero() || best_len == u32::MAX {
            (NodeId::ZERO, u32::MAX)
        } else {
            let var_node = self.bdd_ith_var(var);
            let lit = if use_then { var_node } else { var_node.not() };
            let cube = self.bdd_and(lit, best_cube);
            (cube, best_len.saturating_add(1))
        };

        cache.insert(key, result);
        result
    }

    // ==================================================================
    // Shortest path
    // ==================================================================

    /// Find the shortest path from root to the ONE terminal.
    ///
    /// Returns the variable assignments along the path as `(var_index, value)`
    /// pairs. The path with the fewest decision nodes is chosen (BFS-like
    /// greedy: prefer branches that reach ONE sooner).
    pub fn bdd_shortest_path(&self, f: NodeId) -> Vec<(u16, bool)> {
        if f.is_zero() {
            return Vec::new();
        }
        if f.is_one() {
            return Vec::new();
        }

        let mut path = Vec::new();
        self.shortest_path_rec(f, &mut path);
        path
    }

    /// Recursive helper: greedily follow the branch that reaches ONE
    /// in fewer hops (measured by DAG depth to ONE).
    fn shortest_path_rec(&self, f: NodeId, path: &mut Vec<(u16, bool)>) {
        if f.is_constant() {
            return;
        }
        let var = self.var_index(f.regular());
        let t = self.then_child(f);
        let e = self.else_child(f);

        let t_depth = self.depth_to_one(t);
        let e_depth = self.depth_to_one(e);

        if t_depth <= e_depth {
            path.push((var, true));
            self.shortest_path_rec(t, path);
        } else {
            path.push((var, false));
            self.shortest_path_rec(e, path);
        }
    }

    /// Compute the minimum number of decision nodes from `f` to the ONE terminal.
    /// Returns u32::MAX if ONE is unreachable (i.e., f == ZERO).
    fn depth_to_one(&self, f: NodeId) -> u32 {
        if f.is_one() {
            return 0;
        }
        if f.is_zero() {
            return u32::MAX;
        }
        let t = self.then_child(f);
        let e = self.else_child(f);
        let t_d = self.depth_to_one(t);
        let e_d = self.depth_to_one(e);
        t_d.min(e_d).saturating_add(1)
    }

    // ==================================================================
    // Density
    // ==================================================================

    /// Fraction of minterms that are true: `count_minterm(f) / 2^num_vars`.
    ///
    /// Returns a value in `[0.0, 1.0]`.
    pub fn bdd_density(&self, f: NodeId, num_vars: u32) -> f64 {
        if f.is_zero() {
            return 0.0;
        }
        if f.is_one() {
            return 1.0;
        }
        let count = self.bdd_count_minterm(f, num_vars);
        let total = 2.0f64.powi(num_vars as i32);
        if total == 0.0 {
            0.0
        } else {
            count / total
        }
    }

    // ==================================================================
    // Random minterms
    // ==================================================================

    /// Generate `count` random satisfying assignments from `f`.
    ///
    /// Uses a deterministic PRNG seeded from the BDD structure.
    /// Walks down the BDD, choosing branches proportional to their
    /// minterm count (weighted random sampling).
    pub fn bdd_random_minterms(&self, f: NodeId, count: usize) -> Vec<Vec<bool>> {
        let mut results = Vec::with_capacity(count);
        if f.is_zero() || count == 0 {
            return results;
        }

        let n = self.num_vars as u32;
        // Build a minterm-count cache for weighted sampling
        let mut mt_cache: HashMap<u32, f64> = HashMap::new();

        // Seed PRNG deterministically from node structure
        let mut rng_state: u64 = 0xDEAD_BEEF_CAFE_BABEu64
            ^ (f.regular().raw_index() as u64)
            ^ ((count as u64) << 32);

        for _ in 0..count {
            let mut assignment = vec![false; n as usize];
            self.sample_minterm(f, n, &mut assignment, &mut rng_state, &mut mt_cache);
            results.push(assignment);
        }

        results
    }

    /// Sample one satisfying assignment by walking down the BDD,
    /// choosing branches proportionally to their minterm counts.
    fn sample_minterm(
        &self,
        f: NodeId,
        num_vars: u32,
        assignment: &mut Vec<bool>,
        rng: &mut u64,
        mt_cache: &mut HashMap<u32, f64>,
    ) {
        let mut current = f;
        let mut current_level = if current.is_constant() {
            num_vars
        } else {
            self.level(current.regular())
        };

        // Fill in variables above the root with random values
        for level in 0..current_level {
            let var = self.inv_perm[level as usize] as usize;
            if var < assignment.len() {
                assignment[var] = self.next_bool(rng);
            }
        }

        while !current.is_constant() {
            let var = self.var_index(current.regular());
            let t = self.then_child(current);
            let e = self.else_child(current);

            let t_count = self.cached_minterm_count(t, num_vars, mt_cache);
            let e_count = self.cached_minterm_count(e, num_vars, mt_cache);

            let total = t_count + e_count;
            let go_then = if total <= 0.0 {
                self.next_bool(rng)
            } else {
                let r = self.next_f64(rng);
                r < (t_count / total)
            };

            if go_then {
                assignment[var as usize] = true;
                current = t;
            } else {
                assignment[var as usize] = false;
                current = e;
            }

            // Fill in skipped variables with random values
            let next_level = if current.is_constant() {
                num_vars
            } else {
                self.level(current.regular())
            };
            let this_level = self.perm[var as usize];
            for level in (this_level + 1)..next_level {
                let skip_var = self.inv_perm[level as usize] as usize;
                if skip_var < assignment.len() {
                    assignment[skip_var] = self.next_bool(rng);
                }
            }

            current_level = next_level;
        }

        // Fill remaining variables below with random values
        for level in current_level..num_vars {
            let var = self.inv_perm[level as usize] as usize;
            if var < assignment.len() {
                assignment[var] = self.next_bool(rng);
            }
        }
    }

    /// Cached minterm count for sampling (accounts for skipped levels).
    fn cached_minterm_count(
        &self,
        f: NodeId,
        num_vars: u32,
        cache: &mut HashMap<u32, f64>,
    ) -> f64 {
        if f.is_one() {
            return 1.0;
        }
        if f.is_zero() {
            return 0.0;
        }
        let reg = f.regular();
        let key = reg.raw_index();
        if let Some(&val) = cache.get(&key) {
            return if f.is_complemented() {
                let level = self.level(reg);
                let total = 2.0f64.powi((num_vars - level) as i32);
                total - val
            } else {
                val
            };
        }

        let f_level = self.level(reg);
        let t = self.then_child(f);
        let e = self.else_child(f);

        let t_level = if t.is_constant() { num_vars } else { self.level(t.regular()) };
        let e_level = if e.is_constant() { num_vars } else { self.level(e.regular()) };

        let t_count = self.cached_minterm_count(t, num_vars, cache);
        let e_count = self.cached_minterm_count(e, num_vars, cache);

        let t_factor = 2.0f64.powi((t_level - f_level - 1) as i32);
        let e_factor = 2.0f64.powi((e_level - f_level - 1) as i32);

        let result = t_count * t_factor + e_count * e_factor;

        let regular_result = if f.is_complemented() {
            let total = 2.0f64.powi((num_vars - f_level) as i32);
            total - result
        } else {
            result
        };
        cache.insert(key, regular_result);

        result
    }

    /// Simple xorshift64 PRNG step.
    fn next_u64(&self, state: &mut u64) -> u64 {
        let mut s = *state;
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        *state = s;
        s
    }

    fn next_bool(&self, state: &mut u64) -> bool {
        self.next_u64(state) & 1 == 0
    }

    fn next_f64(&self, state: &mut u64) -> f64 {
        (self.next_u64(state) >> 11) as f64 / (1u64 << 53) as f64
    }

    // ==================================================================
    // Cofactor ratio
    // ==================================================================

    /// Return the density of the positive and negative cofactors of `f`
    /// with respect to `var`.
    ///
    /// Returns `(positive_cofactor_density, negative_cofactor_density)`.
    pub fn bdd_cofactor_ratio(&mut self, f: NodeId, var: u16, num_vars: u32) -> (f64, f64) {
        let (pos, neg) = self.bdd_cofactors(f, var);
        // Use num_vars for density since the cofactor BDD still lives in
        // the same variable space (the cofactored variable just becomes
        // a don't-care).
        let pos_density = self.bdd_density(pos, num_vars);
        let neg_density = self.bdd_density(neg, num_vars);
        (pos_density, neg_density)
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
    // Essential variable
    // ------------------------------------------------------------------

    #[test]
    fn test_essential_single_var() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var(); // x0
        // f = x0: positive cofactor is ONE, negative cofactor is ZERO
        // => x0 is essential (every sat assignment needs x0=1)
        assert!(mgr.bdd_is_var_essential(x, 0));
    }

    #[test]
    fn test_essential_and() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y); // f = x0 & x1
        // Both vars are essential: removing either makes f = 0 for that cofactor
        assert!(mgr.bdd_is_var_essential(f, 0));
        assert!(mgr.bdd_is_var_essential(f, 1));
    }

    #[test]
    fn test_not_essential_or() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_or(x, y); // f = x0 | x1
        // Neither is essential: f(x0=0) = x1 != 0, f(x0=1) = 1 != 0
        assert!(!mgr.bdd_is_var_essential(f, 0));
        assert!(!mgr.bdd_is_var_essential(f, 1));
    }

    #[test]
    fn test_essential_constant() {
        let mut mgr = Manager::new();
        assert!(!mgr.bdd_is_var_essential(NodeId::ONE, 0));
        assert!(!mgr.bdd_is_var_essential(NodeId::ZERO, 0));
    }

    // ------------------------------------------------------------------
    // Largest cube
    // ------------------------------------------------------------------

    #[test]
    fn test_largest_cube_single_var() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let (cube, len) = mgr.bdd_largest_cube(x);
        assert_eq!(len, 1);
        assert!(!cube.is_zero());
    }

    #[test]
    fn test_largest_cube_tautology() {
        let mut mgr = Manager::new();
        let _ = mgr.bdd_new_var();
        let (cube, len) = mgr.bdd_largest_cube(NodeId::ONE);
        assert_eq!(len, 0);
        assert!(cube.is_one());
    }

    #[test]
    fn test_largest_cube_or() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_or(x, y);
        let (_cube, len) = mgr.bdd_largest_cube(f);
        // The largest cube of (x|y) is a single literal (x or y), len = 1
        assert_eq!(len, 1);
    }

    #[test]
    fn test_largest_cube_zero() {
        let mut mgr = Manager::new();
        let _ = mgr.bdd_new_var();
        let (cube, len) = mgr.bdd_largest_cube(NodeId::ZERO);
        assert!(cube.is_zero());
        assert_eq!(len, u32::MAX);
    }

    // ------------------------------------------------------------------
    // Shortest path
    // ------------------------------------------------------------------

    #[test]
    fn test_shortest_path_single() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let path = mgr.bdd_shortest_path(x);
        assert_eq!(path.len(), 1);
        assert_eq!(path[0], (0, true));
    }

    #[test]
    fn test_shortest_path_and() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y);
        let path = mgr.bdd_shortest_path(f);
        // x AND y: shortest path to ONE goes through x=1, y=1
        assert_eq!(path.len(), 2);
    }

    #[test]
    fn test_shortest_path_constant() {
        let mgr = Manager::new();
        assert!(mgr.bdd_shortest_path(NodeId::ONE).is_empty());
        assert!(mgr.bdd_shortest_path(NodeId::ZERO).is_empty());
    }

    // ------------------------------------------------------------------
    // Density
    // ------------------------------------------------------------------

    #[test]
    fn test_density_tautology() {
        let mgr = Manager::new();
        assert!((mgr.bdd_density(NodeId::ONE, 3) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_density_zero() {
        let mgr = Manager::new();
        assert!((mgr.bdd_density(NodeId::ZERO, 3)).abs() < 1e-10);
    }

    #[test]
    fn test_density_single_var() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        // x0 is true for half the minterms
        let d = mgr.bdd_density(x, 1);
        assert!((d - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_density_and() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y);
        // x0 & x1: 1 minterm out of 4
        let d = mgr.bdd_density(f, 2);
        assert!((d - 0.25).abs() < 1e-10);
    }

    // ------------------------------------------------------------------
    // Random minterms
    // ------------------------------------------------------------------

    #[test]
    fn test_random_minterms_basic() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y);

        let samples = mgr.bdd_random_minterms(f, 10);
        assert_eq!(samples.len(), 10);
        for s in &samples {
            assert_eq!(s.len(), 2);
            // All samples must satisfy f = x0 & x1
            assert!(mgr.bdd_eval(f, s));
        }
    }

    #[test]
    fn test_random_minterms_or() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_or(x, y);

        let samples = mgr.bdd_random_minterms(f, 20);
        assert_eq!(samples.len(), 20);
        for s in &samples {
            assert!(mgr.bdd_eval(f, s));
        }
    }

    #[test]
    fn test_random_minterms_zero() {
        let mut mgr = Manager::new();
        let _ = mgr.bdd_new_var();
        let samples = mgr.bdd_random_minterms(NodeId::ZERO, 5);
        assert!(samples.is_empty());
    }

    #[test]
    fn test_random_minterms_tautology() {
        let mut mgr = Manager::new();
        let _ = mgr.bdd_new_var();
        let _ = mgr.bdd_new_var();
        let samples = mgr.bdd_random_minterms(NodeId::ONE, 10);
        assert_eq!(samples.len(), 10);
        for s in &samples {
            assert_eq!(s.len(), 2);
        }
    }

    // ------------------------------------------------------------------
    // Cofactor ratio
    // ------------------------------------------------------------------

    #[test]
    fn test_cofactor_ratio_single_var() {
        let mut mgr = Manager::new();
        let _x = mgr.bdd_new_var();
        // f = x0: cof_pos = ONE (density 1.0 over 0 vars = 1.0),
        //         cof_neg = ZERO (density 0.0)
        let (pos_d, neg_d) = mgr.bdd_cofactor_ratio(_x, 0, 1);
        assert!((pos_d - 1.0).abs() < 1e-10);
        assert!(neg_d.abs() < 1e-10);
    }

    #[test]
    fn test_cofactor_ratio_and() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y);
        // cofactor of (x&y) w.r.t. x: pos=y, neg=0
        // density of y over 1 var = 0.5, density of 0 = 0.0
        let (pos_d, neg_d) = mgr.bdd_cofactor_ratio(f, 0, 2);
        // pos cofactor of (x&y) w.r.t. x = y, density(y, 2) = 2/4 = 0.5
        assert!((pos_d - 0.5).abs() < 1e-10, "pos_d={}", pos_d);
        assert!(neg_d.abs() < 1e-10, "neg_d={}", neg_d);
    }

    // ------------------------------------------------------------------
    // Closest cube
    // ------------------------------------------------------------------

    #[test]
    fn test_closest_cube_same() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let (cube, dist) = mgr.bdd_closest_cube(x, x);
        assert_eq!(dist, 0);
        assert!(!cube.is_zero());
    }

    #[test]
    fn test_closest_cube_disjoint() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        // f = x, g = !x — closest cubes differ in exactly 1 variable
        let g = mgr.bdd_not(x);
        let (_cube, dist) = mgr.bdd_closest_cube(x, g);
        assert_eq!(dist, 1);
    }

    #[test]
    fn test_closest_cube_zero() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let (cube, dist) = mgr.bdd_closest_cube(NodeId::ZERO, x);
        assert!(cube.is_zero());
        assert_eq!(dist, u32::MAX);
    }
}
