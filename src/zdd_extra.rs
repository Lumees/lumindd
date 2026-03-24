// lumindd — Additional ZDD operations: complement, strong division, unate product
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::HashMap;

use crate::manager::Manager;
use crate::node::NodeId;

/// Local cache key for ZDD extra operations.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct ZddExtraCacheKey {
    kind: u8,
    f: u32,
    g: u32,
}

impl Manager {
    // ==================================================================
    // ZDD Universe
    // ==================================================================

    /// Build the ZDD representing all 2^n subsets of {0, 1, ..., num_vars-1}.
    ///
    /// The universe ZDD is a chain of nodes where each variable's then-child
    /// and else-child both point to the next variable's node, culminating in
    /// the ONE terminal. This encodes every possible subset.
    pub fn zdd_universe(&mut self, num_vars: u16) -> NodeId {
        // Ensure enough ZDD variables exist
        while self.num_zdd_vars < num_vars {
            self.zdd_new_var();
        }

        // Build bottom-up: start from ONE, then for each variable from
        // highest index down to 0, create a node with both children
        // pointing to the accumulator.
        let mut result = NodeId::ONE;
        // We must build in reverse level order (highest level = last variable first)
        // so that each new node is above the previous one.
        // Variables are ordered by their ZDD level. We iterate from the
        // highest-level variable down to level 0.
        for level in (0..num_vars as u32).rev() {
            let var = self.zdd_inv_perm[level as usize] as u16;
            // Both then and else children point to the same sub-universe.
            // ZDD node: then=result (subsets containing var), else=result (subsets without var)
            result = self.zdd_unique_inter(var, result, result);
        }
        result
    }

    // ==================================================================
    // ZDD Complement
    // ==================================================================

    /// ZDD complement: the family of all subsets of {0, ..., num_vars-1}
    /// that are NOT in `f`.
    ///
    /// Computed as Universe(num_vars) \ f.
    pub fn zdd_complement(&mut self, f: NodeId, num_vars: u16) -> NodeId {
        let universe = self.zdd_universe(num_vars);
        self.zdd_diff(universe, f)
    }

    // ==================================================================
    // ZDD Strong Division
    // ==================================================================

    /// Strong division: f / g where every element of the quotient, when
    /// unioned with every element of g, yields a subset that is in f.
    ///
    /// More precisely, q = strong_div(f, g) is the largest family such
    /// that q × g ⊆ f, where × is the ZDD product (cross product with union).
    ///
    /// Algorithm: intersect weak divisions of f by each singleton element of g.
    /// For efficiency we use recursive decomposition:
    /// - If g = {∅}, return f
    /// - If g has top variable v:
    ///   - g_t = subsets of g containing v (with v removed)
    ///   - g_e = subsets of g not containing v
    ///   - strong_div(f, g) = strong_div(subset1(f, v), g_t) ∩ strong_div(subset0(f, v), g_e)
    ///     when both g_t and g_e are non-empty
    ///   - When g_e is empty: strong_div(f, g) = weak_div of f_t by g_t
    ///   - When g_t is empty: strong_div(f, g) = weak_div of f_e by g_e
    pub fn zdd_strong_div(&mut self, f: NodeId, g: NodeId) -> NodeId {
        let mut cache = HashMap::new();
        self.zdd_strong_div_rec(f, g, &mut cache)
    }

    fn zdd_strong_div_rec(
        &mut self,
        f: NodeId,
        g: NodeId,
        cache: &mut HashMap<ZddExtraCacheKey, NodeId>,
    ) -> NodeId {
        // Terminal cases
        if f.is_zero() {
            return NodeId::ZERO;
        }
        if g.is_zero() {
            return NodeId::ZERO;
        }
        if g.is_one() {
            // g = {∅}: every set in f, when unioned with ∅, is itself.
            // So quotient is f itself.
            return f;
        }
        if f == g {
            // f / f = {∅}
            return NodeId::ONE;
        }

        let key = ZddExtraCacheKey {
            kind: 0,
            f: f.raw_index(),
            g: g.raw_index(),
        };
        if let Some(&result) = cache.get(&key) {
            return result;
        }

        let f_level = self.zdd_level(f);
        let g_level = self.zdd_level(g);

        let result = if f_level < g_level {
            // f has a variable above g — elements of f containing this var
            // cannot come from g, so only the else-branch matters
            let f_e = self.node(f).else_child();
            self.zdd_strong_div_rec(f_e, g, cache)
        } else if f_level > g_level {
            // g has a variable above f — the divisor references a variable
            // not in f; strong division yields empty
            let g_t = self.node(g).then_child();
            let g_e = self.node(g).else_child();
            if g_t.is_zero() {
                // g only has subsets without this var
                self.zdd_strong_div_rec(f, g_e, cache)
            } else {
                // g has subsets containing a var not in f — no quotient possible
                NodeId::ZERO
            }
        } else {
            // Same top variable
            let f_t = self.node(f).then_child();
            let f_e = self.node(f).else_child();
            let g_t = self.node(g).then_child();
            let g_e = self.node(g).else_child();

            if g_e.is_zero() {
                // g only has sets containing this variable
                self.zdd_strong_div_rec(f_t, g_t, cache)
            } else if g_t.is_zero() {
                // g only has sets not containing this variable
                self.zdd_strong_div_rec(f_e, g_e, cache)
            } else {
                // Both branches of g are non-empty — intersect the two quotients
                let q_t = self.zdd_strong_div_rec(f_t, g_t, cache);
                let q_e = self.zdd_strong_div_rec(f_e, g_e, cache);
                self.zdd_intersect(q_t, q_e)
            }
        };

        cache.insert(key, result);
        result
    }

    // ==================================================================
    // ZDD Unate Product
    // ==================================================================

    /// Unate product: product of two ZDD families whose variables are disjoint.
    ///
    /// This is equivalent to the standard ZDD product but is more efficient
    /// because there is no overlap in variables. Each set in the result is
    /// the union of one set from `f` and one set from `g`.
    ///
    /// Precondition: the variables appearing in `f` and `g` are disjoint.
    /// (Not checked — if violated, the result may be incorrect.)
    pub fn zdd_unate_product(&mut self, f: NodeId, g: NodeId) -> NodeId {
        let mut cache = HashMap::new();
        self.zdd_unate_product_rec(f, g, &mut cache)
    }

    fn zdd_unate_product_rec(
        &mut self,
        f: NodeId,
        g: NodeId,
        cache: &mut HashMap<ZddExtraCacheKey, NodeId>,
    ) -> NodeId {
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

        let key = ZddExtraCacheKey {
            kind: 1,
            f: f.raw_index(),
            g: g.raw_index(),
        };
        if let Some(&result) = cache.get(&key) {
            return result;
        }

        let f_level = self.zdd_level(f);
        let g_level = self.zdd_level(g);

        let result = if f_level < g_level {
            // f's top variable is above g's — since variables are disjoint,
            // g does not mention this variable
            let f_var = self.var_index(f);
            let f_t = self.node(f).then_child();
            let f_e = self.node(f).else_child();
            let t = self.zdd_unate_product_rec(f_t, g, cache);
            let e = self.zdd_unate_product_rec(f_e, g, cache);
            self.zdd_unique_inter(f_var, t, e)
        } else {
            // g's top variable is above (or equal, but shouldn't happen for disjoint)
            let g_var = self.var_index(g);
            let g_t = self.node(g).then_child();
            let g_e = self.node(g).else_child();
            let t = self.zdd_unate_product_rec(f, g_t, cache);
            let e = self.zdd_unate_product_rec(f, g_e, cache);
            self.zdd_unique_inter(g_var, t, e)
        };

        cache.insert(key, result);
        result
    }

    // ==================================================================
    // ZDD Dot Product
    // ==================================================================

    /// Dot product: the union of all pairwise intersections of sets from `f`
    /// and sets from `g`.
    ///
    /// dot_product(f, g) = ⋃ { s ∩ t | s ∈ f, t ∈ g }
    ///
    /// This can also be seen as the "intersection product" — for each pair
    /// of sets (one from f, one from g), compute their intersection, and
    /// collect all resulting sets into a family.
    pub fn zdd_dot_product(&mut self, f: NodeId, g: NodeId) -> NodeId {
        let mut cache = HashMap::new();
        self.zdd_dot_product_rec(f, g, &mut cache)
    }

    fn zdd_dot_product_rec(
        &mut self,
        f: NodeId,
        g: NodeId,
        cache: &mut HashMap<ZddExtraCacheKey, NodeId>,
    ) -> NodeId {
        // Terminal cases
        if f.is_zero() || g.is_zero() {
            return NodeId::ZERO;
        }
        if f.is_one() {
            // {∅} dot anything: ∅ ∩ s = ∅ for all s, so result is {∅}
            // (as long as g is non-empty, which we already checked)
            return NodeId::ONE;
        }
        if g.is_one() {
            return NodeId::ONE;
        }
        if f == g {
            // Each set intersected with itself is itself
            return f;
        }

        // Normalize for caching
        let (a, b) = if f.raw_index() > g.raw_index() {
            (g, f)
        } else {
            (f, g)
        };

        let key = ZddExtraCacheKey {
            kind: 2,
            f: a.raw_index(),
            g: b.raw_index(),
        };
        if let Some(&result) = cache.get(&key) {
            return result;
        }

        let a_level = self.zdd_level(a);
        let b_level = self.zdd_level(b);

        let result = if a_level < b_level {
            // Variable in a but not at top of b
            let _a_var = self.var_index(a);
            let a_t = self.node(a).then_child();
            let a_e = self.node(a).else_child();
            // Sets from b don't have this variable at this level
            // then-branch of a: these sets contain the var, but b's sets
            // at this level go to else-child → intersection removes the var
            let t = self.zdd_dot_product_rec(a_t, b, cache);
            let e = self.zdd_dot_product_rec(a_e, b, cache);
            // The var is only in the intersection if both sets contain it.
            // Since b doesn't branch on this var, b's sets don't contain it.
            // So intersections of a_t sets with b sets won't contain this var.
            self.zdd_union(t, e)
        } else if a_level > b_level {
            let _b_var = self.var_index(b);
            let b_t = self.node(b).then_child();
            let b_e = self.node(b).else_child();
            let t = self.zdd_dot_product_rec(a, b_t, cache);
            let e = self.zdd_dot_product_rec(a, b_e, cache);
            self.zdd_union(t, e)
        } else {
            // Same top variable
            let a_var = self.var_index(a);
            let a_t = self.node(a).then_child();
            let a_e = self.node(a).else_child();
            let b_t = self.node(b).then_child();
            let b_e = self.node(b).else_child();

            // Intersection keeps the variable only when both sets have it
            let tt = self.zdd_dot_product_rec(a_t, b_t, cache); // both have var
            let te = self.zdd_dot_product_rec(a_t, b_e, cache); // only a has var
            let et = self.zdd_dot_product_rec(a_e, b_t, cache); // only b has var
            let ee = self.zdd_dot_product_rec(a_e, b_e, cache); // neither has var

            // tt: intersection includes the variable (both had it)
            // te, et, ee: intersection does not include the variable
            let without_var = self.zdd_union(te, et);
            let without_var = self.zdd_union(without_var, ee);
            self.zdd_unique_inter(a_var, tt, without_var)
        };

        cache.insert(key, result);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zdd_universe_0_vars() {
        let mut mgr = Manager::new();
        let u = mgr.zdd_universe(0);
        // 2^0 = 1 subset: the empty set
        assert!(u.is_one());
        assert_eq!(mgr.zdd_count(u), 1);
    }

    #[test]
    fn test_zdd_universe_1_var() {
        let mut mgr = Manager::new();
        let u = mgr.zdd_universe(1);
        // 2^1 = 2 subsets: {}, {0}
        assert_eq!(mgr.zdd_count(u), 2);
    }

    #[test]
    fn test_zdd_universe_2_vars() {
        let mut mgr = Manager::new();
        let u = mgr.zdd_universe(2);
        // 2^2 = 4 subsets: {}, {0}, {1}, {0,1}
        assert_eq!(mgr.zdd_count(u), 4);
    }

    #[test]
    fn test_zdd_universe_3_vars() {
        let mut mgr = Manager::new();
        let u = mgr.zdd_universe(3);
        assert_eq!(mgr.zdd_count(u), 8);
    }

    #[test]
    fn test_zdd_complement_empty() {
        let mut mgr = Manager::new();
        // Complement of empty family = universe
        let c = mgr.zdd_complement(NodeId::ZERO, 2);
        assert_eq!(mgr.zdd_count(c), 4);
    }

    #[test]
    fn test_zdd_complement_universe() {
        let mut mgr = Manager::new();
        let u = mgr.zdd_universe(2);
        // Complement of universe = empty
        let c = mgr.zdd_complement(u, 2);
        assert!(c.is_zero());
    }

    #[test]
    fn test_zdd_complement_single() {
        let mut mgr = Manager::new();
        // Family containing just the empty set: {∅}
        let f = NodeId::ONE;
        let c = mgr.zdd_complement(f, 2);
        // Should have 4 - 1 = 3 sets
        assert_eq!(mgr.zdd_count(c), 3);
    }

    #[test]
    fn test_zdd_complement_involution() {
        let mut mgr = Manager::new();
        let _v0 = mgr.zdd_new_var();
        let v1 = mgr.zdd_new_var();
        // Family = {{1}} (just the singleton {1})
        let f = v1;
        let c = mgr.zdd_complement(f, 2);
        let cc = mgr.zdd_complement(c, 2);
        assert_eq!(mgr.zdd_count(cc), mgr.zdd_count(f));
    }

    #[test]
    fn test_zdd_strong_div_by_one() {
        let mut mgr = Manager::new();
        let v0 = mgr.zdd_new_var();
        // f / {∅} = f
        let result = mgr.zdd_strong_div(v0, NodeId::ONE);
        assert_eq!(result, v0);
    }

    #[test]
    fn test_zdd_strong_div_self() {
        let mut mgr = Manager::new();
        let v0 = mgr.zdd_new_var();
        // {{0}} / {{0}} = {∅}
        let result = mgr.zdd_strong_div(v0, v0);
        assert!(result.is_one());
    }

    #[test]
    fn test_zdd_strong_div_basic() {
        let mut mgr = Manager::new();
        let v0 = mgr.zdd_new_var(); // {{0}}
        let v1 = mgr.zdd_new_var(); // {{1}}

        // f = {{0,1}} = product of v0 and v1
        let f = mgr.zdd_product(v0, v1);
        // g = {{0}}
        // f / g should be {{1}} (since {1} ∪ {0} = {0,1} ∈ f)
        let result = mgr.zdd_strong_div(f, v0);
        assert_eq!(mgr.zdd_count(result), 1);
    }

    #[test]
    fn test_zdd_unate_product_trivial() {
        let mut mgr = Manager::new();
        let v0 = mgr.zdd_new_var(); // {{0}}
        // {∅} × {{0}} = {{0}}
        let result = mgr.zdd_unate_product(NodeId::ONE, v0);
        assert_eq!(result, v0);
    }

    #[test]
    fn test_zdd_unate_product_disjoint() {
        let mut mgr = Manager::new();
        let v0 = mgr.zdd_new_var(); // {{0}}
        let v1 = mgr.zdd_new_var(); // {{1}}

        // {{0}} × {{1}} = {{0,1}}
        let result = mgr.zdd_unate_product(v0, v1);
        assert_eq!(mgr.zdd_count(result), 1);
        // The single set should have 2 elements
        assert_eq!(mgr.zdd_max_cardinality(result), 2);
    }

    #[test]
    fn test_zdd_unate_product_multiple() {
        let mut mgr = Manager::new();
        let v0 = mgr.zdd_new_var(); // {{0}}
        let v1 = mgr.zdd_new_var(); // {{1}}
        let v2 = mgr.zdd_new_var(); // {{2}}

        // f = {{0}, {1}} (union of v0 and v1)
        let f = mgr.zdd_union(v0, v1);
        // g = {{2}}
        // f × g = {{0,2}, {1,2}}
        let result = mgr.zdd_unate_product(f, v2);
        assert_eq!(mgr.zdd_count(result), 2);
    }

    #[test]
    fn test_zdd_dot_product_trivial() {
        let mut mgr = Manager::new();
        let v0 = mgr.zdd_new_var();
        // {∅} dot anything = {∅}
        let result = mgr.zdd_dot_product(NodeId::ONE, v0);
        assert!(result.is_one());
        assert_eq!(mgr.zdd_count(result), 1);
    }

    #[test]
    fn test_zdd_dot_product_self() {
        let mut mgr = Manager::new();
        let v0 = mgr.zdd_new_var(); // {{0}}
        // {{0}} dot {{0}} = {{0}} (intersection of {0} with {0} = {0})
        let result = mgr.zdd_dot_product(v0, v0);
        assert_eq!(result, v0);
    }

    #[test]
    fn test_zdd_dot_product_disjoint() {
        let mut mgr = Manager::new();
        let v0 = mgr.zdd_new_var(); // {{0}}
        let v1 = mgr.zdd_new_var(); // {{1}}

        // {{0}} dot {{1}} = {∅} ({0} ∩ {1} = ∅)
        let result = mgr.zdd_dot_product(v0, v1);
        // The result should be {∅} since the intersection of {0} and {1} is ∅
        assert_eq!(mgr.zdd_count(result), 1);
        assert!(result.is_one());
    }

    #[test]
    fn test_zdd_dot_product_overlap() {
        let mut mgr = Manager::new();
        let v0 = mgr.zdd_new_var(); // {{0}}
        let v1 = mgr.zdd_new_var(); // {{1}}

        // f = {{0, 1}}
        let f = mgr.zdd_product(v0, v1);
        // g = {{0}}
        // dot(f, g) = {{0} ∩ {0,1}} = {{0}}
        let result = mgr.zdd_dot_product(f, v0);
        assert_eq!(mgr.zdd_count(result), 1);
    }
}
