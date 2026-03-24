// lumindd — Comprehensive edge case and coverage tests
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use lumindd::*;

// =====================================================================
// Helper: evaluate all 2^n assignments and compare two BDDs
// =====================================================================

fn bdds_equal(mgr: &Manager, f: NodeId, g: NodeId, n: usize) -> bool {
    let total = 1u64 << n;
    let mut assignment = vec![false; n];
    for row in 0..total {
        for j in 0..n {
            assignment[j] = (row >> (n - 1 - j)) & 1 == 1;
        }
        if mgr.bdd_eval(f, &assignment) != mgr.bdd_eval(g, &assignment) {
            return false;
        }
    }
    true
}

fn bdd_implies(mgr: &Manager, f: NodeId, g: NodeId, n: usize) -> bool {
    let total = 1u64 << n;
    let mut assignment = vec![false; n];
    for row in 0..total {
        for j in 0..n {
            assignment[j] = (row >> (n - 1 - j)) & 1 == 1;
        }
        if mgr.bdd_eval(f, &assignment) && !mgr.bdd_eval(g, &assignment) {
            return false;
        }
    }
    true
}

// =====================================================================
// BDD OR edge cases
// =====================================================================

#[test]
fn bdd_or_with_one() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let result = mgr.bdd_or(x, NodeId::ONE);
    assert!(mgr.bdd_is_tautology(result));
}

#[test]
fn bdd_or_with_zero() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let result = mgr.bdd_or(x, NodeId::ZERO);
    assert_eq!(result, x);
}

#[test]
fn bdd_or_same_argument() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let f = mgr.bdd_and(x, y);
    let result = mgr.bdd_or(f, f);
    assert_eq!(result, f);
}

// =====================================================================
// BDD XOR edge cases
// =====================================================================

#[test]
fn bdd_xor_same_argument_is_zero() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let result = mgr.bdd_xor(x, x);
    assert!(mgr.bdd_is_unsat(result));
}

#[test]
fn bdd_xor_with_zero() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let result = mgr.bdd_xor(x, NodeId::ZERO);
    assert_eq!(result, x);
}

#[test]
fn bdd_xor_with_one() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let result = mgr.bdd_xor(x, NodeId::ONE);
    let not_x = mgr.bdd_not(x);
    assert_eq!(result, not_x);
}

#[test]
fn bdd_xor_complement_is_one() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let not_x = mgr.bdd_not(x);
    let result = mgr.bdd_xor(x, not_x);
    assert!(mgr.bdd_is_tautology(result));
}

// =====================================================================
// BDD NAND / NOR / XNOR edge cases
// =====================================================================

#[test]
fn bdd_nand_with_constants() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    // NAND(x, ONE) = NOT(x AND ONE) = NOT(x)
    let r1 = mgr.bdd_nand(x, NodeId::ONE);
    assert_eq!(r1, mgr.bdd_not(x));
    // NAND(x, ZERO) = NOT(ZERO) = ONE
    let r2 = mgr.bdd_nand(x, NodeId::ZERO);
    assert!(mgr.bdd_is_tautology(r2));
}

#[test]
fn bdd_nand_same_argument() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    // NAND(x, x) = NOT(x)
    let result = mgr.bdd_nand(x, x);
    assert_eq!(result, mgr.bdd_not(x));
}

#[test]
fn bdd_nor_with_constants() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    // NOR(x, ZERO) = NOT(x OR ZERO) = NOT(x)
    let r1 = mgr.bdd_nor(x, NodeId::ZERO);
    assert_eq!(r1, mgr.bdd_not(x));
    // NOR(x, ONE) = NOT(ONE) = ZERO
    let r2 = mgr.bdd_nor(x, NodeId::ONE);
    assert!(mgr.bdd_is_unsat(r2));
}

#[test]
fn bdd_nor_same_argument() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    // NOR(x, x) = NOT(x OR x) = NOT(x)
    let result = mgr.bdd_nor(x, x);
    assert_eq!(result, mgr.bdd_not(x));
}

#[test]
fn bdd_xnor_with_constants() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    // XNOR(x, ONE) = NOT(x XOR ONE) = NOT(NOT(x)) = x
    let r1 = mgr.bdd_xnor(x, NodeId::ONE);
    assert_eq!(r1, x);
    // XNOR(x, ZERO) = NOT(x XOR ZERO) = NOT(x)
    let r2 = mgr.bdd_xnor(x, NodeId::ZERO);
    assert_eq!(r2, mgr.bdd_not(x));
}

#[test]
fn bdd_xnor_same_argument() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    // XNOR(x, x) = NOT(ZERO) = ONE
    let result = mgr.bdd_xnor(x, x);
    assert!(mgr.bdd_is_tautology(result));
}

// =====================================================================
// BDD ITE terminal cases
// =====================================================================

#[test]
fn bdd_ite_one_y_z() {
    let mut mgr = Manager::new();
    let y = mgr.bdd_new_var();
    let z = mgr.bdd_new_var();
    let result = mgr.bdd_ite(NodeId::ONE, y, z);
    assert_eq!(result, y);
}

#[test]
fn bdd_ite_zero_y_z() {
    let mut mgr = Manager::new();
    let y = mgr.bdd_new_var();
    let z = mgr.bdd_new_var();
    let result = mgr.bdd_ite(NodeId::ZERO, y, z);
    assert_eq!(result, z);
}

#[test]
fn bdd_ite_x_f_f() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let f = mgr.bdd_and(x, y);
    let result = mgr.bdd_ite(x, f, f);
    assert_eq!(result, f);
}

#[test]
fn bdd_ite_f_one_zero() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let result = mgr.bdd_ite(x, NodeId::ONE, NodeId::ZERO);
    assert_eq!(result, x);
}

#[test]
fn bdd_ite_f_zero_one() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let result = mgr.bdd_ite(x, NodeId::ZERO, NodeId::ONE);
    assert_eq!(result, mgr.bdd_not(x));
}

// =====================================================================
// BDD Compose with constant substitution
// =====================================================================

#[test]
fn bdd_compose_with_one() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // var 0
    let y = mgr.bdd_new_var(); // var 1
    let f = mgr.bdd_and(x, y);
    // f[x := ONE] = ONE AND y = y
    let result = mgr.bdd_compose(f, NodeId::ONE, 0);
    assert_eq!(result, y);
}

#[test]
fn bdd_compose_with_zero() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let f = mgr.bdd_or(x, y);
    // f[x := ZERO] = ZERO OR y = y
    let result = mgr.bdd_compose(f, NodeId::ZERO, 0);
    assert_eq!(result, y);
}

#[test]
fn bdd_compose_constant_function() {
    let mut mgr = Manager::new();
    let _x = mgr.bdd_new_var();
    // Compose on constant function returns the constant
    let result = mgr.bdd_compose(NodeId::ONE, NodeId::ZERO, 0);
    assert!(mgr.bdd_is_tautology(result));
}

// =====================================================================
// BDD Restrict and Constrain edge cases
// =====================================================================

#[test]
fn bdd_restrict_with_one() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let f = mgr.bdd_and(x, y);
    let result = mgr.bdd_restrict(f, NodeId::ONE);
    assert_eq!(result, f);
}

#[test]
fn bdd_restrict_with_zero() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let f = mgr.bdd_and(x, y);
    let result = mgr.bdd_restrict(f, NodeId::ZERO);
    assert!(mgr.bdd_is_unsat(result));
}

#[test]
fn bdd_restrict_constant_function() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let result = mgr.bdd_restrict(NodeId::ONE, x);
    assert!(mgr.bdd_is_tautology(result));
}

#[test]
fn bdd_constrain_with_one() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let f = mgr.bdd_and(x, y);
    let result = mgr.bdd_constrain(f, NodeId::ONE);
    assert_eq!(result, f);
}

#[test]
fn bdd_constrain_with_zero() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let f = mgr.bdd_and(x, y);
    let result = mgr.bdd_constrain(f, NodeId::ZERO);
    assert!(mgr.bdd_is_unsat(result));
}

#[test]
fn bdd_constrain_f_equals_c() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let result = mgr.bdd_constrain(x, x);
    assert!(mgr.bdd_is_tautology(result));
}

#[test]
fn bdd_constrain_f_equals_not_c() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let not_x = mgr.bdd_not(x);
    let result = mgr.bdd_constrain(x, not_x);
    assert!(mgr.bdd_is_unsat(result));
}

// =====================================================================
// BDD exist_abstract edge cases
// =====================================================================

#[test]
fn bdd_exist_abstract_empty_cube() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let f = mgr.bdd_and(x, y);
    // Existential abstraction with cube=ONE (empty cube) returns f
    let result = mgr.bdd_exist_abstract(f, NodeId::ONE);
    assert_eq!(result, f);
}

#[test]
fn bdd_exist_abstract_single_var() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // 0
    let y = mgr.bdd_new_var(); // 1
    let f = mgr.bdd_and(x, y);
    let cube = mgr.bdd_cube(&[0]);
    // exists x. (x AND y) = y
    let result = mgr.bdd_exist_abstract(f, cube);
    assert_eq!(result, y);
}

#[test]
fn bdd_exist_abstract_multi_var_cube() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // 0
    let y = mgr.bdd_new_var(); // 1
    let z = mgr.bdd_new_var(); // 2
    let yz = mgr.bdd_and(y, z);
    let f = mgr.bdd_and(x, yz);
    let cube = mgr.bdd_cube(&[0, 1]);
    // exists x,y. (x AND y AND z) = z
    let result = mgr.bdd_exist_abstract(f, cube);
    assert_eq!(result, z);
}

#[test]
fn bdd_exist_abstract_all_vars() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let f = mgr.bdd_and(x, y);
    let cube = mgr.bdd_cube(&[0, 1]);
    // exists x,y. (x AND y) = TRUE (f is satisfiable)
    let result = mgr.bdd_exist_abstract(f, cube);
    assert!(mgr.bdd_is_tautology(result));
}

#[test]
fn bdd_exist_abstract_constant() {
    let mut mgr = Manager::new();
    let _x = mgr.bdd_new_var();
    let cube = mgr.bdd_cube(&[0]);
    let result = mgr.bdd_exist_abstract(NodeId::ZERO, cube);
    assert!(mgr.bdd_is_unsat(result));
}

// =====================================================================
// BDD Approximation (bdd_approx.rs)
// =====================================================================

#[test]
fn bdd_under_approx_constant_inputs() {
    let mut mgr = Manager::new();
    assert_eq!(mgr.bdd_under_approx(NodeId::ONE, 0, 10), NodeId::ONE);
    assert_eq!(mgr.bdd_under_approx(NodeId::ZERO, 0, 10), NodeId::ZERO);
}

#[test]
fn bdd_over_approx_constant_inputs() {
    let mut mgr = Manager::new();
    assert_eq!(mgr.bdd_over_approx(NodeId::ONE, 0, 10), NodeId::ONE);
    assert_eq!(mgr.bdd_over_approx(NodeId::ZERO, 0, 10), NodeId::ZERO);
}

#[test]
fn bdd_under_approx_implies_f() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // 0
    let y = mgr.bdd_new_var(); // 1
    let z = mgr.bdd_new_var(); // 2
    let w = mgr.bdd_new_var(); // 3
    // Build a function with several nodes
    let f1 = mgr.bdd_and(x, y);
    let f2 = mgr.bdd_and(z, w);
    let f = mgr.bdd_or(f1, f2);

    let under = mgr.bdd_under_approx(f, 4, 2);
    // under_approx should imply f
    assert!(bdd_implies(&mgr, under, f, 4));
}

#[test]
fn bdd_over_approx_implies_by_f() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let z = mgr.bdd_new_var();
    let w = mgr.bdd_new_var();
    let f1 = mgr.bdd_and(x, y);
    let f2 = mgr.bdd_and(z, w);
    let f = mgr.bdd_or(f1, f2);

    let over = mgr.bdd_over_approx(f, 4, 2);
    // f should imply over_approx
    assert!(bdd_implies(&mgr, f, over, 4));
}

#[test]
fn bdd_subset_heavy_branch_3vars() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let z = mgr.bdd_new_var();
    let xy = mgr.bdd_and(x, y);
    let f = mgr.bdd_or(xy, z);
    let subset = mgr.bdd_subset_heavy_branch(f, 3, 3);
    // subset must imply f
    assert!(bdd_implies(&mgr, subset, f, 3));
}

#[test]
fn bdd_superset_heavy_branch_3vars() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let z = mgr.bdd_new_var();
    let xy = mgr.bdd_and(x, y);
    let f = mgr.bdd_or(xy, z);
    let superset = mgr.bdd_superset_heavy_branch(f, 3, 3);
    // f must imply superset
    assert!(bdd_implies(&mgr, f, superset, 3));
}

#[test]
fn bdd_squeeze_between_bounds() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let _z = mgr.bdd_new_var();
    // lb = x AND y, ub = x
    let lb = mgr.bdd_and(x, y);
    let ub = x;
    let squeezed = mgr.bdd_squeeze(lb, ub);
    // lb <= squeezed <= ub
    assert!(bdd_implies(&mgr, lb, squeezed, 3));
    assert!(bdd_implies(&mgr, squeezed, ub, 3));
}

#[test]
fn bdd_squeeze_equal_bounds() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let result = mgr.bdd_squeeze(x, x);
    assert_eq!(result, x);
}

#[test]
fn bdd_squeeze_lb_zero() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let result = mgr.bdd_squeeze(NodeId::ZERO, x);
    assert_eq!(result, NodeId::ZERO);
}

#[test]
fn bdd_squeeze_ub_one() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let result = mgr.bdd_squeeze(x, NodeId::ONE);
    assert!(mgr.bdd_is_tautology(result));
}

// =====================================================================
// BDD Priority (bdd_priority.rs)
// =====================================================================

#[test]
fn bdd_inequality_2bit() {
    let mut mgr = Manager::new();
    // x: vars 0,1 (MSB first), y: vars 2,3
    for _ in 0..4 { mgr.bdd_new_var(); }
    let x_vars = [0u16, 1];
    let y_vars = [2u16, 3];
    let gt = mgr.bdd_inequality(2, &x_vars, &y_vars);

    // Check all 16 combinations
    for xv in 0..4u32 {
        for yv in 0..4u32 {
            let assignment = vec![
                xv & 2 != 0, xv & 1 != 0,
                yv & 2 != 0, yv & 1 != 0,
            ];
            let result = mgr.bdd_eval(gt, &assignment);
            assert_eq!(result, xv > yv, "x={} > y={} failed", xv, yv);
        }
    }
}

#[test]
fn bdd_inequality_3bit() {
    let mut mgr = Manager::new();
    for _ in 0..6 { mgr.bdd_new_var(); }
    let x_vars = [0u16, 1, 2];
    let y_vars = [3u16, 4, 5];
    let gt = mgr.bdd_inequality(3, &x_vars, &y_vars);

    for xv in 0..8u32 {
        for yv in 0..8u32 {
            let assignment = vec![
                xv & 4 != 0, xv & 2 != 0, xv & 1 != 0,
                yv & 4 != 0, yv & 2 != 0, yv & 1 != 0,
            ];
            let result = mgr.bdd_eval(gt, &assignment);
            assert_eq!(result, xv > yv, "x={} > y={} failed", xv, yv);
        }
    }
}

#[test]
fn bdd_interval_range() {
    let mut mgr = Manager::new();
    for _ in 0..4 { mgr.bdd_new_var(); }
    let x_vars = [0u16, 1, 2, 3];
    let interval = mgr.bdd_interval(&x_vars, 3, 10);

    for xv in 0..16u64 {
        let assignment = vec![
            xv & 8 != 0, xv & 4 != 0, xv & 2 != 0, xv & 1 != 0,
        ];
        let result = mgr.bdd_eval(interval, &assignment);
        assert_eq!(result, xv >= 3 && xv <= 10, "x={} in [3,10] failed", xv);
    }
}

#[test]
fn bdd_interval_empty() {
    let mut mgr = Manager::new();
    for _ in 0..4 { mgr.bdd_new_var(); }
    let x_vars = [0u16, 1, 2, 3];
    let interval = mgr.bdd_interval(&x_vars, 10, 3); // lower > upper
    assert!(mgr.bdd_is_unsat(interval));
}

#[test]
fn bdd_disequality_2bit() {
    let mut mgr = Manager::new();
    for _ in 0..4 { mgr.bdd_new_var(); }
    let x_vars = [0u16, 1];
    let y_vars = [2u16, 3];
    let neq = mgr.bdd_disequality(2, &x_vars, &y_vars);

    for xv in 0..4u32 {
        for yv in 0..4u32 {
            let assignment = vec![
                xv & 2 != 0, xv & 1 != 0,
                yv & 2 != 0, yv & 1 != 0,
            ];
            let result = mgr.bdd_eval(neq, &assignment);
            assert_eq!(result, xv != yv, "x={} != y={} failed", xv, yv);
        }
    }
}

// =====================================================================
// BDD Decomposition (bdd_decomp.rs)
// =====================================================================

#[test]
fn bdd_conjunctive_decomp_and_is_f() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let z = mgr.bdd_new_var();
    // f = x AND (y OR z) — should be decomposable
    let yz = mgr.bdd_or(y, z);
    let f = mgr.bdd_and(x, yz);
    let (g, h) = mgr.bdd_conjunctive_decomp(f);
    let product = mgr.bdd_and(g, h);
    assert!(bdds_equal(&mgr, product, f, 3), "g AND h must equal f");
}

#[test]
fn bdd_conjunctive_decomp_constant() {
    let mut mgr = Manager::new();
    let (g, h) = mgr.bdd_conjunctive_decomp(NodeId::ONE);
    assert!(mgr.bdd_is_tautology(g));
    assert!(mgr.bdd_is_tautology(h));
}

#[test]
fn bdd_disjunctive_decomp_or_is_f() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let z = mgr.bdd_new_var();
    // f = x OR (y AND z) — should be decomposable
    let yz = mgr.bdd_and(y, z);
    let f = mgr.bdd_or(x, yz);
    let (g, h) = mgr.bdd_disjunctive_decomp(f);
    let sum = mgr.bdd_or(g, h);
    assert!(bdds_equal(&mgr, sum, f, 3), "g OR h must equal f");
}

#[test]
fn bdd_solve_eqn_verify() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // 0
    let y = mgr.bdd_new_var(); // 1
    // f = x XOR y (solve for x: we want f=0, i.e., x XNOR y)
    let f = mgr.bdd_xor(x, y);
    let (particular, care) = mgr.bdd_solve_eqn(f, 0);
    // Substituting particular for var 0 should make f zero on the care set
    let substituted = mgr.bdd_compose(f, particular, 0);
    let check = mgr.bdd_and(substituted, care);
    assert!(mgr.bdd_is_unsat(check), "solution should satisfy f=0 on care set");
}

#[test]
fn bdd_essential_vars_single_var() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let essential = mgr.bdd_essential_vars(x);
    assert_eq!(essential, vec![0]);
}

#[test]
fn bdd_essential_vars_and() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let _y = mgr.bdd_new_var();
    let f = mgr.bdd_and(x, _y);
    let essential = mgr.bdd_essential_vars(f);
    // Top variable (x) is essential — its cofactors differ from f
    assert!(essential.contains(&0));
    // Both vars are in support
    let support = mgr.bdd_support(f);
    assert_eq!(support.len(), 2);
}

#[test]
fn bdd_essential_vars_constant() {
    let mgr = Manager::new();
    let essential = mgr.bdd_essential_vars(NodeId::ONE);
    assert!(essential.is_empty());
}

// =====================================================================
// Advanced Composition (compose_adv.rs)
// =====================================================================

#[test]
fn bdd_vector_compose_identity() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // 0
    let y = mgr.bdd_new_var(); // 1
    let f = mgr.bdd_and(x, y);
    // Identity substitution: var 0 -> var 0, var 1 -> var 1
    let v0 = mgr.bdd_ith_var(0);
    let v1 = mgr.bdd_ith_var(1);
    let result = mgr.bdd_vector_compose(f, &[v0, v1]);
    assert_eq!(result, f);
}

#[test]
fn bdd_vector_compose_swap() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // 0
    let y = mgr.bdd_new_var(); // 1
    let f = mgr.bdd_and(x, mgr.bdd_not(y)); // x AND NOT y
    // Swap: var 0 -> var 1, var 1 -> var 0
    let v0 = mgr.bdd_ith_var(0);
    let v1 = mgr.bdd_ith_var(1);
    let result = mgr.bdd_vector_compose(f, &[v1, v0]);
    // Result should be y AND NOT x
    let expected = mgr.bdd_and(y, mgr.bdd_not(x));
    assert!(bdds_equal(&mgr, result, expected, 2));
}

#[test]
fn bdd_vector_compose_constant_substitution() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let f = mgr.bdd_or(x, y);
    // Replace var 0 with ONE, var 1 with var 1
    let v1 = mgr.bdd_ith_var(1);
    let result = mgr.bdd_vector_compose(f, &[NodeId::ONE, v1]);
    assert!(mgr.bdd_is_tautology(result));
}

#[test]
fn bdd_permute_swap() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // 0
    let y = mgr.bdd_new_var(); // 1
    let _z = mgr.bdd_new_var(); // 2
    let f = mgr.bdd_and(x, y);
    // Permutation: 0->1, 1->0, 2->2
    let result = mgr.bdd_permute(f, &[1, 0, 2]);
    // f(x0, x1) -> f(x1, x0) = x1 AND x0 = same function
    assert!(bdds_equal(&mgr, result, f, 3));
}

#[test]
fn bdd_permute_shift() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // 0
    let _y = mgr.bdd_new_var(); // 1
    let _z = mgr.bdd_new_var(); // 2
    // f = x0
    // Permute: 0->2, 1->1, 2->0
    let result = mgr.bdd_permute(x, &[2, 1, 0]);
    let z = mgr.bdd_ith_var(2);
    assert!(bdds_equal(&mgr, result, z, 3));
}

#[test]
fn bdd_swap_variables_basic() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // 0
    let y = mgr.bdd_new_var(); // 1
    let f = mgr.bdd_and(x, mgr.bdd_not(y)); // x AND NOT y

    let result = mgr.bdd_swap_variables(f, &[0], &[1]);
    // After swapping x and y: y AND NOT x
    let expected = mgr.bdd_and(y, mgr.bdd_not(x));
    assert!(bdds_equal(&mgr, result, expected, 2));
}

#[test]
fn bdd_swap_variables_constant() {
    let mut mgr = Manager::new();
    let _x = mgr.bdd_new_var();
    let _y = mgr.bdd_new_var();
    let result = mgr.bdd_swap_variables(NodeId::ONE, &[0], &[1]);
    assert!(mgr.bdd_is_tautology(result));
}

// =====================================================================
// Export (export.rs)
// =====================================================================

#[test]
fn dump_blif_constant_one() {
    let mut mgr = Manager::new();
    let _x = mgr.bdd_new_var();
    let mut buf = Vec::new();
    mgr.dump_blif(NodeId::ONE, None, "out", &mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains(".model bdd"));
    assert!(output.contains(".outputs out"));
    assert!(output.contains(".end"));
}

#[test]
fn dump_blif_variable() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let f = mgr.bdd_and(x, y);
    let mut buf = Vec::new();
    mgr.dump_blif(f, Some(&["a", "b"]), "out", &mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains(".inputs"));
    assert!(output.contains(".outputs out"));
    assert!(output.contains(".end"));
}

#[test]
fn dump_factored_form_constants() {
    let mgr = Manager::new();
    assert_eq!(mgr.dump_factored_form(NodeId::ONE), "1");
    assert_eq!(mgr.dump_factored_form(NodeId::ZERO), "0");
}

#[test]
fn dump_factored_form_variable() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let form = mgr.dump_factored_form(x);
    assert_eq!(form, "x0");
}

#[test]
fn dump_factored_form_and() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let f = mgr.bdd_and(x, y);
    let form = mgr.dump_factored_form(f);
    // Should contain both variables
    assert!(form.contains("x0"));
    assert!(form.contains("x1"));
}

#[test]
fn dump_truth_table_and() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let f = mgr.bdd_and(x, y);
    let mut buf = Vec::new();
    mgr.dump_truth_table(f, &mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();
    // AND truth table: only 1,1 -> 1
    let lines: Vec<&str> = output.lines().collect();
    // Header + separator + 4 rows = 6 lines
    assert!(lines.len() >= 6);
    // Last row should be "1 1 | 1"
    let last_row = lines.last().unwrap();
    assert!(last_row.contains("| 1"), "last row: {}", last_row);
}

#[test]
fn dump_dot_color_basic() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let f = mgr.bdd_and(x, y);
    let mut buf = Vec::new();
    mgr.dump_dot_color(f, &[], &mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("digraph BDD"));
    assert!(output.contains("ONE"));
    assert!(output.contains("ZERO"));
}

#[test]
fn dump_dot_color_with_highlight() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let f = mgr.bdd_and(x, y);
    let mut buf = Vec::new();
    mgr.dump_dot_color(f, &[x], &mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();
    // Highlighted node should have gold color
    assert!(output.contains("#FFD700") || output.contains("#ADD8E6"));
}

#[test]
fn dump_davinci_constant() {
    let mgr = Manager::new();
    let mut buf = Vec::new();
    mgr.dump_davinci(NodeId::ONE, &mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("["));
    assert!(output.contains("]"));
    assert!(output.contains("\"1\""));
}

#[test]
fn dump_davinci_function() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let f = mgr.bdd_or(x, y);
    let mut buf = Vec::new();
    mgr.dump_davinci(f, &mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("ONE"));
    assert!(output.contains("ZERO"));
    assert!(output.contains("then"));
    assert!(output.contains("else"));
}

// =====================================================================
// ZDD Advanced (zdd_advanced.rs)
// =====================================================================

#[test]
fn zdd_isop_between_bounds() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    // lower = x AND y, upper = x OR y
    let lower = mgr.bdd_and(x, y);
    let upper = mgr.bdd_or(x, y);
    let (_zdd_cover, bdd_func) = mgr.zdd_isop(lower, upper);
    // bdd_func must be between lower and upper
    assert!(bdd_implies(&mgr, lower, bdd_func, 2));
    assert!(bdd_implies(&mgr, bdd_func, upper, 2));
}

#[test]
fn zdd_isop_equal_bounds() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let (_, bdd_func) = mgr.zdd_isop(x, x);
    assert!(bdds_equal(&mgr, bdd_func, x, 1));
}

#[test]
fn zdd_isop_constant_bounds() {
    let mut mgr = Manager::new();
    let (_zdd, bdd) = mgr.zdd_isop(NodeId::ZERO, NodeId::ONE);
    // lower=0 => zdd_cover should be empty or the result should be between 0 and 1
    assert!(bdd_implies(&mgr, NodeId::ZERO, bdd, 0));
}

#[test]
fn zdd_support_basic() {
    let mut mgr = Manager::new();
    let _x = mgr.bdd_new_var();
    let _y = mgr.bdd_new_var();
    mgr.zdd_new_var();
    mgr.zdd_new_var();
    // Build ZDD for {{0}, {1}} using zdd_change
    let s0 = mgr.zdd_change(NodeId::ONE, 0); // {emptyset} -> {{0}}
    let s1 = mgr.zdd_change(NodeId::ONE, 1); // {emptyset} -> {{1}}
    let family = mgr.zdd_union(s0, s1);
    let support = mgr.zdd_support(family);
    assert!(support.contains(&0));
    assert!(support.contains(&1));
}

#[test]
fn zdd_dag_size_constant() {
    let mgr = Manager::new();
    assert_eq!(mgr.zdd_dag_size(NodeId::ZERO), 1);
    assert_eq!(mgr.zdd_dag_size(NodeId::ONE), 1);
}

#[test]
fn zdd_dag_size_single_set() {
    let mut mgr = Manager::new();
    mgr.bdd_new_var();
    mgr.zdd_new_var();
    let s = mgr.zdd_change(NodeId::ONE, 0); // {{0}}
    // s = {{0}}: one internal node + ONE terminal = 2
    let size = mgr.zdd_dag_size(s);
    assert!(size >= 2);
}

#[test]
fn zdd_max_cardinality_basic() {
    let mut mgr = Manager::new();
    mgr.bdd_new_var();
    mgr.bdd_new_var();
    mgr.zdd_new_var();
    mgr.zdd_new_var();
    // {{0, 1}, {0}} — max cardinality is 2
    // Build {{0, 1}} by starting from {emptyset}, toggling 0, then toggling 1
    let s0 = mgr.zdd_change(NodeId::ONE, 0); // {{0}}
    let s01 = mgr.zdd_change(s0, 1); // {{0, 1}}
    let family = mgr.zdd_union(s01, s0);
    assert_eq!(mgr.zdd_max_cardinality(family), 2);
}

#[test]
fn zdd_min_cardinality_basic() {
    let mut mgr = Manager::new();
    mgr.bdd_new_var();
    mgr.bdd_new_var();
    mgr.zdd_new_var();
    mgr.zdd_new_var();
    // {{0, 1}, {0}} — min cardinality is 1
    let s0 = mgr.zdd_change(NodeId::ONE, 0); // {{0}}
    let s01 = mgr.zdd_change(s0, 1); // {{0, 1}}
    let family = mgr.zdd_union(s01, s0);
    assert_eq!(mgr.zdd_min_cardinality(family), 1);
}

#[test]
fn zdd_min_cardinality_empty() {
    let mgr = Manager::new();
    assert_eq!(mgr.zdd_min_cardinality(NodeId::ZERO), u32::MAX);
}

#[test]
fn zdd_min_cardinality_empty_set() {
    let mgr = Manager::new();
    // {emptyset} — ONE represents the family containing the empty set
    assert_eq!(mgr.zdd_min_cardinality(NodeId::ONE), 0);
}

#[test]
fn zdd_count_minterm_basic() {
    let mut mgr = Manager::new();
    mgr.bdd_new_var();
    mgr.zdd_new_var();
    // ZDD = {{0}}: one cube with one literal covering 2^(1-1) = 1 minterm for 1 var
    let s = mgr.zdd_change(NodeId::ONE, 0); // {{0}}
    let count = mgr.zdd_count_minterm(s, 1);
    assert!((count - 1.0).abs() < 1e-9);
}

#[test]
fn zdd_count_minterm_empty_cube() {
    let mgr = Manager::new();
    // ZDD = ONE = {emptyset}: the empty cube covers 2^num_vars minterms
    let count = mgr.zdd_count_minterm(NodeId::ONE, 3);
    assert!((count - 8.0).abs() < 1e-9);
}

// =====================================================================
// ZDD Reorder (zdd_reorder.rs)
// =====================================================================

#[test]
fn zdd_shuffle_heap_preserves_count() {
    let mut mgr = Manager::new();
    mgr.bdd_new_var();
    mgr.bdd_new_var();
    mgr.bdd_new_var();
    mgr.zdd_new_var();
    mgr.zdd_new_var();
    mgr.zdd_new_var();
    // Build {{0}, {1}, {2}}
    let s0 = mgr.zdd_change(NodeId::ONE, 0);
    let s1 = mgr.zdd_change(NodeId::ONE, 1);
    let s2 = mgr.zdd_change(NodeId::ONE, 2);
    let s01 = mgr.zdd_union(s0, s1);
    let family = mgr.zdd_union(s01, s2);
    let count_before = mgr.zdd_count(family);

    // Reverse the permutation
    mgr.zdd_shuffle_heap(&[2, 1, 0]);
    // Count should be preserved (the family semantics don't change)
    // Note: after reordering, the old NodeId may not directly correspond,
    // but zdd_count on the same NodeId should still work if nodes are intact.
    // At minimum, this should not crash.
    let count_after = mgr.zdd_count(family);
    assert_eq!(count_before, count_after, "ZDD family count should be preserved after shuffle");
}

#[test]
fn zdd_reduce_heap_no_crash() {
    let mut mgr = Manager::new();
    mgr.bdd_new_var();
    mgr.bdd_new_var();
    mgr.zdd_new_var();
    mgr.zdd_new_var();
    let s0 = mgr.zdd_change(NodeId::ONE, 0);
    let s1 = mgr.zdd_change(NodeId::ONE, 1);
    let _family = mgr.zdd_union(s0, s1);
    // Should not crash
    mgr.zdd_reduce_heap(ReorderingMethod::Window2);
}

#[test]
fn zdd_reduce_heap_sift_no_crash() {
    let mut mgr = Manager::new();
    mgr.bdd_new_var();
    mgr.bdd_new_var();
    mgr.zdd_new_var();
    mgr.zdd_new_var();
    let s0 = mgr.zdd_change(NodeId::ONE, 0);
    let s1 = mgr.zdd_change(NodeId::ONE, 1);
    let _family = mgr.zdd_union(s0, s1);
    mgr.zdd_reduce_heap(ReorderingMethod::Sift);
}

// =====================================================================
// Integration / Cross-module tests
// =====================================================================

#[test]
fn approximation_ordering_under_le_f_le_over() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let z = mgr.bdd_new_var();
    let w = mgr.bdd_new_var();
    let xy = mgr.bdd_and(x, y);
    let zw = mgr.bdd_and(z, w);
    let f = mgr.bdd_or(xy, zw);
    let under = mgr.bdd_under_approx(f, 4, 2);
    let over = mgr.bdd_over_approx(f, 4, 2);
    // under <= f <= over
    assert!(bdd_implies(&mgr, under, f, 4));
    assert!(bdd_implies(&mgr, f, over, 4));
    // Also: under <= over
    assert!(bdd_implies(&mgr, under, over, 4));
}

#[test]
fn add_to_bdd_to_zdd_pipeline() {
    let mut mgr = Manager::new();
    let _x = mgr.bdd_new_var();
    let _y = mgr.bdd_new_var();

    // Step 1: Build an ADD representing x0 XOR x1 (as 0/1 values)
    let add_x0 = mgr.add_ith_var(0);
    let add_x1 = mgr.add_ith_var(1);
    let add_xor = mgr.add_apply(AddOp::Xor, add_x0, add_x1);

    // Step 2: Convert ADD to BDD via threshold
    let bdd = mgr.add_bdd_pattern(add_xor);

    // Step 3: Verify the BDD represents x0 XOR x1
    let x = mgr.bdd_ith_var(0);
    let y = mgr.bdd_ith_var(1);
    let expected = mgr.bdd_xor(x, y);
    assert!(bdds_equal(&mgr, bdd, expected, 2));

    // Step 4: Convert BDD to ZDD
    let zdd = mgr.zdd_from_bdd(bdd);
    assert!(!zdd.is_zero());
    // The XOR function has 2 satisfying assignments -> ZDD count might differ
    // but should be non-empty
    let count = mgr.zdd_count(zdd);
    assert!(count > 0);
}

#[test]
fn bdd_reorder_then_serialize_roundtrip() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let z = mgr.bdd_new_var();
    let xy = mgr.bdd_and(x, y);
    let f = mgr.bdd_or(xy, z);

    // Record truth table before reorder
    let mut before = Vec::new();
    for row in 0..8u64 {
        let assignment = vec![row & 4 != 0, row & 2 != 0, row & 1 != 0];
        before.push(mgr.bdd_eval(f, &assignment));
    }

    // Reorder
    mgr.reduce_heap(ReorderingMethod::Sift);

    // Serialize to text DDDMP
    let mut buf = Vec::new();
    mgr.dddmp_save_text(f, None, &mut buf).unwrap();

    // Load back into a new manager
    let mut mgr2 = Manager::new();
    let loaded = mgr2.dddmp_load_text(&mut std::io::BufReader::new(&buf[..])).unwrap();

    // Verify truth table matches
    for row in 0..8u64 {
        let assignment = vec![row & 4 != 0, row & 2 != 0, row & 1 != 0];
        let val = mgr2.bdd_eval(loaded, &assignment);
        assert_eq!(val, before[row as usize], "mismatch at row {}", row);
    }
}

#[test]
fn bdd_and_abstract_fused() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // 0
    let y = mgr.bdd_new_var(); // 1
    let z = mgr.bdd_new_var(); // 2
    let f = mgr.bdd_or(x, y);
    let g = mgr.bdd_and(y, z);
    let cube = mgr.bdd_cube(&[1]); // abstract over y

    // Fused
    let fused = mgr.bdd_and_abstract(f, g, cube);
    // Manual: exists y. (f AND g)
    let fg = mgr.bdd_and(f, g);
    let manual = mgr.bdd_exist_abstract(fg, cube);

    assert!(bdds_equal(&mgr, fused, manual, 3));
}

#[test]
fn bdd_univ_abstract_basic() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // 0
    let y = mgr.bdd_new_var(); // 1
    let f = mgr.bdd_or(x, y);
    let cube = mgr.bdd_cube(&[1]);
    // forall y. (x OR y) = x  (when y=0, only x matters)
    let result = mgr.bdd_univ_abstract(f, cube);
    assert_eq!(result, x);
}

#[test]
fn bdd_cube_with_phase() {
    let mut mgr = Manager::new();
    let _x = mgr.bdd_new_var(); // 0
    let _y = mgr.bdd_new_var(); // 1
    // Cube: x0 AND NOT x1
    let cube = mgr.bdd_cube_with_phase(&[0, 1], &[true, false]);
    // Evaluate: only (1, 0) should satisfy
    assert!(mgr.bdd_eval(cube, &[true, false]));
    assert!(!mgr.bdd_eval(cube, &[true, true]));
    assert!(!mgr.bdd_eval(cube, &[false, false]));
    assert!(!mgr.bdd_eval(cube, &[false, true]));
}

#[test]
fn bdd_leq_check() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let f = mgr.bdd_and(x, y);
    // x AND y implies x
    assert!(mgr.bdd_leq(f, x));
    // x does not imply x AND y
    assert!(!mgr.bdd_leq(x, f));
}

#[test]
fn zdd_operations_constants() {
    let mut mgr = Manager::new();
    // Union with ZERO
    let result = mgr.zdd_union(NodeId::ZERO, NodeId::ONE);
    assert_eq!(result, NodeId::ONE);
    // Intersect ONE with ONE
    let result = mgr.zdd_intersect(NodeId::ONE, NodeId::ONE);
    assert_eq!(result, NodeId::ONE);
    // Diff ZERO from anything
    let result = mgr.zdd_diff(NodeId::ZERO, NodeId::ONE);
    assert_eq!(result, NodeId::ZERO);
}

#[test]
fn zdd_product_with_identity() {
    let mut mgr = Manager::new();
    mgr.bdd_new_var();
    mgr.zdd_new_var();
    let s0 = mgr.zdd_change(NodeId::ONE, 0);
    // Product with {emptyset} (ONE) is identity
    let result = mgr.zdd_product(s0, NodeId::ONE);
    assert_eq!(result, s0);
}

#[test]
fn zdd_weak_div_self() {
    let mut mgr = Manager::new();
    mgr.bdd_new_var();
    mgr.zdd_new_var();
    let s0 = mgr.zdd_change(NodeId::ONE, 0);
    // s / s = {emptyset} = ONE
    let result = mgr.zdd_weak_div(s0, s0);
    assert_eq!(result, NodeId::ONE);
}

#[test]
fn zdd_change_toggle() {
    let mut mgr = Manager::new();
    mgr.bdd_new_var();
    mgr.bdd_new_var();
    mgr.zdd_new_var();
    mgr.zdd_new_var();
    // Start with {emptyset}
    // Toggle var 0: {emptyset} -> {{0}}
    let toggled = mgr.zdd_change(NodeId::ONE, 0);
    let count = mgr.zdd_count(toggled);
    assert_eq!(count, 1);
    // Toggle again: {{0}} -> {emptyset}
    let back = mgr.zdd_change(toggled, 0);
    assert_eq!(back, NodeId::ONE);
}

#[test]
fn bdd_remap_under_approx_implies_f() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let z = mgr.bdd_new_var();
    let w = mgr.bdd_new_var();
    let xy = mgr.bdd_and(x, y);
    let zw = mgr.bdd_or(z, w);
    let f = mgr.bdd_or(xy, zw);
    let approx = mgr.bdd_remap_under_approx(f, 4, 2);
    assert!(bdd_implies(&mgr, approx, f, 4));
}

#[test]
fn dump_truth_table_single_var() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let mut buf = Vec::new();
    mgr.dump_truth_table(x, &mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();
    // Should have header + separator + 2 rows
    let data_lines: Vec<&str> = output.lines()
        .filter(|l| l.contains("| 0") || l.contains("| 1"))
        .collect();
    assert_eq!(data_lines.len(), 2);
}

#[test]
fn bdd_verify_sol_basic() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // 0
    let y = mgr.bdd_new_var(); // 1
    // f = x AND y, solve for x: f=0 means NOT(x AND y), particular = NOT(cofactor_pos)
    let f = mgr.bdd_and(x, y);
    let (particular, _care) = mgr.bdd_solve_eqn(f, 0);
    let verified = mgr.bdd_verify_sol(f, &[0], &[particular]);
    assert!(verified, "solution should be verified");
}
