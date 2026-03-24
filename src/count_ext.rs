// lumindd — Extended precision minterm counting
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

//! Extended-precision and arbitrary-precision minterm counting for BDDs.
//!
//! These methods mirror [`Manager::bdd_count_minterm`] but use
//! [`EpDouble`] or [`ApInt`] arithmetic to avoid overflow and rounding.

use std::collections::HashMap;

use crate::apa::ApInt;
use crate::epd::EpDouble;
use crate::manager::Manager;
use crate::node::NodeId;

impl Manager {
    // ==================================================================
    // Extended Precision Double counting
    // ==================================================================

    /// Count the number of minterms of a BDD using extended precision doubles.
    ///
    /// This is equivalent to [`bdd_count_minterm`](Manager::bdd_count_minterm)
    /// but will not overflow even for BDDs with up to ~2 billion variables
    /// (the `i32` exponent range).
    pub fn bdd_count_minterm_epd(&self, f: NodeId, num_vars: u32) -> EpDouble {
        let mut cache: HashMap<u32, EpDouble> = HashMap::new();
        let raw_count = self.count_minterm_epd_rec(f, num_vars, &mut cache);

        // Scale by 2^(root_level) for variables above the root.
        let root_level = if f.is_constant() {
            num_vars
        } else {
            self.level(f.regular())
        };
        raw_count * EpDouble::two_power(root_level as i32)
    }

    /// Recursive EPD minterm counting — same algorithm as the f64 version.
    fn count_minterm_epd_rec(
        &self,
        f: NodeId,
        num_vars: u32,
        cache: &mut HashMap<u32, EpDouble>,
    ) -> EpDouble {
        if f.is_one() {
            return EpDouble::one();
        }
        if f.is_zero() {
            return EpDouble::zero();
        }

        let reg = f.regular();
        let key = reg.raw_index();

        if let Some(&cached) = cache.get(&key) {
            return if f.is_complemented() {
                let level = self.level(reg);
                let total = EpDouble::two_power((num_vars - level) as i32);
                total - cached
            } else {
                cached
            };
        }

        let f_level = self.level(reg);
        let (t, e) = (self.then_child(f), self.else_child(f));

        let t_level = if t.is_constant() {
            num_vars
        } else {
            self.level(t.regular())
        };
        let e_level = if e.is_constant() {
            num_vars
        } else {
            self.level(e.regular())
        };

        let t_count = self.count_minterm_epd_rec(t, num_vars, cache);
        let e_count = self.count_minterm_epd_rec(e, num_vars, cache);

        let t_factor = EpDouble::two_power((t_level - f_level - 1) as i32);
        let e_factor = EpDouble::two_power((e_level - f_level - 1) as i32);

        let result = t_count * t_factor + e_count * e_factor;

        // Cache the result for the regular (non-complemented) node.
        let regular_result = if f.is_complemented() {
            let total = EpDouble::two_power((num_vars - f_level) as i32);
            total - result
        } else {
            result
        };
        cache.insert(key, regular_result);

        result
    }

    // ==================================================================
    // Arbitrary Precision Integer counting
    // ==================================================================

    /// Count the number of minterms of a BDD using arbitrary precision integers.
    ///
    /// Returns the exact integer count — no rounding, no overflow, ever.
    /// This is the most precise counting method but is slower for very
    /// large BDDs due to big-integer arithmetic.
    pub fn bdd_count_minterm_apa(&self, f: NodeId, num_vars: u32) -> ApInt {
        let mut cache: HashMap<u32, ApInt> = HashMap::new();
        let raw_count = self.count_minterm_apa_rec(f, num_vars, &mut cache);

        // Scale by 2^(root_level) for variables above the root.
        let root_level = if f.is_constant() {
            num_vars
        } else {
            self.level(f.regular())
        };
        &raw_count << root_level
    }

    /// Recursive APA minterm counting — same algorithm as the f64 version.
    fn count_minterm_apa_rec(
        &self,
        f: NodeId,
        num_vars: u32,
        cache: &mut HashMap<u32, ApInt>,
    ) -> ApInt {
        if f.is_one() {
            return ApInt::one();
        }
        if f.is_zero() {
            return ApInt::zero();
        }

        let reg = f.regular();
        let key = reg.raw_index();

        if let Some(cached) = cache.get(&key) {
            return if f.is_complemented() {
                let level = self.level(reg);
                let total = ApInt::two_power(num_vars - level);
                &total - cached
            } else {
                cached.clone()
            };
        }

        let f_level = self.level(reg);
        let (t, e) = (self.then_child(f), self.else_child(f));

        let t_level = if t.is_constant() {
            num_vars
        } else {
            self.level(t.regular())
        };
        let e_level = if e.is_constant() {
            num_vars
        } else {
            self.level(e.regular())
        };

        let t_count = self.count_minterm_apa_rec(t, num_vars, cache);
        let e_count = self.count_minterm_apa_rec(e, num_vars, cache);

        let t_shifted = &t_count << (t_level - f_level - 1);
        let e_shifted = &e_count << (e_level - f_level - 1);

        let result = &t_shifted + &e_shifted;

        // Cache the result for the regular (non-complemented) node.
        let regular_result = if f.is_complemented() {
            let total = ApInt::two_power(num_vars - f_level);
            &total - &result
        } else {
            result.clone()
        };
        cache.insert(key, regular_result);

        result
    }
}

#[cfg(test)]
mod tests {
    use crate::Manager;

    #[test]
    fn test_epd_count_basic() {
        let mut mgr = Manager::new();
        let _x = mgr.bdd_new_var();
        let _y = mgr.bdd_new_var();

        // ZERO has 0 minterms
        let count = mgr.bdd_count_minterm_epd(mgr.zero(), 2);
        assert_eq!(count.to_f64().unwrap(), 0.0);

        // ONE has 2^n = 4 minterms over 2 variables
        let count = mgr.bdd_count_minterm_epd(mgr.one(), 2);
        assert!((count.to_f64().unwrap() - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_apa_count_matches_f64() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let z = mgr.bdd_new_var();

        // f = x AND y  => 2 minterms with 3 vars
        let f = mgr.bdd_and(x, y);
        let count = mgr.bdd_count_minterm_apa(f, 3);
        assert_eq!(count.to_string(), "2");

        // g = x OR y OR z => 7 minterms with 3 vars
        let xy = mgr.bdd_or(x, y);
        let g = mgr.bdd_or(xy, z);
        let count = mgr.bdd_count_minterm_apa(g, 3);
        assert_eq!(count.to_string(), "7");

        // NOT x => 4 minterms with 3 vars
        let nx = mgr.bdd_not(x);
        let count = mgr.bdd_count_minterm_apa(nx, 3);
        assert_eq!(count.to_string(), "4");

        // Constants
        let one = mgr.one();
        let zero = mgr.zero();
        assert_eq!(mgr.bdd_count_minterm_apa(one, 3).to_string(), "8");
        assert_eq!(mgr.bdd_count_minterm_apa(zero, 3).to_string(), "0");
    }

    #[test]
    fn test_apa_exact_large() {
        // Verify that APA gives exact results for moderately large variable counts.
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();

        // Just variable x with 100 total vars: minterms = 2^99
        let count = mgr.bdd_count_minterm_apa(x, 100);
        let expected = crate::apa::ApInt::two_power(99);
        assert_eq!(count, expected);
    }
}
