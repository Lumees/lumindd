// lumindd — Genetic algorithm variable reordering
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

//! Genetic algorithm (GA) for BDD variable reordering. Maintains a
//! population of variable orderings and evolves them over generations
//! using selection, crossover (PMX), and mutation (random adjacent swaps).
//! Fitness is the inverse of the total node count under each ordering.

use crate::manager::Manager;

/// Configuration for the genetic algorithm reordering.
#[derive(Clone, Debug)]
pub struct GeneticConfig {
    /// Population size (number of orderings maintained simultaneously).
    pub population_size: usize,

    /// Number of generations to evolve.
    pub generations: usize,

    /// Mutation probability per individual (probability of applying a
    /// random adjacent swap after crossover). Range [0.0, 1.0].
    pub mutation_rate: f64,

    /// Number of random adjacent swaps applied during mutation.
    pub mutation_swaps: usize,

    /// Fraction of the population replaced per generation via crossover.
    /// The rest survive via elitism (best individuals carry over).
    pub crossover_fraction: f64,

    /// Random seed for reproducibility.
    pub seed: u64,
}

impl Default for GeneticConfig {
    fn default() -> Self {
        GeneticConfig {
            population_size: 20,
            generations: 50,
            mutation_rate: 0.3,
            mutation_swaps: 3,
            crossover_fraction: 0.6,
            seed: 0xABCDEF0123456789,
        }
    }
}

/// Simple xorshift64 PRNG.
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Rng {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    #[inline]
    fn next_usize(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }

    #[inline]
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// A single individual in the population: a permutation (variable-to-level
/// mapping) and its fitness (inverse node count).
#[derive(Clone)]
struct Individual {
    /// perm[var] = level for that variable.
    perm: Vec<u32>,
    /// Fitness: 1.0 / node_count. Higher is better.
    fitness: f64,
}

impl Manager {
    /// Run genetic algorithm reordering with default configuration.
    pub fn genetic_reorder(&mut self) {
        self.genetic_reorder_with_config(&GeneticConfig::default());
    }

    /// Run genetic algorithm reordering with a custom configuration.
    ///
    /// Algorithm:
    /// 1. Initialize population with random permutations (plus the
    ///    current ordering).
    /// 2. Evaluate fitness of each individual by applying its permutation
    ///    and counting nodes.
    /// 3. For each generation:
    ///    a. Select parents via roulette wheel (fitness-proportional).
    ///    b. Produce offspring via PMX crossover.
    ///    c. Apply mutation (random adjacent swaps).
    ///    d. Evaluate fitness of offspring.
    ///    e. Merge with old population, keep the best `population_size`.
    /// 4. Apply the best ordering found.
    pub fn genetic_reorder_with_config(&mut self, config: &GeneticConfig) {
        let n = self.num_vars as usize;
        if n <= 1 {
            return;
        }

        let pop_size = config.population_size.max(4);
        let mut rng = Rng::new(config.seed);

        // Save the original permutation.
        let original_perm = self.perm.clone();

        // Initialize population.
        let mut population = Vec::with_capacity(pop_size);

        // Add the current ordering.
        let current_fitness = self.evaluate_perm(&original_perm);
        population.push(Individual {
            perm: original_perm.clone(),
            fitness: current_fitness,
        });

        // Fill the rest with random permutations.
        for _ in 1..pop_size {
            let perm = self.random_permutation(n, &mut rng);
            let fitness = self.evaluate_perm(&perm);
            population.push(Individual { perm, fitness });
        }

        let mut best_individual = population
            .iter()
            .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap())
            .unwrap()
            .clone();

        // Evolution loop.
        for _gen in 0..config.generations {
            let num_offspring =
                (pop_size as f64 * config.crossover_fraction).ceil() as usize;
            let mut offspring = Vec::with_capacity(num_offspring);

            for _ in 0..num_offspring {
                // Select two parents via roulette wheel.
                let parent_a = self.roulette_select(&population, &mut rng);
                let parent_b = self.roulette_select(&population, &mut rng);

                // Crossover (PMX).
                let mut child_perm =
                    self.pmx_crossover(&population[parent_a].perm, &population[parent_b].perm, n, &mut rng);

                // Mutation.
                if rng.next_f64() < config.mutation_rate {
                    self.mutate_permutation(&mut child_perm, config.mutation_swaps, &mut rng);
                }

                let fitness = self.evaluate_perm(&child_perm);
                offspring.push(Individual {
                    perm: child_perm,
                    fitness,
                });
            }

            // Merge populations: take all individuals, sort by fitness,
            // keep the best `pop_size`.
            population.extend(offspring);
            population.sort_by(|a, b| {
                b.fitness.partial_cmp(&a.fitness).unwrap()
            });
            population.truncate(pop_size);

            // Update best.
            if population[0].fitness > best_individual.fitness {
                best_individual = population[0].clone();
            }
        }

        // Apply the best ordering found.
        self.apply_permutation(&best_individual.perm);
        self.cache.clear();
        self.reordered = true;
    }

    // ------------------------------------------------------------------
    // GA operators
    // ------------------------------------------------------------------

    /// Evaluate a permutation by applying it and counting nodes.
    /// Restores the original permutation afterwards.
    fn evaluate_perm(&mut self, perm: &[u32]) -> f64 {
        let saved_perm = self.perm.clone();
        self.apply_permutation(perm);
        let count = self.total_live_nodes();
        self.apply_permutation(&saved_perm);
        if count == 0 {
            f64::MAX
        } else {
            1.0 / count as f64
        }
    }

    /// Generate a random permutation of `[0, n)`.
    fn random_permutation(&self, n: usize, rng: &mut Rng) -> Vec<u32> {
        let mut perm: Vec<u32> = (0..n as u32).collect();
        // Fisher-Yates shuffle.
        for i in (1..n).rev() {
            let j = rng.next_usize(i + 1);
            perm.swap(i, j);
        }
        perm
    }

    /// Roulette wheel selection: pick an individual with probability
    /// proportional to its fitness. Returns the index into `population`.
    fn roulette_select(&self, population: &[Individual], rng: &mut Rng) -> usize {
        let total_fitness: f64 = population.iter().map(|ind| ind.fitness).sum();
        if total_fitness <= 0.0 {
            return rng.next_usize(population.len());
        }

        let threshold = rng.next_f64() * total_fitness;
        let mut cumulative = 0.0;
        for (i, ind) in population.iter().enumerate() {
            cumulative += ind.fitness;
            if cumulative >= threshold {
                return i;
            }
        }
        population.len() - 1
    }

    /// PMX (Partially Mapped Crossover) for permutations.
    ///
    /// 1. Select a random substring from parent A.
    /// 2. Copy that substring into the child.
    /// 3. Fill remaining positions from parent B, resolving conflicts
    ///    via the PMX mapping.
    fn pmx_crossover(
        &self,
        parent_a: &[u32],
        parent_b: &[u32],
        n: usize,
        rng: &mut Rng,
    ) -> Vec<u32> {
        // Select two crossover points.
        let mut pt1 = rng.next_usize(n);
        let mut pt2 = rng.next_usize(n);
        if pt1 > pt2 {
            std::mem::swap(&mut pt1, &mut pt2);
        }
        // Ensure the segment is not the entire permutation.
        if pt2 - pt1 >= n - 1 {
            pt2 = pt1 + n / 2;
            if pt2 >= n {
                pt2 = n - 1;
            }
        }

        let mut child = vec![u32::MAX; n];
        let mut used = vec![false; n];

        // Step 1: Copy segment from parent A.
        for i in pt1..=pt2 {
            child[i] = parent_a[i];
            used[parent_a[i] as usize] = true;
        }

        // Step 2: For each value in the segment of parent B that isn't
        // yet in child, find where to place it using the PMX mapping.
        for i in pt1..=pt2 {
            let val = parent_b[i];
            if used[val as usize] {
                continue; // already placed
            }
            // Follow the mapping chain: parent_b[i] -> parent_a[i],
            // then find where parent_a[i] is in parent_b, etc.
            let mut pos = i;
            loop {
                let mapped = parent_a[pos];
                // Find position of `mapped` in parent_b.
                let mut found = false;
                for j in 0..n {
                    if parent_b[j] == mapped {
                        pos = j;
                        found = true;
                        break;
                    }
                }
                if !found {
                    break;
                }
                if pos < pt1 || pos > pt2 {
                    // Found a free position outside the segment.
                    break;
                }
            }
            child[pos] = val;
            used[val as usize] = true;
        }

        // Step 3: Fill remaining positions from parent B.
        for i in 0..n {
            if child[i] == u32::MAX {
                child[i] = parent_b[i];
                if used[parent_b[i] as usize] {
                    // Conflict — find the first unused value.
                    for v in 0..n as u32 {
                        if !used[v as usize] {
                            child[i] = v;
                            used[v as usize] = true;
                            break;
                        }
                    }
                } else {
                    used[parent_b[i] as usize] = true;
                }
            }
        }

        // Validate permutation (debug only).
        debug_assert!({
            let mut valid = true;
            let mut check = vec![false; n];
            for &v in &child {
                if v as usize >= n || check[v as usize] {
                    valid = false;
                    break;
                }
                check[v as usize] = true;
            }
            valid
        });

        child
    }

    /// Mutate a permutation by performing random adjacent swaps.
    fn mutate_permutation(
        &self,
        perm: &mut [u32],
        num_swaps: usize,
        rng: &mut Rng,
    ) {
        let n = perm.len();
        if n <= 1 {
            return;
        }
        for _ in 0..num_swaps {
            let pos = rng.next_usize(n - 1);
            perm.swap(pos, pos + 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Manager;

    #[test]
    fn test_genetic_basic() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var();
        let x1 = mgr.bdd_new_var();
        let x2 = mgr.bdd_new_var();
        let f = mgr.bdd_and(x0, x1);
        let _g = mgr.bdd_or(f, x2);
        mgr.genetic_reorder();
    }

    #[test]
    fn test_genetic_custom_config() {
        let mut mgr = Manager::new();
        let x0 = mgr.bdd_new_var();
        let x1 = mgr.bdd_new_var();
        let x2 = mgr.bdd_new_var();
        let x3 = mgr.bdd_new_var();
        let a = mgr.bdd_and(x0, x1);
        let b = mgr.bdd_and(x2, x3);
        let _f = mgr.bdd_or(a, b);

        let config = GeneticConfig {
            population_size: 10,
            generations: 20,
            mutation_rate: 0.5,
            mutation_swaps: 2,
            crossover_fraction: 0.5,
            seed: 42,
        };
        mgr.genetic_reorder_with_config(&config);
    }

    #[test]
    fn test_pmx_produces_valid_permutation() {
        let mgr = Manager::with_capacity(5, 0, 10);
        let parent_a: Vec<u32> = vec![0, 1, 2, 3, 4];
        let parent_b: Vec<u32> = vec![4, 3, 2, 1, 0];
        let mut rng = Rng::new(123);
        let child = mgr.pmx_crossover(&parent_a, &parent_b, 5, &mut rng);

        let mut seen = vec![false; 5];
        for &v in &child {
            assert!((v as usize) < 5, "value out of range: {}", v);
            assert!(!seen[v as usize], "duplicate value: {}", v);
            seen[v as usize] = true;
        }
    }

    #[test]
    fn test_roulette_select() {
        let mgr = Manager::new();
        let population = vec![
            Individual {
                perm: vec![0, 1, 2],
                fitness: 0.1,
            },
            Individual {
                perm: vec![2, 1, 0],
                fitness: 0.9,
            },
        ];
        let mut rng = Rng::new(42);
        let mut counts = [0usize; 2];
        for _ in 0..1000 {
            let idx = mgr.roulette_select(&population, &mut rng);
            counts[idx] += 1;
        }
        // The second individual (fitness 0.9) should be selected much more often.
        assert!(counts[1] > counts[0]);
    }
}
