// lumindd — Simulated annealing variable reordering
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

//! Simulated annealing for BDD variable reordering. Uses the Metropolis
//! criterion to accept or reject random adjacent swaps, with a
//! configurable cooling schedule. This probabilistic approach can escape
//! local minima that greedy sifting gets stuck in.

use crate::manager::Manager;

/// Configuration for simulated annealing reordering.
#[derive(Clone, Debug)]
pub struct AnnealingConfig {
    /// Initial temperature. Higher values accept worse moves more often
    /// at the start. Typical range: 1.0 to 100.0.
    pub initial_temp: f64,

    /// Cooling factor (multiplied each outer iteration). Must be in (0, 1).
    /// Values closer to 1.0 cool more slowly and explore more. Typical: 0.95.
    pub cooling_factor: f64,

    /// Minimum temperature — annealing stops when temp drops below this.
    pub min_temp: f64,

    /// Number of random swap attempts per temperature level.
    /// Typical: 10 * num_vars.
    pub moves_per_temp: usize,

    /// Random seed for reproducibility.
    pub seed: u64,
}

impl Default for AnnealingConfig {
    fn default() -> Self {
        AnnealingConfig {
            initial_temp: 20.0,
            cooling_factor: 0.95,
            min_temp: 0.1,
            moves_per_temp: 0, // 0 means auto (10 * num_vars)
            seed: 0xDEADBEEF_CAFEBABE,
        }
    }
}

/// Simple xorshift64 pseudo-random number generator.
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Rng {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    /// Generate next u64.
    #[inline]
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Generate a random usize in `[0, bound)`.
    #[inline]
    fn next_usize(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }

    /// Generate a random f64 in [0.0, 1.0).
    #[inline]
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

impl Manager {
    /// Run simulated annealing variable reordering with default configuration.
    pub fn anneal_reorder(&mut self) {
        self.anneal_reorder_with_config(&AnnealingConfig::default());
    }

    /// Run simulated annealing with a custom configuration.
    ///
    /// The algorithm:
    /// 1. Start at the current variable ordering.
    /// 2. At each temperature level, perform `moves_per_temp` random
    ///    adjacent swaps.
    /// 3. For each swap, compute the change in total node count.
    ///    - If the swap reduces nodes, always accept it.
    ///    - If the swap increases nodes by `delta`, accept it with
    ///      probability `exp(-delta / temperature)` (Metropolis criterion).
    /// 4. Cool the temperature: `temp *= cooling_factor`.
    /// 5. Stop when temperature drops below `min_temp`.
    /// 6. At the end, restore the best ordering seen.
    pub fn anneal_reorder_with_config(&mut self, config: &AnnealingConfig) {
        let n = self.num_vars as usize;
        if n <= 1 {
            return;
        }

        let moves_per_temp = if config.moves_per_temp == 0 {
            10 * n
        } else {
            config.moves_per_temp
        };

        let mut rng = Rng::new(config.seed);
        let mut temp = config.initial_temp;

        let mut current_size = self.total_live_nodes();
        let mut best_size = current_size;
        let mut best_perm = self.perm.clone();

        // Main annealing loop.
        while temp > config.min_temp {
            let mut accepted = 0usize;

            for _ in 0..moves_per_temp {
                // Pick a random level to swap with its neighbour.
                let level = rng.next_usize(n - 1) as u32;

                // Perform the swap.
                self.swap_adjacent_levels(level);
                let new_size = self.total_live_nodes();

                let delta = new_size as i64 - current_size as i64;

                if delta <= 0 {
                    // Improvement or no change — always accept.
                    current_size = new_size;
                    accepted += 1;
                } else {
                    // Worsening move — accept with Metropolis probability.
                    let prob = (-delta as f64 / temp).exp();
                    if rng.next_f64() < prob {
                        current_size = new_size;
                        accepted += 1;
                    } else {
                        // Reject — undo the swap.
                        self.swap_adjacent_levels(level);
                    }
                }

                // Track best ordering.
                if current_size < best_size {
                    best_size = current_size;
                    best_perm = self.perm.clone();
                }
            }

            // Cool down.
            temp *= config.cooling_factor;

            // Early termination if acceptance rate is very low.
            let acceptance_rate = accepted as f64 / moves_per_temp as f64;
            if acceptance_rate < 0.001 && temp < config.initial_temp * 0.1 {
                break;
            }
        }

        // Restore the best ordering found.
        self.apply_permutation(&best_perm);
        self.cache.clear();
        self.reordered = true;
    }

    /// Simulated annealing with a reheat strategy: after the initial
    /// anneal converges, reheat to `reheat_fraction` of the initial
    /// temperature and anneal again. Repeat for `num_reheats` cycles.
    pub fn anneal_reorder_with_reheat(
        &mut self,
        config: &AnnealingConfig,
        num_reheats: usize,
        reheat_fraction: f64,
    ) {
        let n = self.num_vars as usize;
        if n <= 1 {
            return;
        }

        let mut best_size = self.total_live_nodes();
        let mut best_perm = self.perm.clone();

        // Initial anneal.
        self.anneal_reorder_with_config(config);
        let size = self.total_live_nodes();
        if size < best_size {
            best_size = size;
            best_perm = self.perm.clone();
        }

        // Reheat cycles.
        for cycle in 0..num_reheats {
            let reheat_temp = config.initial_temp
                * reheat_fraction.powi((cycle + 1) as i32);

            if reheat_temp < config.min_temp {
                break;
            }

            let mut reheated_config = config.clone();
            reheated_config.initial_temp = reheat_temp;
            reheated_config.seed = config.seed.wrapping_add(cycle as u64 + 1);

            self.anneal_reorder_with_config(&reheated_config);
            let size = self.total_live_nodes();
            if size < best_size {
                best_size = size;
                best_perm = self.perm.clone();
            }
        }

        // Restore overall best.
        self.apply_permutation(&best_perm);
        self.cache.clear();
        self.reordered = true;
    }

    /// Quick anneal: a fast variant with aggressive cooling for use as
    /// a preprocessing step before sifting.
    pub fn quick_anneal(&mut self) {
        let n = self.num_vars as usize;
        let config = AnnealingConfig {
            initial_temp: 5.0,
            cooling_factor: 0.85,
            min_temp: 0.01,
            moves_per_temp: 5 * n.max(1),
            seed: 0xBEEFCAFE,
        };
        self.anneal_reorder_with_config(&config);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Manager;

    #[test]
    fn test_anneal_basic() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var();
        let x1 = mgr.bdd_new_var();
        let x2 = mgr.bdd_new_var();
        let f = mgr.bdd_and(x0, x1);
        let _g = mgr.bdd_or(f, x2);
        mgr.anneal_reorder();
    }

    #[test]
    fn test_anneal_custom_config() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var();
        let x1 = mgr.bdd_new_var();
        let x2 = mgr.bdd_new_var();
        let x3 = mgr.bdd_new_var();
        let a = mgr.bdd_and(x0, x1);
        let b = mgr.bdd_and(x2, x3);
        let _f = mgr.bdd_or(a, b);

        let config = AnnealingConfig {
            initial_temp: 10.0,
            cooling_factor: 0.9,
            min_temp: 0.5,
            moves_per_temp: 20,
            seed: 42,
        };
        mgr.anneal_reorder_with_config(&config);
    }

    #[test]
    fn test_anneal_with_reheat() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var();
        let x1 = mgr.bdd_new_var();
        let x2 = mgr.bdd_new_var();
        let f = mgr.bdd_and(x0, x1);
        let _g = mgr.bdd_or(f, x2);

        let config = AnnealingConfig {
            initial_temp: 10.0,
            cooling_factor: 0.9,
            min_temp: 0.1,
            moves_per_temp: 15,
            seed: 123,
        };
        mgr.anneal_reorder_with_reheat(&config, 3, 0.5);
    }

    #[test]
    fn test_quick_anneal() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var();
        let x1 = mgr.bdd_new_var();
        let x2 = mgr.bdd_new_var();
        let f = mgr.bdd_and(x0, x1);
        let _g = mgr.bdd_or(f, x2);
        mgr.quick_anneal();
    }

    #[test]
    fn test_rng_basic() {
        let mut rng = Rng::new(42);
        let a = rng.next_u64();
        let b = rng.next_u64();
        assert_ne!(a, b);
        let f = rng.next_f64();
        assert!(f >= 0.0 && f < 1.0);
        let u = rng.next_usize(10);
        assert!(u < 10);
    }
}
