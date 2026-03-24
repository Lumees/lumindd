// lumindd — ADD Walsh matrix, Hadamard, and residue operations
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

//! Walsh matrix, Hadamard transform, residue, and XOR indicator ADDs.

use crate::manager::Manager;
use crate::node::NodeId;

impl Manager {
    /// Build the Walsh matrix W(x, y) as an ADD.
    ///
    /// The Walsh matrix is defined as W\[i\]\[j\] = (-1)^popcount(i AND j).
    /// It is constructed using the Kronecker product of 2x2 Walsh kernels:
    ///
    /// ```text
    /// [[1,  1],
    ///  [1, -1]]
    /// ```
    ///
    /// For each pair of variables (x_k, y_k), the kernel is built as an ADD
    /// and the results are multiplied together (Kronecker product via ADD times).
    ///
    /// # Arguments
    /// * `x_vars` — variable indices representing row bits (MSB first)
    /// * `y_vars` — variable indices representing column bits (MSB first)
    ///
    /// # Panics
    /// Panics if `x_vars` and `y_vars` have different lengths.
    pub fn add_walsh(&mut self, x_vars: &[u16], y_vars: &[u16]) -> NodeId {
        assert_eq!(
            x_vars.len(),
            y_vars.len(),
            "Walsh matrix requires equal numbers of x and y variables"
        );

        if x_vars.is_empty() {
            return NodeId::ONE; // scalar 1.0
        }

        // Build a 2x2 Walsh kernel for each variable pair, then multiply them.
        // Kernel for (xi, yi): 1.0 everywhere except when xi=1 AND yi=1 => -1.0
        //
        // In ADD form:
        //   if xi then (if yi then -1.0 else 1.0) else (if yi then 1.0 else 1.0)
        // Simplifies to:
        //   if xi then (if yi then -1.0 else 1.0) else 1.0

        let one = NodeId::ONE;
        let neg_one = self.add_const(-1.0);

        let mut result = one; // accumulate product

        for (&xv, &yv) in x_vars.iter().zip(y_vars.iter()) {
            // Ensure variables exist
            while self.num_vars <= xv {
                self.bdd_new_var();
            }
            while self.num_vars <= yv {
                self.bdd_new_var();
            }

            // Build: if yi then -1.0 else 1.0
            let yi_branch = self.add_unique_inter(yv, neg_one, one);
            // Build: if xi then yi_branch else 1.0
            let kernel = self.add_unique_inter(xv, yi_branch, one);

            result = self.add_times(result, kernel);
        }

        result
    }

    /// Build the Hadamard matrix as an ADD (normalized Walsh).
    ///
    /// H = W / 2^n where W is the Walsh matrix and n = `num_vars`.
    /// The Hadamard matrix is symmetric and orthogonal.
    ///
    /// This allocates `num_vars` pairs of fresh variables: x0..x_{n-1} for rows
    /// and x_n..x_{2n-1} for columns.
    ///
    /// # Returns
    /// The ADD representing the Hadamard matrix.
    pub fn add_hadamard(&mut self, num_vars: u32) -> NodeId {
        let n = num_vars as usize;
        let base = self.num_vars;

        // Create row and column variables
        let x_vars: Vec<u16> = (0..n).map(|i| base + i as u16).collect();
        let y_vars: Vec<u16> = (0..n).map(|i| base + n as u16 + i as u16).collect();

        // Ensure all variables exist
        let max_var = base + 2 * n as u16;
        while self.num_vars < max_var {
            self.bdd_new_var();
        }

        let walsh = self.add_walsh(&x_vars, &y_vars);

        // Normalize: divide by 2^n
        let scale = 1.0 / (1u64 << n) as f64;
        let scale_node = self.add_const(scale);
        self.add_times(walsh, scale_node)
    }

    /// Build an ADD representing `(integer value of x_vars) mod modulus`.
    ///
    /// The ADD maps each assignment of the binary variables `x_vars` to
    /// the value `(sum_i x_vars[i] * 2^(n-1-i)) mod modulus`.
    ///
    /// Variables in `x_vars` are ordered MSB first.
    ///
    /// # Arguments
    /// * `x_vars` — variable indices representing bits (MSB first)
    /// * `modulus` — the modulus (must be > 0)
    ///
    /// # Panics
    /// Panics if `modulus` is 0.
    pub fn add_residue(&mut self, x_vars: &[u16], modulus: u32) -> NodeId {
        assert!(modulus > 0, "modulus must be positive");

        if x_vars.is_empty() {
            return self.add_const(0.0);
        }

        let n = x_vars.len();

        // Ensure all variables exist
        for &v in x_vars {
            while self.num_vars <= v {
                self.bdd_new_var();
            }
        }

        // Build bottom-up. For each possible accumulated residue value r,
        // process bits from LSB to MSB. At each step, the ADD maps each
        // path to the current partial residue.
        //
        // Start from the LSB (x_vars[n-1]) and work up.
        // At bit position i (counting from LSB=0), the bit weight is 2^i.
        //
        // We build a vector of ADD constants for residue values 0..modulus-1,
        // then iteratively construct the ADD from LSB to MSB.

        let m = modulus as usize;

        // Terminal ADDs for each residue class
        let residue_consts: Vec<NodeId> = (0..m)
            .map(|r| self.add_const(r as f64))
            .collect();

        // Start with the identity: each residue value r maps to constant r.
        // This represents the "result so far" for bits processed.
        // Initially (no bits processed), all paths have residue 0.
        // We'll build the ADD from LSB to MSB.

        // For each residue r in 0..m, `current[r]` is the ADD for the sub-problem
        // "given that the partial sum from lower bits is r, what's the final residue?"
        // Initially (no bits below), partial sum r means final answer r.
        let mut current: Vec<NodeId> = residue_consts.clone();

        // Process bits from LSB (index n-1) to MSB (index 0)
        for bit_pos in 0..n {
            let var_idx = x_vars[n - 1 - bit_pos];
            let weight = (1u64 << bit_pos) % modulus as u64;
            let w = weight as usize;

            let mut next = Vec::with_capacity(m);
            for r in 0..m {
                // If this bit is 0: partial residue stays r
                let else_child = current[r];
                // If this bit is 1: partial residue becomes (r + weight) mod m
                let then_child = current[(r + w) % m];

                if then_child == else_child {
                    next.push(then_child);
                } else {
                    let node = self.add_unique_inter(var_idx, then_child, else_child);
                    next.push(node);
                }
            }
            current = next;
        }

        // The result is current[0]: starting from partial sum 0
        current[0]
    }

    /// Build an ADD that is 1.0 where x XOR y = 0 (bitwise equality), 0.0 elsewhere.
    ///
    /// For variable pairs (x_i, y_i), the ADD evaluates to 1.0 if and only if
    /// x_i == y_i for all i.
    ///
    /// # Arguments
    /// * `x_vars` — first set of variable indices
    /// * `y_vars` — second set of variable indices (same length as x_vars)
    ///
    /// # Panics
    /// Panics if `x_vars` and `y_vars` have different lengths.
    pub fn add_xor_indicator(&mut self, x_vars: &[u16], y_vars: &[u16]) -> NodeId {
        assert_eq!(
            x_vars.len(),
            y_vars.len(),
            "XOR indicator requires equal numbers of x and y variables"
        );

        if x_vars.is_empty() {
            return NodeId::ONE; // vacuously true
        }

        let one = NodeId::ONE;
        let zero = self.add_zero();

        // Ensure all variables exist
        for &v in x_vars.iter().chain(y_vars.iter()) {
            while self.num_vars <= v {
                self.bdd_new_var();
            }
        }

        // Build product of per-bit equality indicators.
        // For each pair (xi, yi), the equality indicator is:
        //   if xi then (if yi then 1 else 0) else (if yi then 0 else 1)
        let mut result = one;

        for (&xv, &yv) in x_vars.iter().zip(y_vars.iter()) {
            // if yi then 1 else 0
            let yi_high = self.add_unique_inter(yv, one, zero);
            // if yi then 0 else 1
            let yi_low = self.add_unique_inter(yv, zero, one);
            // if xi then (yi==1 -> 1, yi==0 -> 0) else (yi==1 -> 0, yi==0 -> 1)
            let eq_bit = self.add_unique_inter(xv, yi_high, yi_low);

            result = self.add_times(result, eq_bit);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walsh_1bit() {
        let mut mgr = Manager::new();
        let walsh = mgr.add_walsh(&[0], &[1]);

        // Enumerate: (x=0,y=0)->1, (x=0,y=1)->1, (x=1,y=0)->1, (x=1,y=1)->-1
        let vals = enumerate_2var_add(&mut mgr, walsh, 0, 1);
        assert_eq!(vals, vec![1.0, 1.0, 1.0, -1.0]);
    }

    #[test]
    fn walsh_2bit() {
        let mut mgr = Manager::new();
        // 2-bit Walsh matrix: 4x4
        let walsh = mgr.add_walsh(&[0, 1], &[2, 3]);

        // W[0][0..3] = [1, 1, 1, 1]
        // W[1][0..3] = [1, -1, 1, -1]
        // W[2][0..3] = [1, 1, -1, -1]
        // W[3][0..3] = [1, -1, -1, 1]
        let vals = enumerate_4var_add(&mut mgr, walsh, &[0, 1, 2, 3]);
        let expected = vec![
            1.0, 1.0, 1.0, 1.0,    // row 0
            1.0, -1.0, 1.0, -1.0,  // row 1
            1.0, 1.0, -1.0, -1.0,  // row 2
            1.0, -1.0, -1.0, 1.0,  // row 3
        ];
        assert_eq!(vals, expected);
    }

    #[test]
    fn hadamard_1bit() {
        let mut mgr = Manager::new();
        let h = mgr.add_hadamard(1);
        // H = W / 2 => [[0.5, 0.5], [0.5, -0.5]]
        let base = mgr.num_vars() - 2;
        let vals = enumerate_2var_add(&mut mgr, h, base, base + 1);
        assert_eq!(vals, vec![0.5, 0.5, 0.5, -0.5]);
    }

    #[test]
    fn residue_mod3() {
        let mut mgr = Manager::new();
        // 3-bit number mod 3, vars [0,1,2] (MSB first)
        let r = mgr.add_residue(&[0, 1, 2], 3);
        // Values: 0%3=0, 1%3=1, 2%3=2, 3%3=0, 4%3=1, 5%3=2, 6%3=0, 7%3=1
        let vals = enumerate_3var_add(&mut mgr, r, &[0, 1, 2]);
        assert_eq!(vals, vec![0.0, 1.0, 2.0, 0.0, 1.0, 2.0, 0.0, 1.0]);
    }

    #[test]
    fn residue_mod2() {
        let mut mgr = Manager::new();
        let r = mgr.add_residue(&[0, 1], 2);
        // 2-bit: 0%2=0, 1%2=1, 2%2=0, 3%2=1
        let vals = enumerate_2var_add(&mut mgr, r, 0, 1);
        assert_eq!(vals, vec![0.0, 1.0, 0.0, 1.0]);
    }

    #[test]
    fn xor_indicator_1bit() {
        let mut mgr = Manager::new();
        let ind = mgr.add_xor_indicator(&[0], &[1]);
        // 1 when x==y: (0,0)->1, (0,1)->0, (1,0)->0, (1,1)->1
        let vals = enumerate_2var_add(&mut mgr, ind, 0, 1);
        assert_eq!(vals, vec![1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn xor_indicator_2bit() {
        let mut mgr = Manager::new();
        let ind = mgr.add_xor_indicator(&[0, 1], &[2, 3]);
        let vals = enumerate_4var_add(&mut mgr, ind, &[0, 1, 2, 3]);
        // Should be 1.0 on diagonal (i==j), 0.0 elsewhere
        for i in 0..4u32 {
            for j in 0..4u32 {
                let idx = (i * 4 + j) as usize;
                if i == j {
                    assert_eq!(vals[idx], 1.0, "expected 1.0 at ({},{})", i, j);
                } else {
                    assert_eq!(vals[idx], 0.0, "expected 0.0 at ({},{})", i, j);
                }
            }
        }
    }

    #[test]
    fn walsh_empty() {
        let mut mgr = Manager::new();
        let w = mgr.add_walsh(&[], &[]);
        assert_eq!(mgr.add_value(w), Some(1.0));
    }

    #[test]
    fn residue_single_var() {
        let mut mgr = Manager::new();
        // Single bit mod 1 should always be 0
        let r = mgr.add_residue(&[0], 1);
        let vals = enumerate_1var_add(&mut mgr, r, 0);
        assert_eq!(vals, vec![0.0, 0.0]);
    }

    // ---- helpers for test enumeration ----

    fn evaluate_add(mgr: &Manager, node: NodeId, assignment: &[(u16, bool)]) -> f64 {
        let mut current = node;
        loop {
            if let Some(v) = mgr.add_value(current) {
                return v;
            }
            let var = mgr.var_index(current);
            let n = mgr.node(current);
            let val = assignment.iter().find(|(vi, _)| *vi == var).unwrap().1;
            current = if val { n.then_child() } else { n.else_child() };
        }
    }

    fn enumerate_1var_add(mgr: &mut Manager, node: NodeId, v0: u16) -> Vec<f64> {
        let mut result = Vec::new();
        for a in 0..2u32 {
            let assignment = vec![(v0, a & 1 != 0)];
            result.push(evaluate_add(mgr, node, &assignment));
        }
        result
    }

    fn enumerate_2var_add(mgr: &mut Manager, node: NodeId, v0: u16, v1: u16) -> Vec<f64> {
        let mut result = Vec::new();
        for a in 0..4u32 {
            let assignment = vec![
                (v0, a & 2 != 0),
                (v1, a & 1 != 0),
            ];
            result.push(evaluate_add(mgr, node, &assignment));
        }
        result
    }

    fn enumerate_3var_add(mgr: &mut Manager, node: NodeId, vars: &[u16]) -> Vec<f64> {
        let mut result = Vec::new();
        let n = vars.len();
        for a in 0..(1u32 << n) {
            let assignment: Vec<(u16, bool)> = vars
                .iter()
                .enumerate()
                .map(|(i, &v)| (v, a & (1 << (n - 1 - i)) != 0))
                .collect();
            result.push(evaluate_add(mgr, node, &assignment));
        }
        result
    }

    fn enumerate_4var_add(mgr: &mut Manager, node: NodeId, vars: &[u16]) -> Vec<f64> {
        enumerate_3var_add(mgr, node, vars) // same logic, just more bits
    }
}
