// lumindd — BDD approximation and subsetting methods
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::HashMap;

use crate::manager::Manager;
use crate::node::NodeId;

impl Manager {
    // ==================================================================
    // Node counting helper
    // ==================================================================

    /// Count the number of nodes in the BDD rooted at `f`.
    fn bdd_node_count(&self, f: NodeId) -> u32 {
        let mut visited: std::collections::HashSet<u32> = std::collections::HashSet::new();
        self.bdd_node_count_rec(f, &mut visited);
        visited.len() as u32
    }

    fn bdd_node_count_rec(&self, f: NodeId, visited: &mut std::collections::HashSet<u32>) {
        if f.is_constant() {
            return;
        }
        let raw = f.raw_index();
        if !visited.insert(raw) {
            return;
        }
        let var = self.var_index(f.regular());
        let (t, e) = self.bdd_cofactors(f, var);
        self.bdd_node_count_rec(t, visited);
        self.bdd_node_count_rec(e, visited);
    }

    // ==================================================================
    // Minterm counting helper
    // ==================================================================

    /// Count the number of minterms (satisfying assignments) for `num_vars` variables.
    fn bdd_minterm_count(&self, f: NodeId, num_vars: u32) -> f64 {
        let mut cache: HashMap<(u32, bool), f64> = HashMap::new();
        self.bdd_minterm_count_rec(f, num_vars, 0, &mut cache)
    }

    fn bdd_minterm_count_rec(
        &self,
        f: NodeId,
        num_vars: u32,
        current_level: u32,
        cache: &mut HashMap<(u32, bool), f64>,
    ) -> f64 {
        if f.is_one() {
            return 2.0f64.powi((num_vars - current_level) as i32);
        }
        if f.is_zero() {
            return 0.0;
        }

        let key = (f.raw_index(), f.is_complemented());
        if let Some(&count) = cache.get(&key) {
            // Adjust for the level gap
            let node_level = self.level(f);
            let gap = node_level - current_level;
            return count * 2.0f64.powi(gap as i32);
        }

        let f_var = self.var_index(f.regular());
        let node_level = self.level(f);
        let (f_t, f_e) = self.bdd_cofactors(f, f_var);

        let t_count = self.bdd_minterm_count_rec(f_t, num_vars, node_level + 1, cache);
        let e_count = self.bdd_minterm_count_rec(f_e, num_vars, node_level + 1, cache);

        let total = t_count + e_count;
        cache.insert(key, total);

        // Account for skipped levels between current_level and node_level
        let gap = node_level - current_level;
        total * 2.0f64.powi(gap as i32)
    }

    // ==================================================================
    // Shortest path length helper
    // ==================================================================

    /// Compute the length of the shortest path from the root to a ONE terminal.
    fn bdd_shortest_path_length(&self, f: NodeId) -> u32 {
        let mut cache: HashMap<(u32, bool), u32> = HashMap::new();
        self.bdd_shortest_path_rec(f, &mut cache)
    }

    fn bdd_shortest_path_rec(
        &self,
        f: NodeId,
        cache: &mut HashMap<(u32, bool), u32>,
    ) -> u32 {
        if f.is_one() {
            return 0;
        }
        if f.is_zero() {
            return u32::MAX;
        }

        let key = (f.raw_index(), f.is_complemented());
        if let Some(&len) = cache.get(&key) {
            return len;
        }

        let f_var = self.var_index(f.regular());
        let (f_t, f_e) = self.bdd_cofactors(f, f_var);

        let t_len = self.bdd_shortest_path_rec(f_t, cache);
        let e_len = self.bdd_shortest_path_rec(f_e, cache);

        let result = t_len.min(e_len).saturating_add(1);
        cache.insert(key, result);
        result
    }

    // ==================================================================
    // Underapproximation
    // ==================================================================

    /// Underapproximation: the result implies `f`, with at most `threshold` nodes.
    ///
    /// Replaces subtrees that would exceed the threshold with ZERO, ensuring
    /// the result is a subset of the minterms of `f`.
    pub fn bdd_under_approx(
        &mut self,
        f: NodeId,
        num_vars: u32,
        threshold: u32,
    ) -> NodeId {
        if f.is_constant() {
            return f;
        }
        if self.bdd_node_count(f) <= threshold {
            return f;
        }
        self.bdd_subset_heavy_branch(f, num_vars, threshold)
    }

    // ==================================================================
    // Overapproximation
    // ==================================================================

    /// Overapproximation: `f` implies the result.
    ///
    /// Replaces subtrees that would exceed the threshold with ONE, ensuring
    /// the minterms of `f` are a subset of the result.
    pub fn bdd_over_approx(
        &mut self,
        f: NodeId,
        num_vars: u32,
        threshold: u32,
    ) -> NodeId {
        if f.is_constant() {
            return f;
        }
        if self.bdd_node_count(f) <= threshold {
            return f;
        }
        self.bdd_superset_heavy_branch(f, num_vars, threshold)
    }

    // ==================================================================
    // Heavy-branch subsetting
    // ==================================================================

    /// Keep heavy (high-minterm-count) branches, replace light branches with ZERO.
    ///
    /// At each node, compare the minterm counts of the then- and else-branches.
    /// If the BDD is still too large after recursing, prune the lighter branch.
    pub fn bdd_subset_heavy_branch(
        &mut self,
        f: NodeId,
        num_vars: u32,
        threshold: u32,
    ) -> NodeId {
        let mut info: HashMap<(u32, bool), f64> = HashMap::new();
        // Pre-compute minterm densities
        self.bdd_minterm_count_rec(f, num_vars, 0, &mut info);

        let mut cache: HashMap<(u32, bool), NodeId> = HashMap::new();
        self.bdd_subset_heavy_rec(f, num_vars, threshold, &mut cache)
    }

    fn bdd_subset_heavy_rec(
        &mut self,
        f: NodeId,
        num_vars: u32,
        threshold: u32,
        cache: &mut HashMap<(u32, bool), NodeId>,
    ) -> NodeId {
        if f.is_constant() || threshold <= 1 {
            if threshold == 0 {
                return NodeId::ZERO;
            }
            return f;
        }

        let key = (f.raw_index(), f.is_complemented());
        if let Some(&result) = cache.get(&key) {
            return result;
        }

        let count = self.bdd_node_count(f);
        if count <= threshold {
            cache.insert(key, f);
            return f;
        }

        let f_var = self.var_index(f.regular());
        let (f_t, f_e) = self.bdd_cofactors(f, f_var);

        // Count minterms in each branch
        let t_minterms = self.bdd_minterm_count(f_t, num_vars);
        let e_minterms = self.bdd_minterm_count(f_e, num_vars);

        // Allocate budget proportionally to minterm count, ensuring at least 1 for each
        let half = threshold.saturating_sub(1);
        let (t_budget, e_budget) = if t_minterms + e_minterms > 0.0 {
            let t_frac = t_minterms / (t_minterms + e_minterms);
            let t_alloc = ((half as f64) * t_frac).round() as u32;
            let t_alloc = t_alloc.max(1).min(half.saturating_sub(1));
            (t_alloc, half.saturating_sub(t_alloc).max(1))
        } else {
            let h = half / 2;
            (h.max(1), (half - h).max(1))
        };

        let t = self.bdd_subset_heavy_rec(f_t, num_vars, t_budget, cache);
        let e = self.bdd_subset_heavy_rec(f_e, num_vars, e_budget, cache);

        let result = if t == e { t } else { self.unique_inter(f_var, t, e) };

        // If still too large, prune the lighter branch
        let result = if self.bdd_node_count(result) > threshold {
            if t_minterms >= e_minterms {
                // Keep then, drop else
                let pruned_t = self.bdd_subset_heavy_rec(f_t, num_vars, threshold.saturating_sub(1), cache);
                if pruned_t == NodeId::ZERO {
                    NodeId::ZERO
                } else {
                    self.unique_inter(f_var, pruned_t, NodeId::ZERO)
                }
            } else {
                // Keep else, drop then
                let pruned_e = self.bdd_subset_heavy_rec(f_e, num_vars, threshold.saturating_sub(1), cache);
                if pruned_e == NodeId::ZERO {
                    NodeId::ZERO
                } else {
                    self.unique_inter(f_var, NodeId::ZERO, pruned_e)
                }
            }
        } else {
            result
        };

        cache.insert(key, result);
        result
    }

    /// Dual of heavy-branch subsetting: replace light branches with ONE.
    ///
    /// The result is a superset of the minterms of `f`.
    pub fn bdd_superset_heavy_branch(
        &mut self,
        f: NodeId,
        num_vars: u32,
        threshold: u32,
    ) -> NodeId {
        // Superset via duality: complement, subset, complement
        // NOT(subset_heavy(NOT(f))) gives a superset of f
        let nf = f.not();
        let subset = self.bdd_subset_heavy_branch(nf, num_vars, threshold);
        subset.not()
    }

    // ==================================================================
    // Short-path subsetting
    // ==================================================================

    /// Keep short paths (few decisions to reach ONE), prune long paths to ZERO.
    pub fn bdd_subset_short_paths(
        &mut self,
        f: NodeId,
        num_vars: u32,
        threshold: u32,
    ) -> NodeId {
        let mut cache: HashMap<(u32, bool), NodeId> = HashMap::new();
        self.bdd_subset_short_rec(f, num_vars, threshold, &mut cache)
    }

    fn bdd_subset_short_rec(
        &mut self,
        f: NodeId,
        num_vars: u32,
        threshold: u32,
        cache: &mut HashMap<(u32, bool), NodeId>,
    ) -> NodeId {
        if f.is_constant() || threshold <= 1 {
            if threshold == 0 {
                return NodeId::ZERO;
            }
            return f;
        }

        let key = (f.raw_index(), f.is_complemented());
        if let Some(&result) = cache.get(&key) {
            return result;
        }

        let count = self.bdd_node_count(f);
        if count <= threshold {
            cache.insert(key, f);
            return f;
        }

        let f_var = self.var_index(f.regular());
        let (f_t, f_e) = self.bdd_cofactors(f, f_var);

        // Compute shortest path lengths
        let t_len = self.bdd_shortest_path_length(f_t);
        let e_len = self.bdd_shortest_path_length(f_e);

        // Allocate budget: give more to the branch with shorter paths
        let half = threshold.saturating_sub(1);
        let (t_budget, e_budget) = if t_len <= e_len {
            let t_alloc = (half * 2 / 3).max(1).min(half.saturating_sub(1));
            (t_alloc, half.saturating_sub(t_alloc).max(1))
        } else {
            let e_alloc = (half * 2 / 3).max(1).min(half.saturating_sub(1));
            (half.saturating_sub(e_alloc).max(1), e_alloc)
        };

        let t = self.bdd_subset_short_rec(f_t, num_vars, t_budget, cache);
        let e = self.bdd_subset_short_rec(f_e, num_vars, e_budget, cache);

        let result = if t == e { t } else { self.unique_inter(f_var, t, e) };

        // If still too large, prune the branch with longer paths
        let result = if self.bdd_node_count(result) > threshold {
            if t_len <= e_len {
                let pruned_t = self.bdd_subset_short_rec(f_t, num_vars, threshold.saturating_sub(1), cache);
                if pruned_t == NodeId::ZERO {
                    NodeId::ZERO
                } else {
                    self.unique_inter(f_var, pruned_t, NodeId::ZERO)
                }
            } else {
                let pruned_e = self.bdd_subset_short_rec(f_e, num_vars, threshold.saturating_sub(1), cache);
                if pruned_e == NodeId::ZERO {
                    NodeId::ZERO
                } else {
                    self.unique_inter(f_var, NodeId::ZERO, pruned_e)
                }
            }
        } else {
            result
        };

        cache.insert(key, result);
        result
    }

    /// Dual: replace long paths with ONE (superset).
    pub fn bdd_superset_short_paths(
        &mut self,
        f: NodeId,
        num_vars: u32,
        threshold: u32,
    ) -> NodeId {
        let nf = f.not();
        let subset = self.bdd_subset_short_paths(nf, num_vars, threshold);
        subset.not()
    }

    // ==================================================================
    // Remap-based underapproximation (Ravi & Somenzi, ICCAD'98)
    // ==================================================================

    /// Remap-based underapproximation.
    ///
    /// Iteratively applies restrict with care-set refinement to find a
    /// compact underapproximation. The result implies `f`.
    pub fn bdd_remap_under_approx(
        &mut self,
        f: NodeId,
        num_vars: u32,
        threshold: u32,
    ) -> NodeId {
        if f.is_constant() {
            return f;
        }
        if self.bdd_node_count(f) <= threshold {
            return f;
        }

        // Phase 1: get a heavy-branch subset as initial approximation
        let mut approx = self.bdd_subset_heavy_branch(f, num_vars, threshold);

        // Phase 2: iterative refinement using constrain
        // The idea: approx implies f. Compute the "don't care" set where
        // approx and f agree, then use constrain to simplify.
        for _ in 0..3 {
            let count = self.bdd_node_count(approx);
            if count <= threshold {
                break;
            }

            // care = f OR NOT(approx) -- where we care about the value
            // On the care set, approx must equal f. Simplify approx with
            // respect to this care set.
            let care = self.bdd_or(f, approx.not());
            let simplified = self.bdd_restrict(approx, care);

            let new_count = self.bdd_node_count(simplified);
            if new_count < count {
                // Ensure the result still implies f
                let check = self.bdd_and(simplified, f.not());
                if check.is_zero() {
                    approx = simplified;
                } else {
                    // Intersection to maintain under-approximation
                    approx = self.bdd_and(simplified, f);
                }
            } else {
                break;
            }
        }

        // Final size check: if still too big, fall back to heavy branch
        if self.bdd_node_count(approx) > threshold {
            approx = self.bdd_subset_heavy_branch(f, num_vars, threshold);
        }

        approx
    }

    // ==================================================================
    // Squeeze (Shiple, 1996)
    // ==================================================================

    /// Find a BDD between lower and upper bounds with minimum nodes.
    ///
    /// Precondition: `lb` implies `ub` (lb AND NOT(ub) = 0).
    /// The result `r` satisfies: `lb` implies `r`, and `r` implies `ub`.
    pub fn bdd_squeeze(&mut self, lb: NodeId, ub: NodeId) -> NodeId {
        // Terminal cases
        if lb.is_zero() {
            return NodeId::ZERO;
        }
        if ub.is_one() {
            return NodeId::ONE;
        }
        if lb == ub {
            return lb;
        }
        if ub.is_zero() {
            // lb should also be zero if the precondition holds
            return NodeId::ZERO;
        }
        if lb.is_one() {
            return NodeId::ONE;
        }

        // Check cache
        if let Some(result) = self.cache.lookup(
            crate::computed_table::OpTag::BddSqueeze,
            lb,
            ub,
            NodeId::ZERO,
        ) {
            return result;
        }

        // Find top variable
        let lb_level = self.level(lb);
        let ub_level = self.level(ub);
        let top_level = lb_level.min(ub_level);
        let top_var = self.inv_perm[top_level as usize] as u16;

        let (lb_t, lb_e) = self.bdd_cofactors(lb, top_var);
        let (ub_t, ub_e) = self.bdd_cofactors(ub, top_var);

        // If one cofactor of ub is ONE, we can be aggressive
        if ub_t.is_one() {
            let e = self.bdd_squeeze(lb_e, ub_e);
            let result = if e.is_one() {
                NodeId::ONE
            } else {
                self.unique_inter(top_var, NodeId::ONE, e)
            };
            self.cache.insert(
                crate::computed_table::OpTag::BddSqueeze,
                lb,
                ub,
                NodeId::ZERO,
                result,
            );
            return result;
        }
        if ub_e.is_one() {
            let t = self.bdd_squeeze(lb_t, ub_t);
            let result = if t.is_one() {
                NodeId::ONE
            } else {
                self.unique_inter(top_var, t, NodeId::ONE)
            };
            self.cache.insert(
                crate::computed_table::OpTag::BddSqueeze,
                lb,
                ub,
                NodeId::ZERO,
                result,
            );
            return result;
        }

        // If one cofactor of lb is ZERO, recurse only on the other
        if lb_t.is_zero() {
            let e = self.bdd_squeeze(lb_e, ub_e);
            // Then branch is unconstrained from below; use ZERO
            let result = if e == NodeId::ZERO {
                NodeId::ZERO
            } else {
                self.unique_inter(top_var, NodeId::ZERO, e)
            };
            self.cache.insert(
                crate::computed_table::OpTag::BddSqueeze,
                lb,
                ub,
                NodeId::ZERO,
                result,
            );
            return result;
        }
        if lb_e.is_zero() {
            let t = self.bdd_squeeze(lb_t, ub_t);
            let result = if t == NodeId::ZERO {
                NodeId::ZERO
            } else {
                self.unique_inter(top_var, t, NodeId::ZERO)
            };
            self.cache.insert(
                crate::computed_table::OpTag::BddSqueeze,
                lb,
                ub,
                NodeId::ZERO,
                result,
            );
            return result;
        }

        let t = self.bdd_squeeze(lb_t, ub_t);
        let e = self.bdd_squeeze(lb_e, ub_e);

        let result = if t == e { t } else { self.unique_inter(top_var, t, e) };

        self.cache.insert(
            crate::computed_table::OpTag::BddSqueeze,
            lb,
            ub,
            NodeId::ZERO,
            result,
        );
        result
    }
}
