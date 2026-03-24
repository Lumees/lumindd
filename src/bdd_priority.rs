// lumindd — BDD priority and comparison functions
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use crate::manager::Manager;
use crate::node::NodeId;

impl Manager {
    /// BDD for `x > y` where x and y are n-bit unsigned integers.
    ///
    /// `x_vars[0]` is the MSB of x, `x_vars[n-1]` is the LSB.
    /// Same convention for `y_vars`.
    pub fn bdd_inequality(
        &mut self,
        n: u32,
        x_vars: &[u16],
        y_vars: &[u16],
    ) -> NodeId {
        assert!(x_vars.len() >= n as usize);
        assert!(y_vars.len() >= n as usize);

        // Build from LSB to MSB.
        // At each bit position i, x > y iff:
        //   x[i] > y[i], OR (x[i] == y[i] AND x[i-1..0] > y[i-1..0])
        // Starting from the LSB with the "carry" being ZERO (no inequality yet).
        let mut result = NodeId::ZERO; // x[n-1..n-1] > y[n-1..n-1] starts false

        for i in (0..n as usize).rev() {
            let xi = self.bdd_ith_var(x_vars[i]);
            let yi = self.bdd_ith_var(y_vars[i]);

            // x_i AND NOT y_i: this bit makes x > y regardless of lower bits
            let xi_gt_yi = self.bdd_and(xi, yi.not());

            // x_i XNOR y_i: bits are equal, carry the previous result
            let xi_eq_yi = self.bdd_xnor(xi, yi);
            let carry = self.bdd_and(xi_eq_yi, result);

            result = self.bdd_or(xi_gt_yi, carry);
        }

        result
    }

    /// BDD for `lower <= x <= upper` where x is an unsigned integer
    /// encoded in `x_vars` (MSB first).
    pub fn bdd_interval(
        &mut self,
        x_vars: &[u16],
        lower: u64,
        upper: u64,
    ) -> NodeId {
        if lower > upper {
            return NodeId::ZERO;
        }

        let n = x_vars.len();
        if n == 0 {
            return if lower == 0 { NodeId::ONE } else { NodeId::ZERO };
        }

        // Build BDD for x >= lower AND x <= upper
        let ge_lower = self.bdd_ge_const(x_vars, lower);
        let le_upper = self.bdd_le_const(x_vars, upper);
        self.bdd_and(ge_lower, le_upper)
    }

    /// Helper: BDD for x >= constant value (MSB first encoding).
    fn bdd_ge_const(&mut self, x_vars: &[u16], val: u64) -> NodeId {
        let n = x_vars.len();
        // Build from LSB to MSB
        // ge[i] = true iff x[i..n-1] >= val[i..n-1]
        let mut result = NodeId::ONE; // base: empty suffix is always >=0

        for i in (0..n).rev() {
            let xi = self.bdd_ith_var(x_vars[i]);
            let bit = (val >> (n - 1 - i)) & 1;

            if bit == 1 {
                // Need x[i]=1 AND rest >= rest_val, OR x[i]=1 is needed
                // x[i]=1: need rest >= rest_val (result)
                // x[i]=0: definitely < val at this prefix
                let t_branch = self.bdd_and(xi, result);
                result = t_branch;
            } else {
                // bit == 0
                // x[i]=1: definitely >= val at this prefix -> ONE for rest
                // x[i]=0: need rest >= rest_val (result)
                let not_xi = xi.not();
                let e_branch = self.bdd_and(not_xi, result);
                result = self.bdd_or(xi, e_branch);
            }
        }

        result
    }

    /// Helper: BDD for x <= constant value (MSB first encoding).
    fn bdd_le_const(&mut self, x_vars: &[u16], val: u64) -> NodeId {
        let n = x_vars.len();
        let mut result = NodeId::ONE; // base case

        for i in (0..n).rev() {
            let xi = self.bdd_ith_var(x_vars[i]);
            let bit = (val >> (n - 1 - i)) & 1;

            if bit == 0 {
                // x[i]=0: need rest <= rest_val (result)
                // x[i]=1: definitely > val at this prefix
                let not_xi = xi.not();
                result = self.bdd_and(not_xi, result);
            } else {
                // bit == 1
                // x[i]=0: definitely <= val at this prefix -> ONE
                // x[i]=1: need rest <= rest_val (result)
                let not_xi = xi.not();
                let t_branch = self.bdd_and(xi, result);
                result = self.bdd_or(not_xi, t_branch);
            }
        }

        result
    }

    /// BDD for all assignments within Hamming distance `dist` from the
    /// satisfying assignments of `f`.
    ///
    /// For each satisfying assignment of `f`, the result includes all
    /// assignments that differ in at most `dist` of the variables in `x_vars`.
    pub fn bdd_hamming_distance(
        &mut self,
        f: NodeId,
        x_vars: &[u16],
        dist: u32,
    ) -> NodeId {
        if f.is_zero() {
            return NodeId::ZERO;
        }
        if dist == 0 {
            return f;
        }

        // Build the "Hamming ball" BDD: for each assignment in f, OR in all
        // assignments within distance dist.
        // Strategy: for distance d, we can flip up to d variables.
        // We iterate: ball_0 = f, ball_{k+1} = OR over all vars (ball_k with var flipped)
        let mut ball = f;

        for _ in 0..dist {
            let mut expanded = ball;
            for &v in x_vars {
                // Flip variable v: swap cofactors
                let var_node = self.bdd_ith_var(v);
                let ball_pos = self.bdd_compose(ball, NodeId::ONE, v);
                let ball_neg = self.bdd_compose(ball, NodeId::ZERO, v);
                // Combine: keep original + flipped
                let flipped = self.bdd_ite(var_node, ball_neg, ball_pos);
                expanded = self.bdd_or(expanded, flipped);
            }
            ball = expanded;
        }

        ball
    }

    /// ADD representing the Hamming distance between two variable vectors.
    ///
    /// For each pair `(x_vars[i], y_vars[i])`, adds 1 to the result when
    /// they differ. The result is an ADD with integer values 0..n.
    pub fn add_hamming(
        &mut self,
        x_vars: &[u16],
        y_vars: &[u16],
    ) -> NodeId {
        assert_eq!(
            x_vars.len(),
            y_vars.len(),
            "x_vars and y_vars must have the same length"
        );

        // Start with ADD constant 0
        let mut result = self.add_const(0.0);

        for i in 0..x_vars.len() {
            // XOR of x[i] and y[i] as a BDD
            let xi = self.bdd_ith_var(x_vars[i]);
            let yi = self.bdd_ith_var(y_vars[i]);
            let diff = self.bdd_xor(xi, yi);

            // Convert the XOR BDD to an ADD (0.0 or 1.0)
            let diff_add = self.bdd_to_add(diff);

            // Accumulate
            result = self.add_plus(result, diff_add);
        }

        result
    }

    /// BDD for `x != y` where x and y are n-bit unsigned integers.
    ///
    /// `x_vars[0]` is the MSB of x, `x_vars[n-1]` is the LSB.
    pub fn bdd_disequality(
        &mut self,
        n: u32,
        x_vars: &[u16],
        y_vars: &[u16],
    ) -> NodeId {
        assert!(x_vars.len() >= n as usize);
        assert!(y_vars.len() >= n as usize);

        // x != y iff there exists some bit position where they differ
        // = OR over all i: (x[i] XOR y[i])
        let mut result = NodeId::ZERO;

        for i in 0..n as usize {
            let xi = self.bdd_ith_var(x_vars[i]);
            let yi = self.bdd_ith_var(y_vars[i]);
            let diff = self.bdd_xor(xi, yi);
            result = self.bdd_or(result, diff);
        }

        result
    }
}
