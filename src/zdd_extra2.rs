// lumindd — Additional ZDD operations for CUDD parity
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::HashMap;

use crate::manager::Manager;
use crate::node::NodeId;

impl Manager {
    // ==================================================================
    // ZDD variables from BDD variables
    // ==================================================================

    /// Create ZDD variables from existing BDD variables.
    ///
    /// For each BDD variable, creates `multiplicity` ZDD variables.
    /// This aligns BDD and ZDD variable spaces, which is important
    /// when converting between BDD and ZDD representations with
    /// multiplicity (e.g., for encoding multi-valued variables).
    ///
    /// After calling this with multiplicity=1, the ZDD variable space
    /// mirrors the BDD variable space exactly.
    pub fn zdd_vars_from_bdd_vars(&mut self, multiplicity: u16) {
        assert!(multiplicity > 0, "multiplicity must be at least 1");
        let num_bdd = self.num_vars;
        for _bdd_var in 0..num_bdd {
            for _m in 0..multiplicity {
                if self.num_zdd_vars < num_bdd * multiplicity {
                    self.zdd_new_var();
                }
            }
        }
    }

    // ==================================================================
    // ZDD count (f64 version)
    // ==================================================================

    /// Count the number of sets in a ZDD family, returning f64.
    ///
    /// This is identical to `zdd_count` but returns `f64` to support
    /// very large counts that may exceed `u64` range (using floating
    /// point approximation for extremely large families).
    pub fn zdd_count_double(&self, f: NodeId) -> f64 {
        let mut cache: HashMap<u32, f64> = HashMap::new();
        self.zdd_count_double_rec(f, &mut cache)
    }

    fn zdd_count_double_rec(&self, f: NodeId, cache: &mut HashMap<u32, f64>) -> f64 {
        if f.is_zero() {
            return 0.0;
        }
        if f.is_one() {
            return 1.0;
        }
        let key = f.raw_index();
        if let Some(&cached) = cache.get(&key) {
            return cached;
        }
        let t = self.node(f).then_child();
        let e = self.node(f).else_child();
        let result = self.zdd_count_double_rec(t, cache) + self.zdd_count_double_rec(e, cache);
        cache.insert(key, result);
        result
    }

    // ==================================================================
    // ZDD print minterm
    // ==================================================================

    /// Print each set in the ZDD family as a minterm.
    ///
    /// Each line represents one set in the family. Variables present in
    /// the set are shown as `1`, absent variables as `0`. The output
    /// covers all ZDD variables from 0 to `num_zdd_vars - 1`.
    pub fn zdd_print_minterm(&self, f: NodeId) {
        if f.is_zero() {
            println!("(empty family)");
            return;
        }
        let n = self.num_zdd_vars as usize;
        let mut path = vec![false; n];
        self.zdd_print_minterm_rec(f, &mut path);
    }

    fn zdd_print_minterm_rec(&self, f: NodeId, path: &mut Vec<bool>) {
        if f.is_zero() {
            return;
        }
        if f.is_one() {
            // Print the current path as a minterm
            let s: String = path
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    if v {
                        format!("z{}=1 ", i)
                    } else {
                        format!("z{}=0 ", i)
                    }
                })
                .collect();
            println!("{}", s.trim());
            return;
        }

        let var = self.var_index(f) as usize;
        let t = self.node(f).then_child();
        let e = self.node(f).else_child();

        // Then branch: variable is in the set
        if var < path.len() {
            path[var] = true;
        }
        self.zdd_print_minterm_rec(t, path);

        // Else branch: variable is not in the set
        if var < path.len() {
            path[var] = false;
        }
        self.zdd_print_minterm_rec(e, path);
    }

    // ==================================================================
    // ZDD ith var
    // ==================================================================

    /// Get or create the i-th ZDD variable.
    ///
    /// If ZDD variable `i` does not yet exist, creates all ZDD variables
    /// up to and including `i`. Returns the ZDD node representing the
    /// family `{{i}}` (the singleton set containing only variable `i`).
    pub fn zdd_ith_var(&mut self, i: u16) -> NodeId {
        // Ensure enough ZDD variables exist
        while self.num_zdd_vars <= i {
            self.zdd_new_var();
        }
        // Build the ZDD for {{i}}: then=ONE, else=ZERO at variable i
        self.zdd_unique_inter(i, NodeId::ONE, NodeId::ZERO)
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
    fn test_zdd_vars_from_bdd_vars_multiplicity_1() {
        let mut mgr = Manager::new();
        mgr.bdd_new_var();
        mgr.bdd_new_var();
        mgr.bdd_new_var();
        assert_eq!(mgr.num_vars(), 3);
        assert_eq!(mgr.num_zdd_vars(), 0);

        mgr.zdd_vars_from_bdd_vars(1);
        assert_eq!(mgr.num_zdd_vars(), 3);
    }

    #[test]
    fn test_zdd_vars_from_bdd_vars_multiplicity_2() {
        let mut mgr = Manager::new();
        mgr.bdd_new_var();
        mgr.bdd_new_var();
        assert_eq!(mgr.num_vars(), 2);

        mgr.zdd_vars_from_bdd_vars(2);
        assert_eq!(mgr.num_zdd_vars(), 4);
    }

    #[test]
    fn test_zdd_count_double() {
        let mut mgr = Manager::new();
        let z0 = mgr.zdd_new_var();
        let z1 = mgr.zdd_new_var();

        // {z0} union {z1} = {{0}, {1}}
        let s0 = z0; // {{0}}
        let s1 = z1; // {{1}}
        let family = mgr.zdd_union(s0, s1);

        let count = mgr.zdd_count_double(family);
        assert!((count - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_zdd_count_double_empty() {
        let mgr = Manager::new();
        let count = mgr.zdd_count_double(NodeId::ZERO);
        assert!((count - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_zdd_count_double_singleton_empty_set() {
        let mgr = Manager::new();
        // ONE represents {empty_set}
        let count = mgr.zdd_count_double(NodeId::ONE);
        assert!((count - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_zdd_ith_var() {
        let mut mgr = Manager::new();

        // Creating ZDD var 2 should also create vars 0 and 1
        let z2 = mgr.zdd_ith_var(2);
        assert_eq!(mgr.num_zdd_vars(), 3);

        // z2 represents {{2}}
        let count = mgr.zdd_count(z2);
        assert_eq!(count, 1);

        // Creating var 0 when it already exists
        let z0 = mgr.zdd_ith_var(0);
        assert_eq!(mgr.num_zdd_vars(), 3); // no new vars created
        let count0 = mgr.zdd_count(z0);
        assert_eq!(count0, 1);
    }

    #[test]
    fn test_zdd_ith_var_union() {
        let mut mgr = Manager::new();
        let z0 = mgr.zdd_ith_var(0);
        let z1 = mgr.zdd_ith_var(1);

        let family = mgr.zdd_union(z0, z1);
        let count = mgr.zdd_count(family);
        assert_eq!(count, 2); // {{0}, {1}}
    }

    #[test]
    fn test_zdd_print_minterm_does_not_panic() {
        let mut mgr = Manager::new();
        let z0 = mgr.zdd_ith_var(0);
        let z1 = mgr.zdd_ith_var(1);
        let family = mgr.zdd_union(z0, z1);

        // Just verify it doesn't panic
        mgr.zdd_print_minterm(family);
        mgr.zdd_print_minterm(NodeId::ZERO);
        mgr.zdd_print_minterm(NodeId::ONE);
    }
}
