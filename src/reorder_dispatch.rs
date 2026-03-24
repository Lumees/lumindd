// lumindd — Extended reordering method dispatch
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use crate::manager::Manager;

/// Extended reordering methods covering all 17+ CUDD algorithms.
///
/// This enum provides parity with CUDD's full set of `CUDD_REORDER_*`
/// constants. Each variant dispatches to the corresponding implementation
/// in the `reorder_*` modules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtReorderMethod {
    /// No reordering — a no-op.
    None,
    /// Rudell's sifting algorithm.
    Sift,
    /// Sifting iterated until convergence (no further improvement).
    SiftConverge,
    /// Symmetric sifting — exploits variable symmetry during sifting.
    SymmSift,
    /// Symmetric sifting iterated until convergence.
    SymmSiftConverge,
    /// Group sifting — sifts groups of variables together.
    GroupSift,
    /// Group sifting iterated until convergence.
    GroupSiftConverge,
    /// Window permutation of size 2.
    Window2,
    /// Window permutation of size 3.
    Window3,
    /// Window permutation of size 4.
    Window4,
    /// Window-2 iterated until convergence.
    Window2Converge,
    /// Window-3 iterated until convergence.
    Window3Converge,
    /// Window-4 iterated until convergence.
    Window4Converge,
    /// Linear sifting — combines sifting with XOR linear transforms.
    Linear,
    /// Linear sifting iterated until convergence.
    LinearConverge,
    /// Simulated annealing.
    Annealing,
    /// Genetic algorithm.
    Genetic,
    /// Exact reordering (exhaustive search — only feasible for small BDDs).
    Exact,
    /// Random variable permutation (useful for benchmarking).
    Random,
}

impl Manager {
    /// Trigger variable reordering using an extended method.
    ///
    /// This dispatches to the appropriate reordering implementation
    /// based on the [`ExtReorderMethod`] variant. It covers all
    /// algorithms available in the `reorder_*` modules and provides
    /// CUDD-complete coverage.
    ///
    /// After reordering the computed-table cache is cleared and the
    /// `reordered` flag is set.
    pub fn reduce_heap_ext(&mut self, method: ExtReorderMethod) {
        match method {
            ExtReorderMethod::None => {
                return; // No reordering, no cache clear needed.
            }
            ExtReorderMethod::Sift => {
                self.sift_reorder_ext(false);
            }
            ExtReorderMethod::SiftConverge => {
                self.sift_reorder_ext(true);
            }
            ExtReorderMethod::SymmSift => {
                self.symmetric_sift(false);
            }
            ExtReorderMethod::SymmSiftConverge => {
                self.symmetric_sift_converge();
            }
            ExtReorderMethod::GroupSift => {
                self.group_sift_dispatch(false);
            }
            ExtReorderMethod::GroupSiftConverge => {
                self.group_sift_dispatch(true);
            }
            ExtReorderMethod::Window2 => {
                self.window_reorder_ext(2);
            }
            ExtReorderMethod::Window3 => {
                self.window_reorder_ext(3);
            }
            ExtReorderMethod::Window4 => {
                self.window4_reorder();
            }
            ExtReorderMethod::Window2Converge => {
                self.window2_converge();
            }
            ExtReorderMethod::Window3Converge => {
                self.window3_converge();
            }
            ExtReorderMethod::Window4Converge => {
                self.window4_reorder_converge();
            }
            ExtReorderMethod::Linear => {
                let _ = self.linear_sift(false);
            }
            ExtReorderMethod::LinearConverge => {
                let _ = self.linear_sift_converge();
            }
            ExtReorderMethod::Annealing => {
                self.anneal_reorder();
            }
            ExtReorderMethod::Genetic => {
                self.genetic_reorder();
            }
            ExtReorderMethod::Exact => {
                self.exact_reorder();
            }
            ExtReorderMethod::Random => {
                self.random_reorder_ext();
            }
        }

        self.cache.clear();
        self.reordered = true;
    }

    // ------------------------------------------------------------------
    // Private dispatch helpers (thin wrappers around existing methods)
    // ------------------------------------------------------------------

    /// Dispatches sifting with optional convergence.
    fn sift_reorder_ext(&mut self, converge: bool) {
        use crate::reorder::ReorderingMethod;
        let method = if converge {
            ReorderingMethod::SiftConverge
        } else {
            ReorderingMethod::Sift
        };
        self.reduce_heap(method);
    }

    /// Dispatches window reordering for sizes 2 and 3.
    fn window_reorder_ext(&mut self, size: usize) {
        use crate::reorder::ReorderingMethod;
        let method = match size {
            2 => ReorderingMethod::Window2,
            3 => ReorderingMethod::Window3,
            _ => return,
        };
        self.reduce_heap(method);
    }

    /// Dispatches group sifting using the current group tree.
    fn group_sift_dispatch(&mut self, converge: bool) {
        let n = self.num_vars as usize;
        // Build default singleton groups — one group per variable.
        let specs: Vec<(usize, usize)> = (0..n).map(|i| (i, 1)).collect();
        let groups = self.make_var_groups(&specs);
        if converge {
            self.group_sift_converge(&groups);
        } else {
            self.group_sift(&groups, false);
        }
    }

    /// Dispatches random reordering via the base reduce_heap.
    fn random_reorder_ext(&mut self) {
        use crate::reorder::ReorderingMethod;
        self.reduce_heap(ReorderingMethod::Random);
    }
}

// ======================================================================
// Unit tests
// ======================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Manager;

    /// Helper: create a small manager with 4 BDD variables and a
    /// non-trivial function so reordering has something to work with.
    fn small_mgr() -> (Manager, crate::node::NodeId) {
        let mut mgr = Manager::new();
        let a = mgr.bdd_new_var();
        let b = mgr.bdd_new_var();
        let c = mgr.bdd_new_var();
        let d = mgr.bdd_new_var();

        // f = (a AND b) OR (c AND d)
        let ab = mgr.bdd_and(a, b);
        let cd = mgr.bdd_and(c, d);
        let f = mgr.bdd_or(ab, cd);
        mgr.ref_node(f);
        (mgr, f)
    }

    #[test]
    fn test_ext_none() {
        let (mut mgr, _f) = small_mgr();
        mgr.reduce_heap_ext(ExtReorderMethod::None);
        // No-op — should not panic.
    }

    #[test]
    fn test_ext_sift() {
        let (mut mgr, _f) = small_mgr();
        mgr.reduce_heap_ext(ExtReorderMethod::Sift);
        assert_eq!(mgr.read_size(), 4);
    }

    #[test]
    fn test_ext_sift_converge() {
        let (mut mgr, _f) = small_mgr();
        mgr.reduce_heap_ext(ExtReorderMethod::SiftConverge);
        assert_eq!(mgr.read_size(), 4);
    }

    #[test]
    fn test_ext_symm_sift() {
        let (mut mgr, _f) = small_mgr();
        mgr.reduce_heap_ext(ExtReorderMethod::SymmSift);
        assert_eq!(mgr.read_size(), 4);
    }

    #[test]
    fn test_ext_symm_sift_converge() {
        let (mut mgr, _f) = small_mgr();
        mgr.reduce_heap_ext(ExtReorderMethod::SymmSiftConverge);
        assert_eq!(mgr.read_size(), 4);
    }

    #[test]
    fn test_ext_group_sift() {
        let (mut mgr, _f) = small_mgr();
        mgr.reduce_heap_ext(ExtReorderMethod::GroupSift);
        assert_eq!(mgr.read_size(), 4);
    }

    #[test]
    fn test_ext_group_sift_converge() {
        let (mut mgr, _f) = small_mgr();
        mgr.reduce_heap_ext(ExtReorderMethod::GroupSiftConverge);
        assert_eq!(mgr.read_size(), 4);
    }

    #[test]
    fn test_ext_window2() {
        let (mut mgr, _f) = small_mgr();
        mgr.reduce_heap_ext(ExtReorderMethod::Window2);
        assert_eq!(mgr.read_size(), 4);
    }

    #[test]
    fn test_ext_window3() {
        let (mut mgr, _f) = small_mgr();
        mgr.reduce_heap_ext(ExtReorderMethod::Window3);
        assert_eq!(mgr.read_size(), 4);
    }

    #[test]
    fn test_ext_window4() {
        let (mut mgr, _f) = small_mgr();
        mgr.reduce_heap_ext(ExtReorderMethod::Window4);
        assert_eq!(mgr.read_size(), 4);
    }

    #[test]
    fn test_ext_window2_converge() {
        let (mut mgr, _f) = small_mgr();
        mgr.reduce_heap_ext(ExtReorderMethod::Window2Converge);
        assert_eq!(mgr.read_size(), 4);
    }

    #[test]
    fn test_ext_window3_converge() {
        let (mut mgr, _f) = small_mgr();
        mgr.reduce_heap_ext(ExtReorderMethod::Window3Converge);
        assert_eq!(mgr.read_size(), 4);
    }

    #[test]
    fn test_ext_window4_converge() {
        let (mut mgr, _f) = small_mgr();
        mgr.reduce_heap_ext(ExtReorderMethod::Window4Converge);
        assert_eq!(mgr.read_size(), 4);
    }

    #[test]
    fn test_ext_linear() {
        let (mut mgr, _f) = small_mgr();
        mgr.reduce_heap_ext(ExtReorderMethod::Linear);
        assert_eq!(mgr.read_size(), 4);
    }

    #[test]
    fn test_ext_linear_converge() {
        let (mut mgr, _f) = small_mgr();
        mgr.reduce_heap_ext(ExtReorderMethod::LinearConverge);
        assert_eq!(mgr.read_size(), 4);
    }

    #[test]
    fn test_ext_annealing() {
        let (mut mgr, _f) = small_mgr();
        mgr.reduce_heap_ext(ExtReorderMethod::Annealing);
        assert_eq!(mgr.read_size(), 4);
    }

    #[test]
    fn test_ext_genetic() {
        let (mut mgr, _f) = small_mgr();
        mgr.reduce_heap_ext(ExtReorderMethod::Genetic);
        assert_eq!(mgr.read_size(), 4);
    }

    #[test]
    fn test_ext_exact() {
        let (mut mgr, _f) = small_mgr();
        mgr.reduce_heap_ext(ExtReorderMethod::Exact);
        assert_eq!(mgr.read_size(), 4);
    }

    #[test]
    fn test_ext_random() {
        let (mut mgr, _f) = small_mgr();
        mgr.reduce_heap_ext(ExtReorderMethod::Random);
        assert_eq!(mgr.read_size(), 4);
    }

    #[test]
    fn test_reordered_flag_set() {
        let (mut mgr, _f) = small_mgr();
        mgr.reordered = false;
        mgr.reduce_heap_ext(ExtReorderMethod::Sift);
        assert!(mgr.reordered);
    }

    #[test]
    fn test_reordered_flag_not_set_for_none() {
        let (mut mgr, _f) = small_mgr();
        mgr.reordered = false;
        mgr.reduce_heap_ext(ExtReorderMethod::None);
        assert!(!mgr.reordered);
    }
}
