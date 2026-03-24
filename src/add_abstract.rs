// lumindd — ADD existential, universal, and OR abstraction
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::HashMap;

use crate::manager::Manager;
use crate::node::{NodeId, CONST_INDEX};

/// Local cache key for ADD abstraction operations.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct AbstractCacheKey {
    kind: u8, // 0 = exist, 1 = univ, 2 = or
    f: u32,
    cube: u32,
}

impl Manager {
    // ==================================================================
    // ADD Existential Abstraction
    // ==================================================================

    /// ADD existential abstraction: for each variable in `cube`, sum the
    /// positive and negative cofactors using ADD plus.
    ///
    /// `cube` is a BDD cube (conjunction) of variables to abstract over.
    /// This replaces each quantified variable with Plus(cofactor_pos, cofactor_neg).
    pub fn add_exist_abstract(&mut self, f: NodeId, cube: NodeId) -> NodeId {
        let mut cache = HashMap::new();
        self.add_exist_abstract_rec(f, cube, &mut cache)
    }

    fn add_exist_abstract_rec(
        &mut self,
        f: NodeId,
        cube: NodeId,
        cache: &mut HashMap<AbstractCacheKey, NodeId>,
    ) -> NodeId {
        // Terminal: f is constant — abstracting a constant leaves it unchanged
        if self.var_index(f) == CONST_INDEX {
            return f;
        }
        // No more variables to abstract
        if cube.is_one() {
            return f;
        }

        let key = AbstractCacheKey {
            kind: 0,
            f: f.raw_index(),
            cube: cube.raw_index(),
        };
        if let Some(&result) = cache.get(&key) {
            return result;
        }

        let f_level = self.level(f);
        let cube_level = self.level(cube);

        let result = if cube_level < f_level {
            // Cube variable is above f — skip it
            let next_cube = self.then_child(cube);
            self.add_exist_abstract_rec(f, next_cube, cache)
        } else if cube_level == f_level {
            // Quantify this variable: Plus(cofactor_pos, cofactor_neg)
            let top_var = self.var_index(f);
            let (f_t, f_e) = self.add_cofactors(f, top_var);
            let next_cube = self.then_child(cube);
            let t = self.add_exist_abstract_rec(f_t, next_cube, cache);
            let e = self.add_exist_abstract_rec(f_e, next_cube, cache);
            self.add_plus(t, e)
        } else {
            // f variable is above cube — decompose f
            let top_var = self.var_index(f);
            let (f_t, f_e) = self.add_cofactors(f, top_var);
            let t = self.add_exist_abstract_rec(f_t, cube, cache);
            let e = self.add_exist_abstract_rec(f_e, cube, cache);
            if t == e {
                t
            } else {
                self.add_unique_inter(top_var, t, e)
            }
        };

        cache.insert(key, result);
        result
    }

    // ==================================================================
    // ADD Universal Abstraction
    // ==================================================================

    /// ADD universal abstraction: for each variable in `cube`, take the
    /// minimum of the positive and negative cofactors.
    ///
    /// `cube` is a BDD cube (conjunction) of variables to abstract over.
    /// This replaces each quantified variable with Min(cofactor_pos, cofactor_neg).
    pub fn add_univ_abstract(&mut self, f: NodeId, cube: NodeId) -> NodeId {
        let mut cache = HashMap::new();
        self.add_univ_abstract_rec(f, cube, &mut cache)
    }

    fn add_univ_abstract_rec(
        &mut self,
        f: NodeId,
        cube: NodeId,
        cache: &mut HashMap<AbstractCacheKey, NodeId>,
    ) -> NodeId {
        if self.var_index(f) == CONST_INDEX {
            return f;
        }
        if cube.is_one() {
            return f;
        }

        let key = AbstractCacheKey {
            kind: 1,
            f: f.raw_index(),
            cube: cube.raw_index(),
        };
        if let Some(&result) = cache.get(&key) {
            return result;
        }

        let f_level = self.level(f);
        let cube_level = self.level(cube);

        let result = if cube_level < f_level {
            let next_cube = self.then_child(cube);
            self.add_univ_abstract_rec(f, next_cube, cache)
        } else if cube_level == f_level {
            let top_var = self.var_index(f);
            let (f_t, f_e) = self.add_cofactors(f, top_var);
            let next_cube = self.then_child(cube);
            let t = self.add_univ_abstract_rec(f_t, next_cube, cache);
            let e = self.add_univ_abstract_rec(f_e, next_cube, cache);
            self.add_min(t, e)
        } else {
            let top_var = self.var_index(f);
            let (f_t, f_e) = self.add_cofactors(f, top_var);
            let t = self.add_univ_abstract_rec(f_t, cube, cache);
            let e = self.add_univ_abstract_rec(f_e, cube, cache);
            if t == e {
                t
            } else {
                self.add_unique_inter(top_var, t, e)
            }
        };

        cache.insert(key, result);
        result
    }

    // ==================================================================
    // ADD OR Abstraction
    // ==================================================================

    /// ADD OR abstraction: for each variable in `cube`, take the maximum
    /// of the positive and negative cofactors.
    ///
    /// For 0/1-valued ADDs this corresponds to logical OR. For general ADDs
    /// it computes the pointwise maximum over the quantified variables.
    ///
    /// `cube` is a BDD cube (conjunction) of variables to abstract over.
    pub fn add_or_abstract(&mut self, f: NodeId, cube: NodeId) -> NodeId {
        let mut cache = HashMap::new();
        self.add_or_abstract_rec(f, cube, &mut cache)
    }

    fn add_or_abstract_rec(
        &mut self,
        f: NodeId,
        cube: NodeId,
        cache: &mut HashMap<AbstractCacheKey, NodeId>,
    ) -> NodeId {
        if self.var_index(f) == CONST_INDEX {
            return f;
        }
        if cube.is_one() {
            return f;
        }

        let key = AbstractCacheKey {
            kind: 2,
            f: f.raw_index(),
            cube: cube.raw_index(),
        };
        if let Some(&result) = cache.get(&key) {
            return result;
        }

        let f_level = self.level(f);
        let cube_level = self.level(cube);

        let result = if cube_level < f_level {
            let next_cube = self.then_child(cube);
            self.add_or_abstract_rec(f, next_cube, cache)
        } else if cube_level == f_level {
            let top_var = self.var_index(f);
            let (f_t, f_e) = self.add_cofactors(f, top_var);
            let next_cube = self.then_child(cube);
            let t = self.add_or_abstract_rec(f_t, next_cube, cache);
            let e = self.add_or_abstract_rec(f_e, next_cube, cache);
            self.add_max(t, e)
        } else {
            let top_var = self.var_index(f);
            let (f_t, f_e) = self.add_cofactors(f, top_var);
            let t = self.add_or_abstract_rec(f_t, cube, cache);
            let e = self.add_or_abstract_rec(f_e, cube, cache);
            if t == e {
                t
            } else {
                self.add_unique_inter(top_var, t, e)
            }
        };

        cache.insert(key, result);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_exist_abstract_constant() {
        let mut mgr = Manager::new();
        let c = mgr.add_const(5.0);
        let x = mgr.bdd_new_var();
        let cube = x;
        let result = mgr.add_exist_abstract(c, cube);
        // Abstracting a constant over any variable returns the constant
        assert_eq!(result, c);
    }

    #[test]
    fn test_add_exist_abstract_single_var() {
        let mut mgr = Manager::new();
        // f = ADD variable 0: value 1.0 when x0=1, 0.0 when x0=0
        let x0 = mgr.bdd_new_var(); // variable 0
        let f = mgr.add_ith_var(0);
        let cube = x0;

        // ∃x0. f = f|x0=1 + f|x0=0 = 1.0 + 0.0 = 1.0
        let result = mgr.add_exist_abstract(f, cube);
        assert!(result.is_one()); // ADD 1.0
    }

    #[test]
    fn test_add_exist_abstract_identity_no_cube() {
        let mut mgr = Manager::new();
        let _x0 = mgr.bdd_new_var();
        let f = mgr.add_ith_var(0);
        // Empty cube (ONE) means no abstraction
        let result = mgr.add_exist_abstract(f, NodeId::ONE);
        assert_eq!(result, f);
    }

    #[test]
    fn test_add_univ_abstract_single_var() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var();
        let f = mgr.add_ith_var(0);
        let cube = x0;

        // ∀x0. f = min(f|x0=1, f|x0=0) = min(1.0, 0.0) = 0.0
        let result = mgr.add_univ_abstract(f, cube);
        let val = mgr.add_value(result).unwrap();
        assert!((val - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_add_or_abstract_single_var() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var();
        let f = mgr.add_ith_var(0);
        let cube = x0;

        // OR_x0 f = max(f|x0=1, f|x0=0) = max(1.0, 0.0) = 1.0
        let result = mgr.add_or_abstract(f, cube);
        assert!(result.is_one());
    }

    #[test]
    fn test_add_exist_abstract_two_vars() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var();
        let x1 = mgr.bdd_new_var();

        // f = ADD x0 + ADD x1 (each is 0/1 valued)
        let add_x0 = mgr.add_ith_var(0);
        let add_x1 = mgr.add_ith_var(1);
        let f = mgr.add_plus(add_x0, add_x1);

        // Build cube for x0 only
        let cube = x0;

        // ∃x0. (x0 + x1) = (1 + x1) + (0 + x1) = (1 + x1) + x1
        // When x1=0: 1 + 0 = 1
        // When x1=1: 2 + 1 = 3
        // Wait: ∃x0. f = f|x0=1 + f|x0=0 = (1+x1) + (0+x1) = 1 + 2*x1
        let result = mgr.add_exist_abstract(f, cube);

        // Evaluate at x1=0: should be 1.0
        let (r_t, r_e) = mgr.add_cofactors(result, 1);
        let val_x1_1 = mgr.add_value(r_t).unwrap();
        let val_x1_0 = mgr.add_value(r_e).unwrap();
        assert!((val_x1_0 - 1.0).abs() < f64::EPSILON);
        assert!((val_x1_1 - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_add_univ_abstract_constant_result() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var();
        let _x1 = mgr.bdd_new_var();

        // f = constant 3.0
        let f = mgr.add_const(3.0);
        let cube = x0;

        // ∀x0. 3.0 = min(3.0, 3.0) = 3.0
        let result = mgr.add_univ_abstract(f, cube);
        let val = mgr.add_value(result).unwrap();
        assert!((val - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_add_or_abstract_two_vars() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var();
        let x1 = mgr.bdd_new_var();

        // f = x0 * x1 (ADD times — 1 only when both are 1)
        let add_x0 = mgr.add_ith_var(0);
        let add_x1 = mgr.add_ith_var(1);
        let f = mgr.add_times(add_x0, add_x1);

        // OR abstract over both x0 and x1
        let cube = mgr.bdd_and(x0, x1);

        // max over all {x0, x1}: max of {0, 0, 0, 1} = 1.0
        let result = mgr.add_or_abstract(f, cube);
        assert!(result.is_one());
    }
}
