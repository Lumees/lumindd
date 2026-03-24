// lumindd — BDD correlation and agreement analysis
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::HashMap;

use crate::manager::Manager;
use crate::node::NodeId;

impl Manager {
    /// Compute the fraction of minterms on which `f` and `g` agree.
    ///
    /// The result is in [0.0, 1.0]. Uses: count_minterm(f XNOR g) / 2^num_vars.
    /// A correlation of 1.0 means f and g are identical; 0.0 means they are
    /// complements; 0.5 means they agree on exactly half the minterms.
    pub fn bdd_correlation(&mut self, f: NodeId, g: NodeId, num_vars: u32) -> f64 {
        let xnor = self.bdd_xnor(f, g);
        let agree_count = self.bdd_count_minterm(xnor, num_vars);
        let total = 2.0f64.powi(num_vars as i32);
        if total == 0.0 {
            return 1.0;
        }
        agree_count / total
    }

    /// Weighted correlation where `prob[i]` is the probability of variable `i`
    /// being true.
    ///
    /// This computes the probability that `f` and `g` agree under the given
    /// variable distribution. Traverses both BDDs simultaneously.
    pub fn bdd_correlation_weights(
        &mut self,
        f: NodeId,
        g: NodeId,
        prob: &[f64],
    ) -> f64 {
        // First build XNOR, then compute weighted probability of the result
        let xnor = self.bdd_xnor(f, g);
        let mut cache: HashMap<(u32, bool), f64> = HashMap::new();
        self.weighted_prob_rec(xnor, prob, &mut cache)
    }

    /// Recursively compute the probability that the BDD `f` evaluates to true
    /// under the given variable probabilities.
    fn weighted_prob_rec(
        &self,
        f: NodeId,
        prob: &[f64],
        cache: &mut HashMap<(u32, bool), f64>,
    ) -> f64 {
        if f.is_one() {
            return 1.0;
        }
        if f.is_zero() {
            return 0.0;
        }

        let key = (f.raw_index(), f.is_complemented());
        if let Some(&cached) = cache.get(&key) {
            return cached;
        }

        let var = self.var_index(f.regular()) as usize;
        let p = if var < prob.len() { prob[var] } else { 0.5 };

        let t = self.then_child(f);
        let e = self.else_child(f);

        let t_prob = self.weighted_prob_rec(t, prob, cache);
        let e_prob = self.weighted_prob_rec(e, prob, cache);

        let result = p * t_prob + (1.0 - p) * e_prob;
        cache.insert(key, result);
        result
    }
}

#[cfg(test)]
mod tests {
    use crate::Manager;

    #[test]
    fn correlation_identical() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y);
        let corr = mgr.bdd_correlation(f, f, 2);
        assert!((corr - 1.0).abs() < 1e-10);
    }

    #[test]
    fn correlation_complement() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y);
        let g = mgr.bdd_not(f);
        let corr = mgr.bdd_correlation(f, g, 2);
        assert!((corr - 0.0).abs() < 1e-10);
    }

    #[test]
    fn correlation_half() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let _y = mgr.bdd_new_var();
        // f = x, g = ONE => agree when x=1 (2 minterms out of 4)
        // Actually f=x agrees with g=1 when x=1, which is 2/4 = 0.5
        let corr = mgr.bdd_correlation(x, mgr.one(), 2);
        assert!((corr - 0.5).abs() < 1e-10);
    }

    #[test]
    fn weighted_correlation_uniform() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y);
        // Uniform weights = standard correlation
        let prob = vec![0.5, 0.5];
        let wc = mgr.bdd_correlation_weights(f, f, &prob);
        assert!((wc - 1.0).abs() < 1e-10);
    }

    #[test]
    fn weighted_correlation_biased() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let _y = mgr.bdd_new_var();
        // f = x, g = ONE
        // XNOR(x, 1) = x
        // prob(x=1) with prob[0]=0.8 => weighted prob = 0.8
        let prob = vec![0.8, 0.5];
        let wc = mgr.bdd_correlation_weights(x, mgr.one(), &prob);
        assert!((wc - 0.8).abs() < 1e-10);
    }

    #[test]
    fn correlation_constants() {
        let mut mgr = Manager::new();
        let one = mgr.one();
        let zero = mgr.zero();
        // ONE vs ONE => correlation 1.0
        let c1 = mgr.bdd_correlation(one, one, 0);
        assert!((c1 - 1.0).abs() < 1e-10);
        // ONE vs ZERO => correlation 0.0
        let c2 = mgr.bdd_correlation(one, zero, 0);
        assert!((c2 - 0.0).abs() < 1e-10);
    }
}
