// lumindd — ADD-based matrix operations
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::HashMap;

use crate::manager::Manager;
use crate::node::{NodeId, CONST_INDEX};

/// Local cache key for ADD triangle abstraction (min over sum).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct TriangleCacheKey {
    f: u32,
    cube: u32,
}

impl Manager {
    // ==================================================================
    // ADD Matrix Multiply: A(x,z) * B(z,y) = ∃z. A(x,z) × B(z,y)
    // ==================================================================

    /// Matrix multiplication via ADDs.
    ///
    /// Computes A(x,z) * B(z,y) = ∃z. A(x,z) × B(z,y), where `z_vars`
    /// lists the variable indices encoding the shared (inner) dimension.
    ///
    /// The result is an ADD over the remaining (x,y) variables.
    pub fn add_matrix_multiply(&mut self, a: NodeId, b: NodeId, z_vars: &[u16]) -> NodeId {
        // Step 1: element-wise product A(x,z) × B(z,y)
        let product = self.add_times(a, b);

        // Step 2: existential abstraction (sum) over z variables
        let cube = self.bdd_cube(z_vars);
        self.add_exist_abstract(product, cube)
    }

    /// Alias for `add_matrix_multiply` — the times-plus semiring:
    /// ∃z. A(x,z) × B(z,y), summing over z.
    pub fn add_times_plus(&mut self, a: NodeId, b: NodeId, z_vars: &[u16]) -> NodeId {
        self.add_matrix_multiply(a, b, z_vars)
    }

    // ==================================================================
    // ADD Triangle: min_z( A(x,z) + B(z,y) )
    // ==================================================================

    /// Triangle operation for shortest-path computations.
    ///
    /// Computes A(x,z) △ B(z,y) = min_z( A(x,z) + B(z,y) ).
    /// This uses the min-plus (tropical) semiring instead of the
    /// plus-times semiring used in standard matrix multiplication.
    pub fn add_triangle(&mut self, a: NodeId, b: NodeId, z_vars: &[u16]) -> NodeId {
        // Step 1: element-wise sum A(x,z) + B(z,y)
        let sum = self.add_plus(a, b);

        // Step 2: min-abstraction over z variables
        let cube = self.bdd_cube(z_vars);
        self.add_min_abstract(sum, cube)
    }

    /// ADD min-abstraction: for each variable in `cube`, take the minimum
    /// of the positive and negative cofactors.
    ///
    /// This is identical to `add_univ_abstract` but provided as a separate
    /// entry point for clarity in the triangle operation context.
    fn add_min_abstract(&mut self, f: NodeId, cube: NodeId) -> NodeId {
        let mut cache = HashMap::new();
        self.add_min_abstract_rec(f, cube, &mut cache)
    }

    fn add_min_abstract_rec(
        &mut self,
        f: NodeId,
        cube: NodeId,
        cache: &mut HashMap<TriangleCacheKey, NodeId>,
    ) -> NodeId {
        if self.var_index(f) == CONST_INDEX {
            return f;
        }
        if cube.is_one() {
            return f;
        }

        let key = TriangleCacheKey {
            f: f.raw_index(),
            cube: cube.raw_index(),
        };
        if let Some(&result) = cache.get(&key) {
            return result;
        }

        let f_level = self.level(f);
        let cube_level = self.level(cube);

        let result = if cube_level < f_level {
            let next_cube = self.then_child(cube);
            self.add_min_abstract_rec(f, next_cube, cache)
        } else if cube_level == f_level {
            let top_var = self.var_index(f);
            let (f_t, f_e) = self.add_cofactors(f, top_var);
            let next_cube = self.then_child(cube);
            let t = self.add_min_abstract_rec(f_t, next_cube, cache);
            let e = self.add_min_abstract_rec(f_e, next_cube, cache);
            self.add_min(t, e)
        } else {
            let top_var = self.var_index(f);
            let (f_t, f_e) = self.add_cofactors(f, top_var);
            let t = self.add_min_abstract_rec(f_t, cube, cache);
            let e = self.add_min_abstract_rec(f_e, cube, cache);
            if t == e {
                t
            } else {
                self.add_unique_inter(top_var, t, e)
            }
        };

        cache.insert(key, result);
        result
    }

    // ==================================================================
    // ADD Outer Sum: result[i][j] = a[i] + b[j]
    // ==================================================================

    /// Outer sum of two ADD vectors.
    ///
    /// Given ADD `a` over row variables and ADD `b` over column variables
    /// (which must be disjoint sets of variables), produces an ADD where
    /// result(row, col) = a(row) + b(col).
    ///
    /// Since `a` and `b` depend on disjoint variables, this is simply
    /// the ADD pointwise sum of the two functions (each acts as a constant
    /// with respect to the other's variables).
    pub fn add_outer_sum(&mut self, a: NodeId, b: NodeId) -> NodeId {
        self.add_plus(a, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a 2×2 ADD matrix encoded with row variables and column variables.
    ///
    /// Row variable = `rv`, column variable = `cv`.
    /// Matrix entries: m[0][0], m[0][1], m[1][0], m[1][1]
    /// Encoding: row=0 means rv=0, row=1 means rv=1, col=0 means cv=0, col=1 means cv=1.
    fn build_2x2_matrix(
        mgr: &mut Manager,
        rv: u16,
        cv: u16,
        m00: f64,
        m01: f64,
        m10: f64,
        m11: f64,
    ) -> NodeId {
        let c00 = mgr.add_const(m00);
        let c01 = mgr.add_const(m01);
        let c10 = mgr.add_const(m10);
        let c11 = mgr.add_const(m11);

        // Build column-level nodes first (lower variable)
        // When rv=0: ITE(cv, m01, m00)
        let row0 = if c01 == c00 {
            c00
        } else {
            mgr.add_unique_inter(cv, c01, c00)
        };
        // When rv=1: ITE(cv, m11, m10)
        let row1 = if c11 == c10 {
            c10
        } else {
            mgr.add_unique_inter(cv, c11, c10)
        };

        // Build row-level node
        if row1 == row0 {
            row0
        } else {
            mgr.add_unique_inter(rv, row1, row0)
        }
    }

    #[test]
    fn test_matrix_multiply_identity() {
        // Multiply a 2×2 matrix by the identity matrix.
        // Variables: row=0, shared=1, col=2
        let mut mgr = Manager::new();
        let _x0 = mgr.bdd_new_var(); // row var (index 0)
        let _x1 = mgr.bdd_new_var(); // shared var (index 1)
        let _x2 = mgr.bdd_new_var(); // col var (index 2)

        // A(row=0, z=1) = [[1,2],[3,4]]
        let a = build_2x2_matrix(&mut mgr, 0, 1, 1.0, 2.0, 3.0, 4.0);

        // B(z=1, col=2) = identity = [[1,0],[0,1]]
        let b = build_2x2_matrix(&mut mgr, 1, 2, 1.0, 0.0, 0.0, 1.0);

        // C = A * B, abstracting over z (var 1)
        let c = mgr.add_matrix_multiply(a, b, &[1]);

        // C should be [[1,2],[3,4]] over (row=0, col=2)
        // Evaluate: row=0,col=0 -> 1; row=0,col=1 -> 2; row=1,col=0 -> 3; row=1,col=1 -> 4
        let (c_r1, c_r0) = mgr.add_cofactors(c, 0); // split on row var
        let (c_r0_c1, c_r0_c0) = mgr.add_cofactors(c_r0, 2); // split on col var
        let (c_r1_c1, c_r1_c0) = mgr.add_cofactors(c_r1, 2);

        assert!((mgr.add_value(c_r0_c0).unwrap() - 1.0).abs() < f64::EPSILON, "C[0][0] should be 1");
        assert!((mgr.add_value(c_r0_c1).unwrap() - 2.0).abs() < f64::EPSILON, "C[0][1] should be 2");
        assert!((mgr.add_value(c_r1_c0).unwrap() - 3.0).abs() < f64::EPSILON, "C[1][0] should be 3");
        assert!((mgr.add_value(c_r1_c1).unwrap() - 4.0).abs() < f64::EPSILON, "C[1][1] should be 4");
    }

    #[test]
    fn test_matrix_multiply_2x2() {
        // A = [[1,2],[3,4]], B = [[5,6],[7,8]]
        // C = A*B = [[1*5+2*7, 1*6+2*8], [3*5+4*7, 3*6+4*8]]
        //         = [[19, 22], [43, 50]]
        let mut mgr = Manager::new();
        let _x0 = mgr.bdd_new_var(); // row (index 0)
        let _x1 = mgr.bdd_new_var(); // shared (index 1)
        let _x2 = mgr.bdd_new_var(); // col (index 2)

        let a = build_2x2_matrix(&mut mgr, 0, 1, 1.0, 2.0, 3.0, 4.0);
        let b = build_2x2_matrix(&mut mgr, 1, 2, 5.0, 6.0, 7.0, 8.0);

        let c = mgr.add_matrix_multiply(a, b, &[1]);

        let (c_r1, c_r0) = mgr.add_cofactors(c, 0);
        let (c_r0_c1, c_r0_c0) = mgr.add_cofactors(c_r0, 2);
        let (c_r1_c1, c_r1_c0) = mgr.add_cofactors(c_r1, 2);

        assert!((mgr.add_value(c_r0_c0).unwrap() - 19.0).abs() < f64::EPSILON, "C[0][0]");
        assert!((mgr.add_value(c_r0_c1).unwrap() - 22.0).abs() < f64::EPSILON, "C[0][1]");
        assert!((mgr.add_value(c_r1_c0).unwrap() - 43.0).abs() < f64::EPSILON, "C[1][0]");
        assert!((mgr.add_value(c_r1_c1).unwrap() - 50.0).abs() < f64::EPSILON, "C[1][1]");
    }

    #[test]
    fn test_triangle_shortest_path() {
        // Triangle (min-plus): min_z( A(x,z) + B(z,y) )
        // A = [[0, 3], [7, 1]], B = [[0, 2], [5, 0]]
        // C[0][0] = min(0+0, 3+5) = 0
        // C[0][1] = min(0+2, 3+0) = 2
        // C[1][0] = min(7+0, 1+5) = 6
        // C[1][1] = min(7+2, 1+0) = 1
        let mut mgr = Manager::new();
        let _x0 = mgr.bdd_new_var();
        let _x1 = mgr.bdd_new_var();
        let _x2 = mgr.bdd_new_var();

        let a = build_2x2_matrix(&mut mgr, 0, 1, 0.0, 3.0, 7.0, 1.0);
        let b = build_2x2_matrix(&mut mgr, 1, 2, 0.0, 2.0, 5.0, 0.0);

        let c = mgr.add_triangle(a, b, &[1]);

        let (c_r1, c_r0) = mgr.add_cofactors(c, 0);
        let (c_r0_c1, c_r0_c0) = mgr.add_cofactors(c_r0, 2);
        let (c_r1_c1, c_r1_c0) = mgr.add_cofactors(c_r1, 2);

        assert!((mgr.add_value(c_r0_c0).unwrap() - 0.0).abs() < f64::EPSILON, "C[0][0]");
        assert!((mgr.add_value(c_r0_c1).unwrap() - 2.0).abs() < f64::EPSILON, "C[0][1]");
        assert!((mgr.add_value(c_r1_c0).unwrap() - 6.0).abs() < f64::EPSILON, "C[1][0]");
        assert!((mgr.add_value(c_r1_c1).unwrap() - 1.0).abs() < f64::EPSILON, "C[1][1]");
    }

    #[test]
    fn test_outer_sum() {
        // a depends on var 0: a(0)=1, a(1)=3
        // b depends on var 1: b(0)=10, b(1)=20
        // outer_sum[i][j] = a[i] + b[j]
        // result: [[11,21],[13,23]]
        let mut mgr = Manager::new();
        let _x0 = mgr.bdd_new_var();
        let _x1 = mgr.bdd_new_var();

        let c1 = mgr.add_const(1.0);
        let c3 = mgr.add_const(3.0);
        let a = mgr.add_unique_inter(0, c3, c1); // var0: then=3, else=1

        let c10 = mgr.add_const(10.0);
        let c20 = mgr.add_const(20.0);
        let b = mgr.add_unique_inter(1, c20, c10); // var1: then=20, else=10

        let result = mgr.add_outer_sum(a, b);

        // Check all four entries
        let (r1, r0) = mgr.add_cofactors(result, 0);
        let (r0_c1, r0_c0) = mgr.add_cofactors(r0, 1);
        let (r1_c1, r1_c0) = mgr.add_cofactors(r1, 1);

        assert!((mgr.add_value(r0_c0).unwrap() - 11.0).abs() < f64::EPSILON);
        assert!((mgr.add_value(r0_c1).unwrap() - 21.0).abs() < f64::EPSILON);
        assert!((mgr.add_value(r1_c0).unwrap() - 13.0).abs() < f64::EPSILON);
        assert!((mgr.add_value(r1_c1).unwrap() - 23.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_times_plus_alias() {
        let mut mgr = Manager::new();
        let _x0 = mgr.bdd_new_var();
        let _x1 = mgr.bdd_new_var();
        let _x2 = mgr.bdd_new_var();

        let a = build_2x2_matrix(&mut mgr, 0, 1, 1.0, 0.0, 0.0, 1.0);
        let b = build_2x2_matrix(&mut mgr, 1, 2, 2.0, 3.0, 4.0, 5.0);

        let c1 = mgr.add_matrix_multiply(a, b, &[1]);
        let c2 = mgr.add_times_plus(a, b, &[1]);

        assert_eq!(c1, c2, "times_plus should be an alias for matrix_multiply");
    }
}
