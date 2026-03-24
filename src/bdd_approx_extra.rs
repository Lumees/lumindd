// lumindd — Additional BDD approximation methods for CUDD parity
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::HashMap;

use crate::manager::Manager;
use crate::node::NodeId;

impl Manager {
    // ==================================================================
    // Remap-based overapproximation
    // ==================================================================

    /// Remap-based overapproximation.
    ///
    /// Dual of `bdd_remap_under_approx`: computes NOT(remap_under_approx(NOT f)).
    /// The result is a superset of the minterms of `f`.
    pub fn bdd_remap_over_approx(
        &mut self,
        f: NodeId,
        num_vars: u32,
        threshold: u32,
    ) -> NodeId {
        if f.is_constant() {
            return f;
        }
        let nf = f.not();
        let under = self.bdd_remap_under_approx(nf, num_vars, threshold);
        under.not()
    }

    // ==================================================================
    // Biased underapproximation
    // ==================================================================

    /// Biased underapproximation.
    ///
    /// Like heavy-branch subsetting but uses `bias` (0.0 to 1.0) to
    /// weight the decision of which branches to keep. Higher bias favors
    /// keeping the then-branch; lower bias favors the else-branch.
    ///
    /// The result implies `f` (is a subset of `f`'s minterms).
    pub fn bdd_biased_under_approx(
        &mut self,
        f: NodeId,
        num_vars: u32,
        threshold: u32,
        bias: f64,
    ) -> NodeId {
        if f.is_constant() {
            return f;
        }
        if self.bdd_node_count_ext(f) <= threshold {
            return f;
        }
        let bias_clamped = bias.clamp(0.0, 1.0);
        let mut cache: HashMap<(u32, bool), NodeId> = HashMap::new();
        self.bdd_biased_under_rec(f, num_vars, threshold, bias_clamped, &mut cache)
    }

    fn bdd_biased_under_rec(
        &mut self,
        f: NodeId,
        num_vars: u32,
        threshold: u32,
        bias: f64,
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

        let count = self.bdd_node_count_ext(f);
        if count <= threshold {
            cache.insert(key, f);
            return f;
        }

        let f_var = self.var_index(f.regular());
        let (f_t, f_e) = self.bdd_cofactors(f, f_var);

        // Allocate budget using bias: bias controls the fraction given to the then-branch
        let half = threshold.saturating_sub(1);
        let t_alloc = ((half as f64) * bias).round() as u32;
        let t_budget = t_alloc.max(1).min(half.saturating_sub(1));
        let e_budget = half.saturating_sub(t_budget).max(1);

        let t = self.bdd_biased_under_rec(f_t, num_vars, t_budget, bias, cache);
        let e = self.bdd_biased_under_rec(f_e, num_vars, e_budget, bias, cache);

        let result = if t == e { t } else { self.unique_inter(f_var, t, e) };

        // If still too large, prune based on bias preference
        let result = if self.bdd_node_count_ext(result) > threshold {
            if bias >= 0.5 {
                // Favor then-branch: prune else
                let pruned_t = self.bdd_biased_under_rec(f_t, num_vars, threshold.saturating_sub(1), bias, cache);
                if pruned_t == NodeId::ZERO {
                    NodeId::ZERO
                } else {
                    self.unique_inter(f_var, pruned_t, NodeId::ZERO)
                }
            } else {
                // Favor else-branch: prune then
                let pruned_e = self.bdd_biased_under_rec(f_e, num_vars, threshold.saturating_sub(1), bias, cache);
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

    // ==================================================================
    // Biased overapproximation
    // ==================================================================

    /// Biased overapproximation.
    ///
    /// Dual of biased underapproximation: NOT(biased_under(NOT f)).
    /// The result is a superset of `f`'s minterms.
    pub fn bdd_biased_over_approx(
        &mut self,
        f: NodeId,
        num_vars: u32,
        threshold: u32,
        bias: f64,
    ) -> NodeId {
        if f.is_constant() {
            return f;
        }
        let nf = f.not();
        let under = self.bdd_biased_under_approx(nf, num_vars, threshold, bias);
        under.not()
    }

    // ==================================================================
    // Subset compression
    // ==================================================================

    /// Subset compression: iteratively restrict and subset until size is within threshold.
    ///
    /// First restricts `f` by its heavy-branch subset, then takes the heavy-branch
    /// subset of the result. Repeats until the BDD size is within `threshold`.
    /// The result implies `f`.
    pub fn bdd_subset_compress(
        &mut self,
        f: NodeId,
        num_vars: u32,
        threshold: u32,
    ) -> NodeId {
        if f.is_constant() {
            return f;
        }
        if self.bdd_node_count_ext(f) <= threshold {
            return f;
        }

        let mut approx = f;
        for _ in 0..5 {
            let count = self.bdd_node_count_ext(approx);
            if count <= threshold {
                break;
            }

            // Get a heavy-branch subset
            let subset = self.bdd_subset_heavy_branch(approx, num_vars, threshold);

            // Use the subset as a care set and restrict
            if !subset.is_zero() {
                let restricted = self.bdd_restrict(approx, subset);
                // Ensure the result still implies f
                let check = self.bdd_and(restricted, f);
                if self.bdd_node_count_ext(check) < count {
                    approx = check;
                } else {
                    approx = subset;
                    break;
                }
            } else {
                approx = subset;
                break;
            }
        }

        // Final fallback: if still too large, do a plain heavy-branch subset
        if self.bdd_node_count_ext(approx) > threshold {
            approx = self.bdd_subset_heavy_branch(f, num_vars, threshold);
        }

        approx
    }

    // ==================================================================
    // Superset compression
    // ==================================================================

    /// Superset compression: dual of subset compression.
    ///
    /// NOT(subset_compress(NOT f)). The result is a superset of `f`'s minterms.
    pub fn bdd_superset_compress(
        &mut self,
        f: NodeId,
        num_vars: u32,
        threshold: u32,
    ) -> NodeId {
        if f.is_constant() {
            return f;
        }
        let nf = f.not();
        let subset = self.bdd_subset_compress(nf, num_vars, threshold);
        subset.not()
    }

    // ==================================================================
    // Hamming distance comparison BDDs
    // ==================================================================

    /// Build BDD for d(x,y) > d(x,z) where d is Hamming distance.
    ///
    /// The three variable vectors `x_vars`, `y_vars`, `z_vars` must all
    /// have the same length. The result BDD is true for assignments where
    /// the Hamming distance between x and y exceeds the Hamming distance
    /// between x and z.
    pub fn bdd_dxygtdxz(
        &mut self,
        x_vars: &[u16],
        y_vars: &[u16],
        z_vars: &[u16],
    ) -> NodeId {
        assert_eq!(x_vars.len(), y_vars.len(), "x_vars and y_vars must have same length");
        assert_eq!(x_vars.len(), z_vars.len(), "x_vars and z_vars must have same length");

        // Compute d(x,y) as an ADD
        let dxy = self.add_hamming(x_vars, y_vars);
        // Compute d(x,z) as an ADD
        let dxz = self.add_hamming(x_vars, z_vars);

        // Compute diff = d(x,y) - d(x,z) as an ADD
        let diff = self.add_minus(dxy, dxz);

        // Convert to BDD: result is 1 where diff > 0
        self.add_bdd_strict_threshold(diff, 0.0)
    }

    /// Build BDD for d(x,y) > d(y,z) where d is Hamming distance.
    ///
    /// Similar to `bdd_dxygtdxz` but compares d(x,y) with d(y,z).
    pub fn bdd_dxygtdyz(
        &mut self,
        x_vars: &[u16],
        y_vars: &[u16],
        z_vars: &[u16],
    ) -> NodeId {
        assert_eq!(x_vars.len(), y_vars.len(), "x_vars and y_vars must have same length");
        assert_eq!(x_vars.len(), z_vars.len(), "y_vars and z_vars must have same length");

        // Compute d(x,y) as an ADD
        let dxy = self.add_hamming(x_vars, y_vars);
        // Compute d(y,z) as an ADD
        let dyz = self.add_hamming(y_vars, z_vars);

        // Compute diff = d(x,y) - d(y,z) as an ADD
        let diff = self.add_minus(dxy, dyz);

        // Convert to BDD: result is 1 where diff > 0
        self.add_bdd_strict_threshold(diff, 0.0)
    }

    // ==================================================================
    // Private helper
    // ==================================================================

    /// Node count helper (wraps bdd_node_count from bdd_approx).
    fn bdd_node_count_ext(&self, f: NodeId) -> u32 {
        let mut visited: std::collections::HashSet<u32> = std::collections::HashSet::new();
        self.bdd_node_count_ext_rec(f, &mut visited);
        visited.len() as u32
    }

    fn bdd_node_count_ext_rec(&self, f: NodeId, visited: &mut std::collections::HashSet<u32>) {
        if f.is_constant() {
            return;
        }
        let raw = f.raw_index();
        if !visited.insert(raw) {
            return;
        }
        let var = self.var_index(f.regular());
        let (t, e) = self.bdd_cofactors(f, var);
        self.bdd_node_count_ext_rec(t, visited);
        self.bdd_node_count_ext_rec(e, visited);
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
    fn test_remap_over_approx_constant() {
        let mut mgr = Manager::new();
        let result = mgr.bdd_remap_over_approx(NodeId::ONE, 0, 10);
        assert!(result.is_one());
        let result2 = mgr.bdd_remap_over_approx(NodeId::ZERO, 0, 10);
        assert!(result2.is_zero());
    }

    #[test]
    fn test_remap_over_approx_superset() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var();
        let x1 = mgr.bdd_new_var();
        let x2 = mgr.bdd_new_var();

        // f = x0 AND x1 AND x2
        let f = mgr.bdd_and(x0, x1);
        let f = mgr.bdd_and(f, x2);

        let over = mgr.bdd_remap_over_approx(f, 3, 100);

        // Over-approximation: f implies over
        let check = mgr.bdd_and(f, over.not());
        assert!(check.is_zero());
    }

    #[test]
    fn test_biased_under_approx_subset() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var();
        let x1 = mgr.bdd_new_var();
        let x2 = mgr.bdd_new_var();

        let f = mgr.bdd_or(x0, x1);
        let f = mgr.bdd_or(f, x2);

        let under = mgr.bdd_biased_under_approx(f, 3, 2, 0.7);

        // Under-approximation: under implies f
        let check = mgr.bdd_and(under, f.not());
        assert!(check.is_zero());
    }

    #[test]
    fn test_biased_under_approx_bias_effect() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var();
        let x1 = mgr.bdd_new_var();

        let f = mgr.bdd_or(x0, x1);

        // Both should be valid under-approximations
        let under_high = mgr.bdd_biased_under_approx(f, 2, 1, 0.9);
        let under_low = mgr.bdd_biased_under_approx(f, 2, 1, 0.1);

        // Both must imply f
        let check_h = mgr.bdd_and(under_high, f.not());
        let check_l = mgr.bdd_and(under_low, f.not());
        assert!(check_h.is_zero());
        assert!(check_l.is_zero());
    }

    #[test]
    fn test_biased_over_approx_superset() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var();
        let x1 = mgr.bdd_new_var();

        let f = mgr.bdd_and(x0, x1);

        let over = mgr.bdd_biased_over_approx(f, 2, 2, 0.5);

        // f implies over
        let check = mgr.bdd_and(f, over.not());
        assert!(check.is_zero());
    }

    #[test]
    fn test_subset_compress_subset() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var();
        let x1 = mgr.bdd_new_var();
        let x2 = mgr.bdd_new_var();

        let f = mgr.bdd_or(x0, x1);
        let f = mgr.bdd_or(f, x2);

        let compressed = mgr.bdd_subset_compress(f, 3, 2);

        // Must imply f
        let check = mgr.bdd_and(compressed, f.not());
        assert!(check.is_zero());
    }

    #[test]
    fn test_superset_compress_superset() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var();
        let x1 = mgr.bdd_new_var();
        let x2 = mgr.bdd_new_var();

        let f = mgr.bdd_and(x0, x1);
        let f = mgr.bdd_and(f, x2);

        let compressed = mgr.bdd_superset_compress(f, 3, 2);

        // f implies compressed
        let check = mgr.bdd_and(f, compressed.not());
        assert!(check.is_zero());
    }

    #[test]
    fn test_bdd_dxygtdxz_trivial() {
        let mut mgr = Manager::new();
        // 1-bit case: x={0}, y={1}, z={2}
        let _x0 = mgr.bdd_new_var();
        let _x1 = mgr.bdd_new_var();
        let _x2 = mgr.bdd_new_var();

        let result = mgr.bdd_dxygtdxz(&[0], &[1], &[2]);

        // d(x,y) > d(x,z) for 1-bit:
        // d(x,y) = x XOR y, d(x,z) = x XOR z
        // True when x!=y and x==z, i.e., (x XOR y) > (x XOR z)
        // That means d(x,y)=1 and d(x,z)=0: x!=y, x==z

        // x=0, y=0, z=0: d=0,0 -> false
        assert!(!mgr.bdd_eval(result, &[false, false, false]));
        // x=0, y=1, z=0: d=1,0 -> true (x!=y, x==z)
        assert!(mgr.bdd_eval(result, &[false, true, false]));
        // x=1, y=0, z=1: d=1,0 -> true (x!=y, x==z)
        assert!(mgr.bdd_eval(result, &[true, false, true]));
        // x=1, y=1, z=1: d=0,0 -> false
        assert!(!mgr.bdd_eval(result, &[true, true, true]));
        // x=0, y=0, z=1: d=0,1 -> false
        assert!(!mgr.bdd_eval(result, &[false, false, true]));
    }

    #[test]
    fn test_bdd_dxygtdyz_trivial() {
        let mut mgr = Manager::new();
        let _x0 = mgr.bdd_new_var();
        let _x1 = mgr.bdd_new_var();
        let _x2 = mgr.bdd_new_var();

        let result = mgr.bdd_dxygtdyz(&[0], &[1], &[2]);

        // d(x,y) > d(y,z) for 1-bit:
        // True when x!=y and y==z
        // x=0, y=1, z=1: d(x,y)=1, d(y,z)=0 -> true
        assert!(mgr.bdd_eval(result, &[false, true, true]));
        // x=1, y=0, z=0: d(x,y)=1, d(y,z)=0 -> true
        assert!(mgr.bdd_eval(result, &[true, false, false]));
        // x=0, y=0, z=0: d=0,0 -> false
        assert!(!mgr.bdd_eval(result, &[false, false, false]));
        // x=0, y=1, z=0: d=1,1 -> false (equal, not greater)
        assert!(!mgr.bdd_eval(result, &[false, true, false]));
    }

    #[test]
    fn test_bdd_dxygtdxz_two_bits() {
        let mut mgr = Manager::new();
        for _ in 0..6 {
            mgr.bdd_new_var();
        }
        // x = {0,1}, y = {2,3}, z = {4,5}
        let result = mgr.bdd_dxygtdxz(&[0, 1], &[2, 3], &[4, 5]);

        // x=00, y=11, z=00: d(x,y)=2, d(x,z)=0 -> true
        assert!(mgr.bdd_eval(result, &[false, false, true, true, false, false]));
        // x=00, y=00, z=11: d(x,y)=0, d(x,z)=2 -> false
        assert!(!mgr.bdd_eval(result, &[false, false, false, false, true, true]));
        // x=00, y=10, z=01: d(x,y)=1, d(x,z)=1 -> false (equal)
        assert!(!mgr.bdd_eval(result, &[false, false, true, false, false, true]));
    }
}
