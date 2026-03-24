// lumindd — Signature-based BDD simulation
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use crate::manager::Manager;
use crate::node::NodeId;

impl Manager {
    /// Compute a hash-based signature for a BDD by evaluating it on
    /// `num_samples` pseudo-random input vectors.
    ///
    /// Two BDDs with different signatures are guaranteed to represent
    /// different Boolean functions. Two BDDs with the same signature
    /// are likely (but not guaranteed) to be equivalent.
    ///
    /// The pseudo-random vectors are generated deterministically using
    /// a simple hash so results are reproducible.
    pub fn bdd_signature(&self, f: NodeId, num_samples: u32) -> u64 {
        let n = self.num_vars as usize;
        let mut sig: u64 = 0;
        let mut assignment = vec![false; n];

        for sample in 0..num_samples {
            // Generate a deterministic pseudo-random input vector from the sample index.
            let mut hash = sample as u64;
            for var in 0..n {
                // Simple per-variable hash to decide true/false
                hash = hash.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                assignment[var] = (hash >> 33) & 1 != 0;
            }

            let result = self.bdd_eval(f, &assignment);
            if result {
                // Mix the sample index into the signature
                let bit_hash = (sample as u64)
                    .wrapping_mul(0x9E3779B97F4A7C15)
                    .wrapping_add(0x6A09E667F3BCC908);
                sig ^= bit_hash;
            }
        }

        sig
    }

    /// Quick probabilistic equivalence check.
    ///
    /// Returns `true` if `f` and `g` produce the same output on all
    /// `num_samples` pseudo-random input vectors. If this returns `false`,
    /// the functions are definitely different. If `true`, they are likely
    /// (but not certainly) equivalent.
    pub fn bdd_signatures_match(&self, f: NodeId, g: NodeId, num_samples: u32) -> bool {
        let n = self.num_vars as usize;
        let mut assignment = vec![false; n];

        for sample in 0..num_samples {
            let mut hash = sample as u64;
            for var in 0..n {
                hash = hash.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                assignment[var] = (hash >> 33) & 1 != 0;
            }

            let rf = self.bdd_eval(f, &assignment);
            let rg = self.bdd_eval(g, &assignment);
            if rf != rg {
                return false;
            }
        }

        true
    }

    /// Evaluate the BDD on a batch of input vectors.
    ///
    /// Each element of `inputs` is a complete variable assignment (a `Vec<bool>`
    /// of length >= `num_vars()`). Returns a `Vec<bool>` of the same length
    /// as `inputs`, containing the evaluation result for each vector.
    pub fn bdd_simulate(&self, f: NodeId, inputs: &[Vec<bool>]) -> Vec<bool> {
        inputs.iter().map(|assignment| self.bdd_eval(f, assignment)).collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::Manager;

    #[test]
    fn signature_identical_bdds() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y);
        let g = mgr.bdd_and(x, y);
        assert_eq!(
            mgr.bdd_signature(f, 1000),
            mgr.bdd_signature(g, 1000)
        );
    }

    #[test]
    fn signature_different_bdds() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y);
        let g = mgr.bdd_or(x, y);
        // These are different functions, signatures should differ
        // (with high probability for 1000 samples)
        assert_ne!(
            mgr.bdd_signature(f, 1000),
            mgr.bdd_signature(g, 1000)
        );
    }

    #[test]
    fn signatures_match_equivalent() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        // De Morgan: !(x AND y) == !x OR !y
        let f = mgr.bdd_nand(x, y);
        let g = mgr.bdd_or(x.not(), y.not());
        assert!(mgr.bdd_signatures_match(f, g, 1000));
    }

    #[test]
    fn signatures_mismatch_different() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y);
        let g = mgr.bdd_xor(x, y);
        assert!(!mgr.bdd_signatures_match(f, g, 1000));
    }

    #[test]
    fn simulate_basic() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y);

        let inputs = vec![
            vec![false, false],
            vec![false, true],
            vec![true, false],
            vec![true, true],
        ];
        let results = mgr.bdd_simulate(f, &inputs);
        assert_eq!(results, vec![false, false, false, true]);
    }

    #[test]
    fn simulate_or() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_or(x, y);

        let inputs = vec![
            vec![false, false],
            vec![false, true],
            vec![true, false],
            vec![true, true],
        ];
        let results = mgr.bdd_simulate(f, &inputs);
        assert_eq!(results, vec![false, true, true, true]);
    }

    #[test]
    fn simulate_empty() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let _y = mgr.bdd_new_var();
        let inputs: Vec<Vec<bool>> = Vec::new();
        let results = mgr.bdd_simulate(x, &inputs);
        assert!(results.is_empty());
    }

    #[test]
    fn signature_constants() {
        let mgr = Manager::new();
        let sig_one = mgr.bdd_signature(mgr.one(), 100);
        let sig_zero = mgr.bdd_signature(mgr.zero(), 100);
        // ONE and ZERO must have different signatures
        assert_ne!(sig_one, sig_zero);
    }
}
