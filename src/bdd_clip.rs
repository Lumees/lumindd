// lumindd — BDD clipping (bounded operations)
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use crate::manager::Manager;
use crate::node::NodeId;

/// Direction for clipping incomplete branches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipDirection {
    /// Incomplete branches become ZERO (result implies exact).
    Under,
    /// Incomplete branches become ONE (exact implies result).
    Over,
}

impl Manager {
    /// Clipped AND: like AND but stops recursing at `max_depth`.
    ///
    /// If `direction` is `Under`, incomplete branches become ZERO
    /// (underapproximation — the result implies the exact answer).
    /// If `Over`, they become ONE (overapproximation — the exact answer
    /// implies the result).
    pub fn bdd_clip_and(
        &mut self,
        f: NodeId,
        g: NodeId,
        max_depth: u32,
        direction: ClipDirection,
    ) -> NodeId {
        self.clip_and_rec(f, g, max_depth, direction)
    }

    fn clip_and_rec(
        &mut self,
        f: NodeId,
        g: NodeId,
        depth: u32,
        direction: ClipDirection,
    ) -> NodeId {
        // Terminal cases (same as bdd_and)
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

        // Depth limit reached — return approximation
        if depth == 0 {
            return match direction {
                ClipDirection::Under => NodeId::ZERO,
                ClipDirection::Over => NodeId::ONE,
            };
        }

        // Find top variable
        let a_level = self.level(f);
        let b_level = self.level(g);
        let top_level = a_level.min(b_level);
        let top_var = self.inv_perm[top_level as usize] as u16;

        // Cofactor
        let (f_t, f_e) = self.bdd_cofactors(f, top_var);
        let (g_t, g_e) = self.bdd_cofactors(g, top_var);

        // Recurse with decremented depth
        let t = self.clip_and_rec(f_t, g_t, depth - 1, direction);
        let e = self.clip_and_rec(f_e, g_e, depth - 1, direction);

        if t == e {
            t
        } else {
            self.unique_inter(top_var, t, e)
        }
    }

    /// Clipped OR: like OR but stops recursing at `max_depth`.
    ///
    /// If `direction` is `Under`, incomplete branches become ZERO
    /// (underapproximation). If `Over`, they become ONE (overapproximation).
    pub fn bdd_clip_or(
        &mut self,
        f: NodeId,
        g: NodeId,
        max_depth: u32,
        direction: ClipDirection,
    ) -> NodeId {
        self.clip_or_rec(f, g, max_depth, direction)
    }

    fn clip_or_rec(
        &mut self,
        f: NodeId,
        g: NodeId,
        depth: u32,
        direction: ClipDirection,
    ) -> NodeId {
        // Terminal cases (same as bdd_or)
        if f.is_one() || g.is_one() {
            return NodeId::ONE;
        }
        if f.is_zero() {
            return g;
        }
        if g.is_zero() {
            return f;
        }
        if f == g {
            return f;
        }
        if f == g.not() {
            return NodeId::ONE;
        }

        // Depth limit reached — return approximation
        if depth == 0 {
            return match direction {
                ClipDirection::Under => NodeId::ZERO,
                ClipDirection::Over => NodeId::ONE,
            };
        }

        // Find top variable
        let a_level = self.level(f);
        let b_level = self.level(g);
        let top_level = a_level.min(b_level);
        let top_var = self.inv_perm[top_level as usize] as u16;

        // Cofactor
        let (f_t, f_e) = self.bdd_cofactors(f, top_var);
        let (g_t, g_e) = self.bdd_cofactors(g, top_var);

        // Recurse with decremented depth
        let t = self.clip_or_rec(f_t, g_t, depth - 1, direction);
        let e = self.clip_or_rec(f_e, g_e, depth - 1, direction);

        if t == e {
            t
        } else {
            self.unique_inter(top_var, t, e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ClipDirection;
    use crate::Manager;
    use crate::NodeId;

    #[test]
    fn clip_and_exact_with_large_depth() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        // With sufficient depth, clipped AND should equal exact AND
        let exact = mgr.bdd_and(x, y);
        let clipped = mgr.bdd_clip_and(x, y, 100, ClipDirection::Under);
        assert_eq!(exact, clipped);
    }

    #[test]
    fn clip_and_under_implies_exact() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let z = mgr.bdd_new_var();
        let f = mgr.bdd_or(x, y);
        let g = mgr.bdd_or(y, z);
        let exact = mgr.bdd_and(f, g);
        let under = mgr.bdd_clip_and(f, g, 1, ClipDirection::Under);
        // Under-approximation implies exact: under <= exact
        let check = mgr.bdd_leq(under, exact);
        assert!(check);
    }

    #[test]
    fn clip_and_over_implied_by_exact() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let z = mgr.bdd_new_var();
        let f = mgr.bdd_or(x, y);
        let g = mgr.bdd_or(y, z);
        let exact = mgr.bdd_and(f, g);
        let over = mgr.bdd_clip_and(f, g, 1, ClipDirection::Over);
        // Exact implies over-approximation: exact <= over
        let check = mgr.bdd_leq(exact, over);
        assert!(check);
    }

    #[test]
    fn clip_or_exact_with_large_depth() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let exact = mgr.bdd_or(x, y);
        let clipped = mgr.bdd_clip_or(x, y, 100, ClipDirection::Under);
        assert_eq!(exact, clipped);
    }

    #[test]
    fn clip_or_under_implies_exact() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let z = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y);
        let g = mgr.bdd_and(y, z);
        let exact = mgr.bdd_or(f, g);
        let under = mgr.bdd_clip_or(f, g, 1, ClipDirection::Under);
        let check = mgr.bdd_leq(under, exact);
        assert!(check);
    }

    #[test]
    fn clip_or_over_implied_by_exact() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let z = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y);
        let g = mgr.bdd_and(y, z);
        let exact = mgr.bdd_or(f, g);
        let over = mgr.bdd_clip_or(f, g, 1, ClipDirection::Over);
        let check = mgr.bdd_leq(exact, over);
        assert!(check);
    }

    #[test]
    fn clip_and_zero_depth() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let under = mgr.bdd_clip_and(x, y, 0, ClipDirection::Under);
        assert_eq!(under, NodeId::ZERO);
        let over = mgr.bdd_clip_and(x, y, 0, ClipDirection::Over);
        assert_eq!(over, NodeId::ONE);
    }

    #[test]
    fn clip_terminal_cases() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        // AND with ONE should return x regardless of depth
        let r = mgr.bdd_clip_and(x, mgr.one(), 0, ClipDirection::Under);
        assert_eq!(r, x);
        // OR with ZERO should return x regardless of depth
        let r = mgr.bdd_clip_or(x, mgr.zero(), 0, ClipDirection::Under);
        assert_eq!(r, x);
    }
}
