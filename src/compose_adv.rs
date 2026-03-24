// lumindd — Advanced composition and permutation operations
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::HashMap;

use crate::manager::Manager;
use crate::node::NodeId;

impl Manager {
    /// Simultaneous substitution of all variables in `f`.
    ///
    /// `vector[i]` is the BDD that replaces variable `i`. The vector must have
    /// length >= `num_vars`. This is the core operation for image computation
    /// in symbolic model checking.
    ///
    /// Algorithm: recursively decompose by top variable; for variable `i`,
    /// compute `ITE(vector[i], f_then, f_else)`.
    pub fn bdd_vector_compose(
        &mut self,
        f: NodeId,
        vector: &[NodeId],
    ) -> NodeId {
        let mut cache: HashMap<(u32, bool), NodeId> = HashMap::new();
        self.bdd_vector_compose_rec(f, vector, &mut cache)
    }

    fn bdd_vector_compose_rec(
        &mut self,
        f: NodeId,
        vector: &[NodeId],
        cache: &mut HashMap<(u32, bool), NodeId>,
    ) -> NodeId {
        // Terminal case
        if f.is_constant() {
            return f;
        }

        // Check local cache
        let key = (f.raw_index(), f.is_complemented());
        if let Some(&result) = cache.get(&key) {
            return result;
        }

        let f_var = self.var_index(f.regular());
        let (f_t, f_e) = self.bdd_cofactors(f, f_var);

        // Recurse on both children
        let t = self.bdd_vector_compose_rec(f_t, vector, cache);
        let e = self.bdd_vector_compose_rec(f_e, vector, cache);

        // Get the replacement for this variable
        let replacement = if (f_var as usize) < vector.len() {
            vector[f_var as usize]
        } else {
            // No replacement specified; use the variable's projection function
            self.bdd_ith_var(f_var)
        };

        // ITE(replacement, then_result, else_result)
        let result = self.bdd_ite(replacement, t, e);

        cache.insert(key, result);
        result
    }

    /// Rename variables in `f` according to `permutation`.
    ///
    /// Variable `i` becomes variable `permutation[i]`. The permutation must
    /// be a valid bijection (no two variables map to the same target), but
    /// this is not checked for performance reasons.
    pub fn bdd_permute(
        &mut self,
        f: NodeId,
        permutation: &[u16],
    ) -> NodeId {
        let mut cache: HashMap<(u32, bool), NodeId> = HashMap::new();
        self.bdd_permute_rec(f, permutation, &mut cache)
    }

    fn bdd_permute_rec(
        &mut self,
        f: NodeId,
        permutation: &[u16],
        cache: &mut HashMap<(u32, bool), NodeId>,
    ) -> NodeId {
        if f.is_constant() {
            return f;
        }

        let key = (f.raw_index(), f.is_complemented());
        if let Some(&result) = cache.get(&key) {
            return result;
        }

        let f_var = self.var_index(f.regular());
        let (f_t, f_e) = self.bdd_cofactors(f, f_var);

        let t = self.bdd_permute_rec(f_t, permutation, cache);
        let e = self.bdd_permute_rec(f_e, permutation, cache);

        // Map the variable
        let new_var = if (f_var as usize) < permutation.len() {
            permutation[f_var as usize]
        } else {
            f_var
        };

        // We cannot simply do unique_inter with new_var because the children
        // may already contain nodes at the new variable's level. Use ITE with
        // the new variable's projection function.
        let proj = self.bdd_ith_var(new_var);
        let result = self.bdd_ite(proj, t, e);

        cache.insert(key, result);
        result
    }

    /// Swap two sets of variables: `x[i]` is swapped with `y[i]`.
    ///
    /// Both slices must have the same length. This builds a vector-compose
    /// table where x[i] maps to the projection of y[i] and vice versa,
    /// then performs a simultaneous substitution.
    pub fn bdd_swap_variables(
        &mut self,
        f: NodeId,
        x: &[u16],
        y: &[u16],
    ) -> NodeId {
        assert_eq!(x.len(), y.len(), "x and y must have the same length");

        if f.is_constant() {
            return f;
        }

        // Build the substitution vector: identity by default
        let n = self.num_vars as usize;
        let mut vector: Vec<NodeId> = (0..n)
            .map(|i| self.bdd_ith_var(i as u16))
            .collect();

        // Set up swaps
        for i in 0..x.len() {
            let xi = x[i] as usize;
            let yi = y[i] as usize;
            if xi < n && yi < n {
                vector[xi] = self.bdd_ith_var(y[i]);
                vector[yi] = self.bdd_ith_var(x[i]);
            }
        }

        self.bdd_vector_compose(f, &vector)
    }

    /// Simultaneous substitution for ADD: `vector[i]` replaces variable `i`.
    ///
    /// Same algorithm as BDD vector compose but uses ADD ITE.
    pub fn add_vector_compose(
        &mut self,
        f: NodeId,
        vector: &[NodeId],
    ) -> NodeId {
        let mut cache: HashMap<u32, NodeId> = HashMap::new();
        self.add_vector_compose_rec(f, vector, &mut cache)
    }

    fn add_vector_compose_rec(
        &mut self,
        f: NodeId,
        vector: &[NodeId],
        cache: &mut HashMap<u32, NodeId>,
    ) -> NodeId {
        if f.is_constant() || self.var_index(f) == crate::node::CONST_INDEX {
            return f;
        }

        let key = f.raw_index();
        if let Some(&result) = cache.get(&key) {
            return result;
        }

        let f_var = self.var_index(f);
        let (f_t, f_e) = self.add_cofactors(f, f_var);

        let t = self.add_vector_compose_rec(f_t, vector, cache);
        let e = self.add_vector_compose_rec(f_e, vector, cache);

        let replacement = if (f_var as usize) < vector.len() {
            vector[f_var as usize]
        } else {
            self.add_ith_var(f_var)
        };

        // ADD ITE: select t where replacement > 0, e otherwise
        let result = self.add_ite(replacement, t, e);

        cache.insert(key, result);
        result
    }

    /// Rename variables in an ADD according to `permutation`.
    pub fn add_permute(
        &mut self,
        f: NodeId,
        permutation: &[u16],
    ) -> NodeId {
        let mut cache: HashMap<u32, NodeId> = HashMap::new();
        self.add_permute_rec(f, permutation, &mut cache)
    }

    fn add_permute_rec(
        &mut self,
        f: NodeId,
        permutation: &[u16],
        cache: &mut HashMap<u32, NodeId>,
    ) -> NodeId {
        if f.is_constant() || self.var_index(f) == crate::node::CONST_INDEX {
            return f;
        }

        let key = f.raw_index();
        if let Some(&result) = cache.get(&key) {
            return result;
        }

        let f_var = self.var_index(f);
        let (f_t, f_e) = self.add_cofactors(f, f_var);

        let t = self.add_permute_rec(f_t, permutation, cache);
        let e = self.add_permute_rec(f_e, permutation, cache);

        let new_var = if (f_var as usize) < permutation.len() {
            permutation[f_var as usize]
        } else {
            f_var
        };

        let proj = self.add_ith_var(new_var);
        let result = self.add_ite(proj, t, e);

        cache.insert(key, result);
        result
    }
}
