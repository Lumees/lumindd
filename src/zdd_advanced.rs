// lumindd — ZDD advanced operations: ISOP, cover manipulation, utilities
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::{HashMap, HashSet};

use crate::manager::Manager;
use crate::node::NodeId;

impl Manager {
    // ==================================================================
    // ISOP — Irredundant Sum of Products (Minato-Morreale algorithm)
    // ==================================================================

    /// Compute an Irredundant Sum of Products between lower and upper bounds.
    ///
    /// Given a lower bound `lower` (L) and an upper bound `upper` (U) such that
    /// L implies U (i.e., L AND !U = 0), this produces a pair `(zdd_cover, bdd_func)`
    /// where:
    /// - `zdd_cover` is a ZDD representing the irredundant set of cubes (cover)
    /// - `bdd_func` is the BDD of the function realized by that cover
    ///
    /// The cover C satisfies: L <= C <= U.
    ///
    /// This implements the Minato-Morreale ISOP algorithm which recursively
    /// decomposes the problem by Shannon expansion and builds both the BDD
    /// and ZDD representations simultaneously.
    pub fn zdd_isop(&mut self, lower: NodeId, upper: NodeId) -> (NodeId, NodeId) {
        // Terminal cases
        if lower.is_zero() {
            return (NodeId::ZERO, NodeId::ZERO);
        }
        if upper.is_one() {
            return (NodeId::ONE, NodeId::ONE);
        }
        if lower.is_one() {
            // lower = 1, upper != 0 (since lower implies upper)
            // The empty cube covers everything, return {∅} and TRUE
            return (NodeId::ONE, NodeId::ONE);
        }
        if upper.is_zero() {
            // Cannot satisfy: lower > 0 but upper = 0 (should not happen if L implies U)
            // Return empty cover
            return (NodeId::ZERO, NodeId::ZERO);
        }

        // Find the top variable across both lower and upper
        let l_level = self.level(lower.regular());
        let u_level = self.level(upper.regular());
        let top_level = l_level.min(u_level);
        let top_var = self.inv_perm[top_level as usize] as u16;

        // Shannon cofactors of lower and upper w.r.t. top_var
        let (l1, l0) = self.bdd_cofactors(lower, top_var);
        let (u1, u0) = self.bdd_cofactors(upper, top_var);

        // Recursive ISOP on the positive cofactor:
        // new_lower_pos = l1 AND NOT u0  (minterms that MUST use positive literal)
        let not_u0 = u0.not();
        let lower_pos = self.bdd_and(l1, not_u0);
        let (zdd_pos, bdd_pos) = self.zdd_isop(lower_pos, u1);

        // Recursive ISOP on the negative cofactor:
        // new_lower_neg = l0 AND NOT u1  (minterms that MUST use negative literal)
        let not_u1 = u1.not();
        let lower_neg = self.bdd_and(l0, not_u1);
        let (zdd_neg, bdd_neg) = self.zdd_isop(lower_neg, u0);

        // Recursive ISOP on the don't-care middle part:
        // remaining_lower = (l1 AND NOT bdd_pos) OR (l0 AND NOT bdd_neg)
        // But simplified: new_lower_dc = l1 OR l0 minus what's already covered
        let not_bdd_pos = bdd_pos.not();
        let not_bdd_neg = bdd_neg.not();
        let rem1 = self.bdd_and(l1, not_bdd_pos);
        let rem0 = self.bdd_and(l0, not_bdd_neg);
        let lower_dc = self.bdd_or(rem1, rem0);

        // Upper bound for DC part: cubes that are valid in both cofactors.
        // upper_dc = (u1 OR bdd_pos) AND (u0 OR bdd_neg)
        // This ensures any DC cube c satisfies: c <= u1 (when var=1) and c <= u0 (when var=0).
        let u1_or_pos = self.bdd_or(u1, bdd_pos);
        let u0_or_neg = self.bdd_or(u0, bdd_neg);
        let upper_dc = self.bdd_and(u1_or_pos, u0_or_neg);

        let (zdd_dc, bdd_dc) = self.zdd_isop(lower_dc, upper_dc);

        // Build the result BDD: ITE(top_var, bdd_pos OR bdd_dc, bdd_neg OR bdd_dc)
        let bdd_t = self.bdd_or(bdd_pos, bdd_dc);
        let bdd_e = self.bdd_or(bdd_neg, bdd_dc);
        let result_bdd = self.unique_inter(top_var, bdd_t, bdd_e);

        // Ensure ZDD variables exist
        while self.num_zdd_vars <= top_var {
            self.zdd_new_var();
        }

        // Build the result ZDD:
        // positive cubes get top_var added (zdd_change adds the variable to each set)
        let zdd_pos_with_var = self.zdd_change(zdd_pos, top_var);
        // negative cubes get complemented var — in ZDD cover representation,
        // we use 2 ZDD variables per BDD variable: var*2 for positive, var*2+1 for negative.
        // However, for simplicity and following Minato's approach, we build:
        // result = union(change(zdd_pos, top_var), zdd_neg, zdd_dc)
        // where zdd_pos cubes include top_var, zdd_neg and zdd_dc don't mention it.
        let zdd_pos_neg = self.zdd_union(zdd_pos_with_var, zdd_neg);
        let result_zdd = self.zdd_union(zdd_pos_neg, zdd_dc);

        (result_zdd, result_bdd)
    }

    // ==================================================================
    // BDD-to-ZDD cover conversion
    // ==================================================================

    /// Convert a BDD to a ZDD cover representation.
    ///
    /// The resulting ZDD represents the set of cubes in the on-set of the BDD.
    /// Each path to ONE in the BDD becomes a set in the ZDD family, where
    /// the set contains the variables that appear as positive literals in the cube.
    ///
    /// This is a simpler conversion than ISOP — it produces a (possibly redundant)
    /// cover by enumerating all prime paths.
    pub fn zdd_make_from_bdd_cover(&mut self, bdd: NodeId) -> NodeId {
        if bdd.is_zero() {
            return NodeId::ZERO;
        }
        if bdd.is_one() {
            // The tautology has one cube: the empty cube (no literals)
            return NodeId::ONE;
        }

        let mut cache: HashMap<u32, NodeId> = HashMap::new();
        self.zdd_from_bdd_cover_rec(bdd, &mut cache)
    }

    fn zdd_from_bdd_cover_rec(
        &mut self,
        f: NodeId,
        cache: &mut HashMap<u32, NodeId>,
    ) -> NodeId {
        if f.is_zero() {
            return NodeId::ZERO;
        }
        if f.is_one() {
            return NodeId::ONE;
        }

        // Use raw_index + complement bit as cache key
        let key = f.regular().raw_index() * 2 + f.is_complemented() as u32;
        if let Some(&cached) = cache.get(&key) {
            return cached;
        }

        let var = self.var_index(f.regular());
        let t = self.then_child(f);
        let e = self.else_child(f);

        // Ensure ZDD variable exists
        while self.num_zdd_vars <= var {
            self.zdd_new_var();
        }

        // Convert then-branch (positive literal cubes)
        let zdd_t = self.zdd_from_bdd_cover_rec(t, cache);
        // Convert else-branch (negative literal cubes — variable absent)
        let zdd_e = self.zdd_from_bdd_cover_rec(e, cache);

        // In the ZDD cover: then-branch cubes include `var`, else-branch cubes don't
        // ZDD node: if var is in the set, go to zdd_t; otherwise go to zdd_e
        let result = self.zdd_unique_inter(var, zdd_t, zdd_e);

        cache.insert(key, result);
        result
    }

    // ==================================================================
    // ZDD minterm counting
    // ==================================================================

    /// Count the total number of minterms covered by a ZDD family.
    ///
    /// Each set in the family is a cube (conjunction of literals). This counts
    /// the total number of minterms (complete variable assignments) covered by
    /// the union of all cubes, given `num_vars` total variables.
    ///
    /// For a set of size k in a universe of n variables, that cube covers
    /// 2^(n-k) minterms. This sums over all sets, but note that overlapping
    /// minterms are counted multiple times (this is the weighted count, not
    /// the exact onset size).
    pub fn zdd_count_minterm(&self, f: NodeId, num_vars: u16) -> f64 {
        let mut cache: HashMap<u32, f64> = HashMap::new();
        self.zdd_count_minterm_rec(f, num_vars, &mut cache)
    }

    fn zdd_count_minterm_rec(
        &self,
        f: NodeId,
        num_vars: u16,
        cache: &mut HashMap<u32, f64>,
    ) -> f64 {
        if f.is_zero() {
            return 0.0;
        }
        if f.is_one() {
            // The empty set represents the empty cube (tautology) covering 2^num_vars minterms
            return 2.0f64.powi(num_vars as i32);
        }

        let key = f.raw_index();
        if let Some(&cached) = cache.get(&key) {
            return cached;
        }

        let t = self.node(f).then_child();
        let e = self.node(f).else_child();

        // Then-child: cubes that include this variable (one fewer don't-care)
        let t_count = self.zdd_count_minterm_rec(t, num_vars.saturating_sub(1), cache);
        // Else-child: cubes that exclude this variable (one fewer don't-care)
        let e_count = self.zdd_count_minterm_rec(e, num_vars.saturating_sub(1), cache);

        let result = t_count + e_count;
        cache.insert(key, result);
        result
    }

    // ==================================================================
    // ZDD support
    // ==================================================================

    /// Get the set of variable indices that appear in any set of the ZDD family.
    ///
    /// This traverses the ZDD DAG and collects all variable indices encountered
    /// at internal nodes.
    pub fn zdd_support(&self, f: NodeId) -> Vec<u16> {
        let mut support = HashSet::new();
        self.zdd_support_rec(f, &mut support);
        let mut result: Vec<u16> = support.into_iter().collect();
        result.sort();
        result
    }

    fn zdd_support_rec(&self, f: NodeId, support: &mut HashSet<u16>) {
        self.zdd_support_rec_inner(f, support, &mut HashSet::new());
    }

    fn zdd_support_rec_inner(
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
        let var = self.var_index(f);
        support.insert(var);
        let t = self.node(f).then_child();
        let e = self.node(f).else_child();
        self.zdd_support_rec_inner(t, support, visited);
        self.zdd_support_rec_inner(e, support, visited);
    }

    // ==================================================================
    // ZDD DAG size
    // ==================================================================

    /// Count the number of nodes in the ZDD DAG rooted at `f`.
    ///
    /// This counts internal nodes plus terminal nodes reachable from `f`.
    pub fn zdd_dag_size(&self, f: NodeId) -> usize {
        let mut visited = HashSet::new();
        self.zdd_dag_size_rec(f, &mut visited);
        visited.len()
    }

    fn zdd_dag_size_rec(&self, f: NodeId, visited: &mut HashSet<u32>) {
        let raw = f.raw_index();
        if !visited.insert(raw) {
            return;
        }
        if f.is_constant() {
            return;
        }
        let t = self.node(f).then_child();
        let e = self.node(f).else_child();
        self.zdd_dag_size_rec(t, visited);
        self.zdd_dag_size_rec(e, visited);
    }

    // ==================================================================
    // ZDD cover printing
    // ==================================================================

    /// Print the ZDD as a sum-of-products cover.
    ///
    /// Each set in the ZDD family is printed as a product term. Variables
    /// present in the set appear as positive literals. The output format is:
    ///
    /// ```text
    /// x0 x2       (cube: x0 AND x2)
    /// x1          (cube: x1)
    /// <empty>     (the empty cube / tautology term)
    /// ```
    pub fn zdd_print_cover(&self, f: NodeId) {
        if f.is_zero() {
            println!("(empty cover — no cubes)");
            return;
        }
        let cubes = self.zdd_enumerate_sets(f);
        if cubes.is_empty() {
            println!("(empty cover — no cubes)");
            return;
        }
        for cube in &cubes {
            if cube.is_empty() {
                println!("1  (tautology cube)");
            } else {
                let terms: Vec<String> = cube.iter().map(|&v| format!("x{}", v)).collect();
                println!("{}", terms.join(" & "));
            }
        }
        println!("--- {} cube(s) ---", cubes.len());
    }

    /// Enumerate all sets in the ZDD family as vectors of variable indices.
    fn zdd_enumerate_sets(&self, f: NodeId) -> Vec<Vec<u16>> {
        let mut result = Vec::new();
        let mut current_set = Vec::new();
        self.zdd_enumerate_rec(f, &mut current_set, &mut result);
        result
    }

    fn zdd_enumerate_rec(
        &self,
        f: NodeId,
        current: &mut Vec<u16>,
        result: &mut Vec<Vec<u16>>,
    ) {
        if f.is_zero() {
            return;
        }
        if f.is_one() {
            result.push(current.clone());
            return;
        }
        let var = self.var_index(f);
        let t = self.node(f).then_child();
        let e = self.node(f).else_child();

        // Then-branch: include this variable
        current.push(var);
        self.zdd_enumerate_rec(t, current, result);
        current.pop();

        // Else-branch: exclude this variable
        self.zdd_enumerate_rec(e, current, result);
    }

    // ==================================================================
    // ZDD cardinality operations
    // ==================================================================

    /// Get the size of the largest set in the ZDD family.
    ///
    /// Returns 0 if the family is empty (ZDD is ZERO).
    pub fn zdd_max_cardinality(&self, f: NodeId) -> u32 {
        let mut cache: HashMap<u32, u32> = HashMap::new();
        self.zdd_max_card_rec(f, &mut cache)
    }

    fn zdd_max_card_rec(&self, f: NodeId, cache: &mut HashMap<u32, u32>) -> u32 {
        if f.is_zero() {
            return 0;
        }
        if f.is_one() {
            // The empty set has cardinality 0
            return 0;
        }

        let key = f.raw_index();
        if let Some(&cached) = cache.get(&key) {
            return cached;
        }

        let t = self.node(f).then_child();
        let e = self.node(f).else_child();

        // Then-branch: sets include this variable, so add 1
        let t_max = if t.is_zero() {
            0
        } else {
            1 + self.zdd_max_card_rec(t, cache)
        };
        let e_max = self.zdd_max_card_rec(e, cache);

        let result = t_max.max(e_max);
        cache.insert(key, result);
        result
    }

    /// Get the size of the smallest set in the ZDD family.
    ///
    /// Returns `u32::MAX` if the family is empty (ZDD is ZERO).
    pub fn zdd_min_cardinality(&self, f: NodeId) -> u32 {
        let mut cache: HashMap<u32, u32> = HashMap::new();
        self.zdd_min_card_rec(f, &mut cache)
    }

    fn zdd_min_card_rec(&self, f: NodeId, cache: &mut HashMap<u32, u32>) -> u32 {
        if f.is_zero() {
            return u32::MAX;
        }
        if f.is_one() {
            // The empty set has cardinality 0
            return 0;
        }

        let key = f.raw_index();
        if let Some(&cached) = cache.get(&key) {
            return cached;
        }

        let t = self.node(f).then_child();
        let e = self.node(f).else_child();

        // Then-branch: sets include this variable, so add 1
        let t_min = self.zdd_min_card_rec(t, cache).saturating_add(1);
        let e_min = self.zdd_min_card_rec(e, cache);

        let result = t_min.min(e_min);
        cache.insert(key, result);
        result
    }
}
