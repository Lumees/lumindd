// lumindd — Utility and query functions
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::{HashMap, HashSet};
use std::io::Write;

use crate::manager::Manager;
use crate::node::NodeId;

impl Manager {
    // ==================================================================
    // Minterm counting
    // ==================================================================

    /// Count the number of minterms (satisfying assignments) of a BDD,
    /// given `num_vars` total variables.
    ///
    /// Returns the number of truth-table rows where the function evaluates to 1.
    pub fn bdd_count_minterm(&self, f: NodeId, num_vars: u32) -> f64 {
        let mut cache: HashMap<u32, f64> = HashMap::new();
        let raw_count = self.count_minterm_rec(f, num_vars, &mut cache);
        // Scale by 2^(root_level) to account for variables above the root
        let root_level = if f.is_constant() {
            num_vars
        } else {
            self.level(f.regular())
        };
        raw_count * 2.0f64.powi(root_level as i32)
    }

    /// Standard BDD minterm counting algorithm.
    ///
    /// For a node at level `l` with children at levels `lt` and `le`:
    ///   count(node) = 2^(lt - l - 1) * count(then) + 2^(le - l - 1) * count(else)
    ///
    /// The factors of 2 account for "skipped" variables between this node and
    /// its children — each skipped variable doubles the number of minterms.
    fn count_minterm_rec(
        &self,
        f: NodeId,
        num_vars: u32,
        cache: &mut HashMap<u32, f64>,
    ) -> f64 {
        if f.is_one() {
            return 1.0;
        }
        if f.is_zero() {
            return 0.0;
        }

        let reg = f.regular();
        let key = reg.raw_index();
        if let Some(&cached) = cache.get(&key) {
            return if f.is_complemented() {
                // The total possible is 2^(num_vars - level_of_this_node),
                // but since we normalize at the end, just negate via complement.
                // Actually: for complemented edges, we compute at the call site.
                // The cache stores the count for the regular node with the
                // assumption that it's called from a specific level. Let's
                // use a simpler approach: cache by (raw_index, is_complemented).
                // But that's wasteful. Instead: complement = total - regular.
                // "total" for a subtree rooted at level l = 2^(num_vars - l).
                let level = self.level(reg);
                let total = 2.0f64.powi((num_vars - level) as i32);
                total - cached
            } else {
                cached
            };
        }

        let f_level = self.level(reg);
        let (t, e) = (self.then_child(f), self.else_child(f));

        // Determine the level of each child (constants are at level num_vars)
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

        let t_count = self.count_minterm_rec(t, num_vars, cache);
        let e_count = self.count_minterm_rec(e, num_vars, cache);

        // Skipped variables between this node and each child multiply by 2
        let t_skip = t_level.saturating_sub(f_level + 1);
        let e_skip = e_level.saturating_sub(f_level + 1);
        let t_factor = 2.0f64.powi(t_skip as i32);
        let e_factor = 2.0f64.powi(e_skip as i32);

        let result = t_count * t_factor + e_count * e_factor;

        // Cache the result for the regular node
        let regular_result = if f.is_complemented() {
            let total = 2.0f64.powi((num_vars - f_level) as i32);
            total - result
        } else {
            result
        };
        cache.insert(key, regular_result);

        result
    }

    // ==================================================================
    // Support (set of variables a BDD depends on)
    // ==================================================================

    /// Get the set of variable indices that f depends on.
    pub fn bdd_support(&self, f: NodeId) -> Vec<u16> {
        let mut support = HashSet::new();
        self.support_rec(f, &mut support);
        let mut result: Vec<u16> = support.into_iter().collect();
        result.sort();
        result
    }

    pub(crate) fn support_rec(&self, f: NodeId, support: &mut HashSet<u16>) {
        if f.is_constant() {
            return;
        }
        let reg = f.regular();
        let var = self.var_index(reg);
        if !support.insert(var) {
            return; // already visited
        }
        self.support_rec(self.raw_then(f), support);
        self.support_rec(self.raw_else(f), support);
    }

    /// Get the number of variables f depends on.
    pub fn bdd_support_size(&self, f: NodeId) -> usize {
        self.bdd_support(f).len()
    }

    // ==================================================================
    // DAG size
    // ==================================================================

    /// Count the number of nodes in the DAG of f.
    pub fn dag_size(&self, f: NodeId) -> usize {
        let mut visited = HashSet::new();
        self.dag_size_rec(f, &mut visited);
        visited.len()
    }

    fn dag_size_rec(&self, f: NodeId, visited: &mut HashSet<u32>) {
        let raw = f.raw_index();
        if !visited.insert(raw) {
            return;
        }
        if f.is_constant() {
            return;
        }
        self.dag_size_rec(self.raw_then(f), visited);
        self.dag_size_rec(self.raw_else(f), visited);
    }

    // ==================================================================
    // Path operations
    // ==================================================================

    /// Count the number of paths from root to ONE terminal.
    pub fn bdd_count_path(&self, f: NodeId) -> f64 {
        let mut cache: HashMap<u32, f64> = HashMap::new();
        self.count_path_rec(f, &mut cache)
    }

    fn count_path_rec(&self, f: NodeId, cache: &mut HashMap<u32, f64>) -> f64 {
        if f.is_one() {
            return 1.0;
        }
        if f.is_zero() {
            return 0.0;
        }
        // For complemented edges: paths(NOT f) counts paths to the ZERO
        // terminal of f, which is total_paths - paths(f). But for path
        // counting through the DAG, we need to traverse the actual structure.
        let key = f.regular().raw_index();
        if let Some(&cached) = cache.get(&key) {
            return cached;
        }
        let t = self.then_child(f);
        let e = self.else_child(f);
        let result = self.count_path_rec(t, cache) + self.count_path_rec(e, cache);
        cache.insert(key, result);
        result
    }

    // ==================================================================
    // Cube iteration
    // ==================================================================

    /// Iterate over all cubes (satisfying assignments) of a BDD.
    ///
    /// Each cube is a `Vec<Option<bool>>` of length `num_vars`.
    /// `None` means the variable is don't-care, `Some(true)` means
    /// positive literal, `Some(false)` means negative literal.
    pub fn bdd_iter_cubes(&self, f: NodeId) -> Vec<Vec<Option<bool>>> {
        let mut cubes = Vec::new();
        if f.is_zero() {
            return cubes;
        }
        let n = self.num_vars as usize;
        let mut path = vec![None; n];
        self.iter_cubes_rec(f, &mut path, &mut cubes);
        cubes
    }

    fn iter_cubes_rec(
        &self,
        f: NodeId,
        path: &mut Vec<Option<bool>>,
        cubes: &mut Vec<Vec<Option<bool>>>,
    ) {
        if f.is_one() {
            cubes.push(path.clone());
            return;
        }
        if f.is_zero() {
            return;
        }

        let var = self.var_index(f.regular()) as usize;
        let t = self.then_child(f);
        let e = self.else_child(f);

        // Then branch (var = true)
        path[var] = Some(true);
        self.iter_cubes_rec(t, path, cubes);

        // Else branch (var = false)
        path[var] = Some(false);
        self.iter_cubes_rec(e, path, cubes);

        // Restore don't-care
        path[var] = None;
    }

    // ==================================================================
    // DOT export
    // ==================================================================

    /// Export a BDD to DOT format for visualization.
    pub fn dump_dot<W: Write>(&self, f: NodeId, out: &mut W) -> std::io::Result<()> {
        writeln!(out, "digraph BDD {{")?;
        writeln!(out, "  rankdir=TB;")?;
        writeln!(out, "  // Terminal nodes")?;
        writeln!(out, "  {{ rank=sink;")?;
        writeln!(out, "    ONE [shape=box, label=\"1\"];")?;
        writeln!(out, "    ZERO [shape=box, label=\"0\"];")?;
        writeln!(out, "  }}")?;

        let mut visited = HashSet::new();
        self.dump_dot_rec(f, out, &mut visited)?;

        writeln!(out, "}}")?;
        Ok(())
    }

    fn dump_dot_rec<W: Write>(
        &self,
        f: NodeId,
        out: &mut W,
        visited: &mut HashSet<u32>,
    ) -> std::io::Result<()> {
        if f.is_constant() {
            return Ok(());
        }

        let raw = f.raw_index();
        if !visited.insert(raw) {
            return Ok(());
        }

        let var = self.var_index(f.regular());
        let t = self.raw_then(f);
        let e = self.raw_else(f);

        writeln!(
            out,
            "  n{} [label=\"x{}\", shape=ellipse];",
            raw, var
        )?;

        // Then edge (solid)
        let t_label = if t.is_one() {
            "ONE".to_string()
        } else if t.is_zero() {
            "ZERO".to_string()
        } else {
            format!("n{}", t.raw_index())
        };
        writeln!(
            out,
            "  n{} -> {} [style=solid{}];",
            raw,
            t_label,
            if t.is_complemented() { ", color=red" } else { "" }
        )?;

        // Else edge (dashed)
        let e_label = if e.is_one() {
            "ONE".to_string()
        } else if e.is_zero() {
            "ZERO".to_string()
        } else {
            format!("n{}", e.raw_index())
        };
        writeln!(
            out,
            "  n{} -> {} [style=dashed{}];",
            raw,
            e_label,
            if e.is_complemented() { ", color=red" } else { "" }
        )?;

        self.dump_dot_rec(t.regular(), out, visited)?;
        self.dump_dot_rec(e.regular(), out, visited)?;

        Ok(())
    }

    // ==================================================================
    // Print minterm
    // ==================================================================

    /// Print all minterms of a BDD.
    pub fn bdd_print_minterms(&self, f: NodeId) {
        let cubes = self.bdd_iter_cubes(f);
        for cube in &cubes {
            let s: String = cube
                .iter()
                .enumerate()
                .map(|(i, v)| match v {
                    Some(true) => format!("x{}=1 ", i),
                    Some(false) => format!("x{}=0 ", i),
                    None => format!("x{}=- ", i),
                })
                .collect();
            println!("{}", s.trim());
        }
    }

    // ==================================================================
    // Evaluation
    // ==================================================================

    /// Evaluate a BDD under a given variable assignment.
    ///
    /// `assignment` must have length >= `num_vars()`. Panics if any
    /// variable index in the BDD exceeds the assignment length.
    pub fn bdd_eval(&self, f: NodeId, assignment: &[bool]) -> bool {
        assert!(
            assignment.len() >= self.num_vars as usize,
            "assignment length {} < num_vars {}",
            assignment.len(),
            self.num_vars
        );
        let mut current = f;
        loop {
            if current.is_one() {
                return true;
            }
            if current.is_zero() {
                return false;
            }
            let var = self.var_index(current.regular()) as usize;
            if assignment[var] {
                current = self.then_child(current);
            } else {
                current = self.else_child(current);
            }
        }
    }

    // ==================================================================
    // Satisfying assignment (pick one)
    // ==================================================================

    /// Find one satisfying assignment (returns None if f is ZERO).
    pub fn bdd_pick_one_cube(&self, f: NodeId) -> Option<Vec<bool>> {
        if f.is_zero() {
            return None;
        }
        let n = self.num_vars as usize;
        let mut assignment = vec![false; n];
        self.pick_one_rec(f, &mut assignment);
        Some(assignment)
    }

    fn pick_one_rec(&self, f: NodeId, assignment: &mut Vec<bool>) {
        if f.is_constant() {
            return;
        }
        let var = self.var_index(f.regular()) as usize;
        let t = self.then_child(f);
        let e = self.else_child(f);

        // Prefer the then-branch if it's satisfiable
        if !t.is_zero() {
            assignment[var] = true;
            self.pick_one_rec(t, assignment);
        } else {
            assignment[var] = false;
            self.pick_one_rec(e, assignment);
        }
    }

    // ==================================================================
    // Statistics
    // ==================================================================

    /// Print manager statistics.
    pub fn print_info(&self) {
        println!("=== lumindd Manager Info ===");
        println!("BDD/ADD variables: {}", self.num_vars);
        println!("ZDD variables:     {}", self.num_zdd_vars);
        println!("Total nodes:       {}", self.nodes.len());
        println!("Dead nodes:        {}", self.dead);
        println!("GC runs:           {}", self.gc_count);
        let (hits, misses) = self.cache_stats();
        let total = hits + misses;
        let rate = if total > 0 {
            hits as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        println!("Cache hits:        {} ({:.1}%)", hits, rate);
        println!("Cache misses:      {}", misses);
        println!("============================");
    }
}
