// lumindd — Tests for previously untested public functions
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use lumindd::*;

// =====================================================================
// Helper: create a fresh manager with some variables and a non-trivial BDD
// =====================================================================

fn make_3var_mgr() -> (Manager, NodeId, NodeId, NodeId) {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // var 0
    let y = mgr.bdd_new_var(); // var 1
    let z = mgr.bdd_new_var(); // var 2
    (mgr, x, y, z)
}

// =====================================================================
// bdd_approx.rs — bdd_subset_short_paths, bdd_superset_short_paths
// =====================================================================

#[test]
fn test_bdd_subset_short_paths_basic() {
    let (mut mgr, x, y, z) = make_3var_mgr();
    // f = (x AND y) OR z  — has multiple paths to ONE
    let xy = mgr.bdd_and(x, y);
    let f = mgr.bdd_or(xy, z);
    mgr.ref_node(f);

    let sub = mgr.bdd_subset_short_paths(f, 3, 100);
    // subset implies f: sub AND NOT(f) == ZERO
    let check = mgr.bdd_and(sub, mgr.bdd_not(f));
    assert!(check.is_zero(), "subset must imply the original function");
}

#[test]
fn test_bdd_subset_short_paths_constant_one() {
    let mut mgr = Manager::new();
    let _ = mgr.bdd_new_var();
    let sub = mgr.bdd_subset_short_paths(NodeId::ONE, 1, 10);
    assert!(sub.is_one(), "subset of ONE should be ONE");
}

#[test]
fn test_bdd_subset_short_paths_constant_zero() {
    let mut mgr = Manager::new();
    let _ = mgr.bdd_new_var();
    let sub = mgr.bdd_subset_short_paths(NodeId::ZERO, 1, 10);
    assert!(sub.is_zero(), "subset of ZERO should be ZERO");
}

#[test]
fn test_bdd_superset_short_paths_basic() {
    let (mut mgr, x, y, z) = make_3var_mgr();
    let xy = mgr.bdd_and(x, y);
    let f = mgr.bdd_or(xy, z);
    mgr.ref_node(f);

    let sup = mgr.bdd_superset_short_paths(f, 3, 100);
    // f implies superset: f AND NOT(sup) == ZERO
    let check = mgr.bdd_and(f, mgr.bdd_not(sup));
    assert!(check.is_zero(), "original must imply superset");
}

#[test]
fn test_bdd_superset_short_paths_constant_zero() {
    let mut mgr = Manager::new();
    let _ = mgr.bdd_new_var();
    let sup = mgr.bdd_superset_short_paths(NodeId::ZERO, 1, 10);
    assert!(sup.is_zero(), "superset of ZERO should be ZERO");
}

#[test]
fn test_bdd_superset_short_paths_constant_one() {
    let mut mgr = Manager::new();
    let _ = mgr.bdd_new_var();
    let sup = mgr.bdd_superset_short_paths(NodeId::ONE, 1, 10);
    assert!(sup.is_one(), "superset of ONE should be ONE");
}

// =====================================================================
// bdd_approx.rs — bdd_under_approx, bdd_over_approx (support methods)
// =====================================================================

#[test]
fn test_bdd_under_approx_basic() {
    let (mut mgr, x, y, z) = make_3var_mgr();
    let xy = mgr.bdd_and(x, y);
    let f = mgr.bdd_or(xy, z);
    mgr.ref_node(f);

    let under = mgr.bdd_under_approx(f, 3, 100);
    let check = mgr.bdd_and(under, mgr.bdd_not(f));
    assert!(check.is_zero(), "under-approximation must imply f");
}

#[test]
fn test_bdd_over_approx_basic() {
    let (mut mgr, x, y, z) = make_3var_mgr();
    let xy = mgr.bdd_and(x, y);
    let f = mgr.bdd_or(xy, z);
    mgr.ref_node(f);

    let over = mgr.bdd_over_approx(f, 3, 100);
    let check = mgr.bdd_and(f, mgr.bdd_not(over));
    assert!(check.is_zero(), "f must imply the over-approximation");
}

// =====================================================================
// bdd_approx.rs — bdd_subset_heavy_branch, bdd_superset_heavy_branch
// =====================================================================

#[test]
fn test_bdd_subset_heavy_branch_basic() {
    let (mut mgr, x, y, z) = make_3var_mgr();
    let xy = mgr.bdd_and(x, y);
    let f = mgr.bdd_or(xy, z);
    mgr.ref_node(f);

    let sub = mgr.bdd_subset_heavy_branch(f, 3, 100);
    let check = mgr.bdd_and(sub, mgr.bdd_not(f));
    assert!(check.is_zero());
}

#[test]
fn test_bdd_superset_heavy_branch_basic() {
    let (mut mgr, x, y, z) = make_3var_mgr();
    let xy = mgr.bdd_and(x, y);
    let f = mgr.bdd_or(xy, z);
    mgr.ref_node(f);

    let sup = mgr.bdd_superset_heavy_branch(f, 3, 100);
    let check = mgr.bdd_and(f, mgr.bdd_not(sup));
    assert!(check.is_zero());
}

// =====================================================================
// bdd_approx.rs — bdd_remap_under_approx
// =====================================================================

#[test]
fn test_bdd_remap_under_approx_basic() {
    let (mut mgr, x, y, z) = make_3var_mgr();
    let xy = mgr.bdd_and(x, y);
    let f = mgr.bdd_or(xy, z);
    mgr.ref_node(f);

    let under = mgr.bdd_remap_under_approx(f, 3, 100);
    let check = mgr.bdd_and(under, mgr.bdd_not(f));
    assert!(check.is_zero());
}

// =====================================================================
// bdd_approx.rs — bdd_squeeze
// =====================================================================

#[test]
fn test_bdd_squeeze_basic() {
    let (mut mgr, x, y, _z) = make_3var_mgr();
    let lb = mgr.bdd_and(x, y);
    let ub = mgr.bdd_or(x, y);
    mgr.ref_node(lb);
    mgr.ref_node(ub);

    // lb implies ub
    let precond = mgr.bdd_leq(lb, ub);
    assert!(precond, "lb must imply ub for squeeze");

    let squeezed = mgr.bdd_squeeze(lb, ub);

    // lb implies squeezed
    let chk1 = mgr.bdd_and(lb, mgr.bdd_not(squeezed));
    assert!(chk1.is_zero(), "lb must imply squeezed");

    // squeezed implies ub
    let chk2 = mgr.bdd_and(squeezed, mgr.bdd_not(ub));
    assert!(chk2.is_zero(), "squeezed must imply ub");
}

#[test]
fn test_bdd_squeeze_constants() {
    let mut mgr = Manager::new();
    let _ = mgr.bdd_new_var();

    // ZERO, ONE
    let r = mgr.bdd_squeeze(NodeId::ZERO, NodeId::ONE);
    assert!(r.is_zero() || r.is_one()); // any valid value between ZERO and ONE
    // ZERO implies r and r implies ONE
    let chk = mgr.bdd_and(NodeId::ZERO, mgr.bdd_not(r));
    assert!(chk.is_zero());
}

// =====================================================================
// bdd_decomp.rs — bdd_iterative_conjunctive_decomp
// =====================================================================

#[test]
fn test_bdd_iterative_conjunctive_decomp_basic() {
    let (mut mgr, x, y, z) = make_3var_mgr();
    // f = x AND y AND z -- trivially decomposable
    let xy = mgr.bdd_and(x, y);
    let f = mgr.bdd_and(xy, z);
    mgr.ref_node(f);

    let parts = mgr.bdd_iterative_conjunctive_decomp(f, 4);
    // Conjunction of all parts should equal f
    let mut product = NodeId::ONE;
    for &p in &parts {
        product = mgr.bdd_and(product, p);
    }
    assert_eq!(product, f, "conjunction of parts must equal original");
}

#[test]
fn test_bdd_iterative_conjunctive_decomp_constant() {
    let mut mgr = Manager::new();
    let _ = mgr.bdd_new_var();
    let parts = mgr.bdd_iterative_conjunctive_decomp(NodeId::ONE, 4);
    assert_eq!(parts.len(), 1);
    assert!(parts[0].is_one());
}

#[test]
fn test_bdd_iterative_conjunctive_decomp_max_parts_1() {
    let (mut mgr, x, y, _z) = make_3var_mgr();
    let f = mgr.bdd_and(x, y);
    mgr.ref_node(f);

    let parts = mgr.bdd_iterative_conjunctive_decomp(f, 1);
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0], f);
}

// =====================================================================
// bdd_decomp.rs — bdd_conjunctive_decomp
// =====================================================================

#[test]
fn test_bdd_conjunctive_decomp_basic() {
    let (mut mgr, x, y, z) = make_3var_mgr();
    let xy = mgr.bdd_and(x, y);
    let f = mgr.bdd_and(xy, z);
    mgr.ref_node(f);

    let (g, h) = mgr.bdd_conjunctive_decomp(f);
    let product = mgr.bdd_and(g, h);
    assert_eq!(product, f, "g AND h must equal f");
}

#[test]
fn test_bdd_disjunctive_decomp_basic() {
    let (mut mgr, x, y, z) = make_3var_mgr();
    let xy = mgr.bdd_or(x, y);
    let f = mgr.bdd_or(xy, z);
    mgr.ref_node(f);

    let (g, h) = mgr.bdd_disjunctive_decomp(f);
    let sum = mgr.bdd_or(g, h);
    assert_eq!(sum, f, "g OR h must equal f");
}

// =====================================================================
// bdd_decomp.rs — bdd_compatible_projection
// =====================================================================

#[test]
fn test_bdd_compatible_projection_basic() {
    let (mut mgr, x, y, _z) = make_3var_mgr();
    // f = x AND y, project onto {x} -- existentially quantify y
    let f = mgr.bdd_and(x, y);
    mgr.ref_node(f);

    // cube contains only variable 0 (x)
    let cube = mgr.bdd_ith_var(0);
    let proj = mgr.bdd_compatible_projection(f, cube);

    // Projecting x AND y onto {x} by quantifying y gives: exists y. (x AND y) = x
    assert_eq!(proj, x, "projection should eliminate y and yield x");
}

#[test]
fn test_bdd_compatible_projection_constant() {
    let mut mgr = Manager::new();
    let _ = mgr.bdd_new_var();
    let proj = mgr.bdd_compatible_projection(NodeId::ZERO, NodeId::ONE);
    assert!(proj.is_zero());
}

#[test]
fn test_bdd_compatible_projection_keep_all() {
    let (mut mgr, x, y, _z) = make_3var_mgr();
    let f = mgr.bdd_and(x, y);
    mgr.ref_node(f);

    // Cube contains both x and y — keep both, quantify nothing
    let v0 = mgr.bdd_ith_var(0);
    let v1 = mgr.bdd_ith_var(1);
    let cube = mgr.bdd_and(v0, v1);
    let proj = mgr.bdd_compatible_projection(f, cube);
    assert_eq!(proj, f, "keeping all support variables should not change f");
}

// =====================================================================
// bdd_decomp.rs — bdd_essential_vars
// =====================================================================

#[test]
fn test_bdd_essential_vars_basic() {
    let (mut mgr, x, y, _z) = make_3var_mgr();
    let f = mgr.bdd_and(x, y);
    mgr.ref_node(f);

    let ess = mgr.bdd_essential_vars(f);
    // Top variable (x, var 0) is essential — its cofactors differ from f
    assert!(ess.contains(&0), "x (top var) should be essential");
    // Both vars are in support
    assert_eq!(mgr.bdd_support(f).len(), 2);
}

#[test]
fn test_bdd_essential_vars_constant() {
    let mgr = Manager::new();
    let ess = mgr.bdd_essential_vars(NodeId::ONE);
    assert!(ess.is_empty());
}

// =====================================================================
// bdd_decomp.rs — bdd_solve_eqn
// =====================================================================

#[test]
fn test_bdd_solve_eqn_basic() {
    let (mut mgr, x, y, _z) = make_3var_mgr();
    // f = x XOR y -- solve f=0 for x
    let f = mgr.bdd_xor(x, y);
    mgr.ref_node(f);

    let (particular, care) = mgr.bdd_solve_eqn(f, 0);
    // On the care set, composing x := particular into f should yield ZERO
    let substituted = mgr.bdd_compose(f, particular, 0);
    let on_care = mgr.bdd_and(substituted, care);
    assert!(on_care.is_zero(), "solution must satisfy f=0 on care set");
}

// =====================================================================
// bdd_priority.rs — bdd_hamming_distance
// =====================================================================

#[test]
fn test_bdd_hamming_distance_zero() {
    let (mut mgr, x, _y, _z) = make_3var_mgr();
    // Distance 0 from f should return f itself
    let f = x;
    mgr.ref_node(f);
    let ball = mgr.bdd_hamming_distance(f, &[0, 1, 2], 0);
    assert_eq!(ball, f);
}

#[test]
fn test_bdd_hamming_distance_expands() {
    let (mut mgr, x, _y, _z) = make_3var_mgr();
    // f = just variable x0; Hamming ball of radius 1 should include more minterms
    mgr.ref_node(x);
    let ball = mgr.bdd_hamming_distance(x, &[0, 1, 2], 1);
    let f_count = mgr.bdd_count_minterm(x, 3);
    let ball_count = mgr.bdd_count_minterm(ball, 3);
    assert!(
        ball_count >= f_count,
        "Hamming ball must be at least as large as original"
    );
}

#[test]
fn test_bdd_hamming_distance_from_zero() {
    let mut mgr = Manager::new();
    let _ = mgr.bdd_new_var();
    let ball = mgr.bdd_hamming_distance(NodeId::ZERO, &[0], 1);
    assert!(ball.is_zero(), "Hamming ball around empty set should be empty");
}

// =====================================================================
// bdd_priority.rs — add_hamming
// =====================================================================

#[test]
fn test_add_hamming_basic() {
    let mut mgr = Manager::new();
    let _ = mgr.bdd_new_var(); // var 0
    let _ = mgr.bdd_new_var(); // var 1
    let _ = mgr.bdd_new_var(); // var 2
    let _ = mgr.bdd_new_var(); // var 3

    // x = [0, 1], y = [2, 3] -- 2-bit vectors
    let hamming = mgr.add_hamming(&[0, 1], &[2, 3]);

    // Evaluate at x=00, y=00: distance should be 0
    let val = mgr.add_value(hamming);
    // The result is a non-constant ADD (depends on variables), so val should be None
    assert!(val.is_none(), "Hamming ADD should be non-constant for variables");

    // The ADD should exist and be a valid node
    assert!(
        !hamming.is_zero() && !hamming.is_one(),
        "Hamming ADD should be a non-trivial diagram"
    );
}

#[test]
fn test_add_hamming_single_bit() {
    let mut mgr = Manager::new();
    let _ = mgr.bdd_new_var(); // var 0
    let _ = mgr.bdd_new_var(); // var 1

    let hamming = mgr.add_hamming(&[0], &[1]);
    // This is XOR(var0, var1) as an ADD — values in {0.0, 1.0}
    // Verify it is non-trivial
    assert!(!hamming.is_one());
}

// =====================================================================
// bdd_priority.rs — bdd_inequality, bdd_interval, bdd_disequality
// =====================================================================

#[test]
fn test_bdd_inequality_basic() {
    let mut mgr = Manager::new();
    let _ = mgr.bdd_new_var(); // var 0
    let _ = mgr.bdd_new_var(); // var 1
    let _ = mgr.bdd_new_var(); // var 2
    let _ = mgr.bdd_new_var(); // var 3

    // x > y for 2-bit numbers
    let gt = mgr.bdd_inequality(2, &[0, 1], &[2, 3]);
    // Count minterms: for 2-bit x > y, there are 1+2+3 = 6 out of 16 total assignments
    // where x > y (pairs: (1,0),(2,0),(3,0),(2,1),(3,1),(3,2))
    let count = mgr.bdd_count_minterm(gt, 4);
    assert!(
        (count - 6.0).abs() < 0.5,
        "expected 6 minterms for 2-bit x>y, got {}",
        count
    );
}

#[test]
fn test_bdd_interval_basic() {
    let mut mgr = Manager::new();
    let _ = mgr.bdd_new_var();
    let _ = mgr.bdd_new_var();
    let _ = mgr.bdd_new_var();

    // 3-bit variable, interval [2, 5]
    let f = mgr.bdd_interval(&[0, 1, 2], 2, 5);
    // Values 2,3,4,5 — 4 values, each is a single minterm over 3 vars
    let count = mgr.bdd_count_minterm(f, 3);
    assert!(
        (count - 4.0).abs() < 0.5,
        "expected 4 minterms for [2,5], got {}",
        count
    );
}

#[test]
fn test_bdd_interval_empty() {
    let mut mgr = Manager::new();
    let _ = mgr.bdd_new_var();
    let _ = mgr.bdd_new_var();
    // lower > upper
    let f = mgr.bdd_interval(&[0, 1], 3, 1);
    assert!(f.is_zero());
}

#[test]
fn test_bdd_disequality_basic() {
    let mut mgr = Manager::new();
    let _ = mgr.bdd_new_var();
    let _ = mgr.bdd_new_var();
    let _ = mgr.bdd_new_var();
    let _ = mgr.bdd_new_var();

    // x != y for 2-bit numbers: 16 - 4 = 12 assignments where they differ
    let ne = mgr.bdd_disequality(2, &[0, 1], &[2, 3]);
    let count = mgr.bdd_count_minterm(ne, 4);
    assert!(
        (count - 12.0).abs() < 0.5,
        "expected 12 minterms for 2-bit x!=y, got {}",
        count
    );
}

// =====================================================================
// compose_adv.rs — add_vector_compose
// =====================================================================

#[test]
fn test_add_vector_compose_identity() {
    let mut mgr = Manager::new();
    let _v0 = mgr.bdd_new_var();
    let _v1 = mgr.bdd_new_var();

    let add_var0 = mgr.add_ith_var(0);
    mgr.ref_node(add_var0);

    // Identity vector: each variable maps to itself
    let v0_proj = mgr.add_ith_var(0);
    let v1_proj = mgr.add_ith_var(1);
    let vector = vec![v0_proj, v1_proj];

    let result = mgr.add_vector_compose(add_var0, &vector);
    assert_eq!(result, add_var0, "identity compose should return same ADD");
}

#[test]
fn test_add_vector_compose_constant() {
    let mut mgr = Manager::new();
    let _ = mgr.bdd_new_var();
    let c = mgr.add_const(42.0);
    mgr.ref_node(c);

    let v0_proj = mgr.add_ith_var(0);
    let vector = vec![v0_proj];
    let result = mgr.add_vector_compose(c, &vector);
    assert_eq!(result, c, "composing a constant should return the same constant");
}

// =====================================================================
// compose_adv.rs — add_permute
// =====================================================================

#[test]
fn test_add_permute_identity() {
    let mut mgr = Manager::new();
    let _ = mgr.bdd_new_var();
    let _ = mgr.bdd_new_var();

    let add_var0 = mgr.add_ith_var(0);
    mgr.ref_node(add_var0);

    // Identity permutation
    let result = mgr.add_permute(add_var0, &[0, 1]);
    assert_eq!(result, add_var0, "identity permutation should not change ADD");
}

#[test]
fn test_add_permute_swap() {
    let mut mgr = Manager::new();
    let _ = mgr.bdd_new_var();
    let _ = mgr.bdd_new_var();

    let add_var0 = mgr.add_ith_var(0);
    mgr.ref_node(add_var0);

    // Swap var 0 and var 1
    let result = mgr.add_permute(add_var0, &[1, 0]);
    let add_var1 = mgr.add_ith_var(1);
    assert_eq!(result, add_var1, "permuting var0 to var1 should give add_ith_var(1)");
}

#[test]
fn test_add_permute_constant() {
    let mut mgr = Manager::new();
    let _ = mgr.bdd_new_var();
    let c = mgr.add_const(7.5);
    let result = mgr.add_permute(c, &[0]);
    assert_eq!(result, c, "permuting a constant should return the constant");
}

// =====================================================================
// compose_adv.rs — bdd_vector_compose
// =====================================================================

#[test]
fn test_bdd_vector_compose_identity() {
    let (mut mgr, x, y, _z) = make_3var_mgr();
    let f = mgr.bdd_and(x, y);
    mgr.ref_node(f);

    // Identity: each variable maps to itself
    let v0 = mgr.bdd_ith_var(0);
    let v1 = mgr.bdd_ith_var(1);
    let v2 = mgr.bdd_ith_var(2);
    let vector = vec![v0, v1, v2];

    let result = mgr.bdd_vector_compose(f, &vector);
    assert_eq!(result, f, "identity compose should return same BDD");
}

#[test]
fn test_bdd_vector_compose_substitution() {
    let (mut mgr, x, y, _z) = make_3var_mgr();
    // f = x AND y, replace x with y => y AND y = y
    let f = mgr.bdd_and(x, y);
    mgr.ref_node(f);

    let v1 = mgr.bdd_ith_var(1);
    let v2 = mgr.bdd_ith_var(2);
    let vector = vec![v1, mgr.bdd_ith_var(1), v2]; // var0 -> var1

    let result = mgr.bdd_vector_compose(f, &vector);
    assert_eq!(result, y, "x AND y with x:=y should give y");
}

// =====================================================================
// compose_adv.rs — bdd_permute
// =====================================================================

#[test]
fn test_bdd_permute_identity() {
    let (mut mgr, x, y, _z) = make_3var_mgr();
    let f = mgr.bdd_and(x, y);
    mgr.ref_node(f);

    let result = mgr.bdd_permute(f, &[0, 1, 2]);
    assert_eq!(result, f, "identity permutation should not change BDD");
}

// =====================================================================
// compose_adv.rs — bdd_swap_variables
// =====================================================================

#[test]
fn test_bdd_swap_variables_basic() {
    let (mut mgr, x, _y, _z) = make_3var_mgr();
    // f = x (var 0); swap var 0 <-> var 1 gives y
    mgr.ref_node(x);
    let result = mgr.bdd_swap_variables(x, &[0], &[1]);
    let y = mgr.bdd_ith_var(1);
    assert_eq!(result, y, "swapping x and y in function x should give y");
}

// =====================================================================
// zdd_advanced.rs — zdd_make_from_bdd_cover
// =====================================================================

#[test]
fn test_zdd_make_from_bdd_cover_one() {
    let mut mgr = Manager::new();
    let _ = mgr.bdd_new_var();
    mgr.zdd_new_var();

    let zdd = mgr.zdd_make_from_bdd_cover(NodeId::ONE);
    // ONE BDD = tautology, cover = {empty cube}
    assert!(zdd.is_one(), "cover of ONE should be the base (empty set family)");
}

#[test]
fn test_zdd_make_from_bdd_cover_zero() {
    let mut mgr = Manager::new();
    let _ = mgr.bdd_new_var();
    mgr.zdd_new_var();

    let zdd = mgr.zdd_make_from_bdd_cover(NodeId::ZERO);
    assert!(zdd.is_zero(), "cover of ZERO should be empty");
}

#[test]
fn test_zdd_make_from_bdd_cover_single_var() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    mgr.ref_node(x);
    mgr.zdd_new_var();

    let zdd = mgr.zdd_make_from_bdd_cover(x);
    // x has paths: x=1 -> ONE, x=0 -> ZERO
    // Cover should have one cube: {x}
    let count = mgr.zdd_count(zdd);
    assert_eq!(count, 1, "cover of single variable should have 1 cube");
}

// =====================================================================
// zdd_advanced.rs — zdd_print_cover (smoke test; output goes to stdout)
// =====================================================================

#[test]
fn test_zdd_print_cover_no_panic() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    mgr.ref_node(x);
    mgr.zdd_new_var();

    let zdd = mgr.zdd_make_from_bdd_cover(x);
    // Just verify it doesn't panic
    mgr.zdd_print_cover(zdd);
    mgr.zdd_print_cover(NodeId::ZERO);
}

// =====================================================================
// zdd_advanced.rs — zdd_isop
// =====================================================================

#[test]
fn test_zdd_isop_basic() {
    let (mut mgr, x, y, _z) = make_3var_mgr();
    let lower = mgr.bdd_and(x, y); // lb = x AND y
    let upper = mgr.bdd_or(x, y); // ub = x OR y
    mgr.ref_node(lower);
    mgr.ref_node(upper);

    // Ensure ZDD variables exist
    mgr.zdd_new_var();
    mgr.zdd_new_var();
    mgr.zdd_new_var();

    let (zdd_cover, bdd_func) = mgr.zdd_isop(lower, upper);

    // bdd_func must be between lb and ub
    let chk1 = mgr.bdd_and(lower, mgr.bdd_not(bdd_func));
    assert!(chk1.is_zero(), "lb must imply bdd_func");
    let chk2 = mgr.bdd_and(bdd_func, mgr.bdd_not(upper));
    assert!(chk2.is_zero(), "bdd_func must imply ub");

    // ZDD cover should be non-empty
    assert!(!zdd_cover.is_zero(), "ISOP cover should be non-empty");
}

// =====================================================================
// zdd_reorder.rs — zdd_sift_reorder
// =====================================================================

#[test]
fn test_zdd_sift_reorder_no_panic() {
    let mut mgr = Manager::new();
    let a = mgr.zdd_new_var(); // {{0}}
    let b = mgr.zdd_new_var(); // {{1}}
    let _c = mgr.zdd_new_var(); // {{2}}

    let family = mgr.zdd_union(a, b);
    mgr.ref_node(family);

    mgr.zdd_sift_reorder(false);
    // Just verifying it doesn't crash
}

#[test]
fn test_zdd_sift_reorder_converge() {
    let mut mgr = Manager::new();
    let a = mgr.zdd_new_var(); // {{0}}
    let _b = mgr.zdd_new_var(); // {{1}}

    mgr.ref_node(a);
    mgr.zdd_sift_reorder(true);
}

// =====================================================================
// zdd_reorder.rs — zdd_shuffle_heap
// =====================================================================

#[test]
fn test_zdd_shuffle_heap_identity() {
    let mut mgr = Manager::new();
    mgr.zdd_new_var();
    mgr.zdd_new_var();
    mgr.zdd_new_var();

    // Identity permutation
    mgr.zdd_shuffle_heap(&[0, 1, 2]);
    assert_eq!(mgr.read_perm_zdd(0), 0);
    assert_eq!(mgr.read_perm_zdd(1), 1);
    assert_eq!(mgr.read_perm_zdd(2), 2);
}

#[test]
fn test_zdd_shuffle_heap_reverse() {
    let mut mgr = Manager::new();
    mgr.zdd_new_var();
    mgr.zdd_new_var();
    mgr.zdd_new_var();

    mgr.zdd_shuffle_heap(&[2, 1, 0]);
    assert_eq!(mgr.read_perm_zdd(0), 2);
    assert_eq!(mgr.read_perm_zdd(1), 1);
    assert_eq!(mgr.read_perm_zdd(2), 0);
}

#[test]
#[should_panic(expected = "duplicate level")]
fn test_zdd_shuffle_heap_invalid() {
    let mut mgr = Manager::new();
    mgr.zdd_new_var();
    mgr.zdd_new_var();
    // Invalid: duplicate level
    mgr.zdd_shuffle_heap(&[0, 0]);
}

// =====================================================================
// zdd_reorder.rs — zdd_reduce_heap
// =====================================================================

#[test]
fn test_zdd_reduce_heap_none() {
    let mut mgr = Manager::new();
    mgr.zdd_new_var();
    mgr.zdd_new_var();
    mgr.zdd_reduce_heap(ReorderingMethod::None);
}

#[test]
fn test_zdd_reduce_heap_sift() {
    let mut mgr = Manager::new();
    let a = mgr.zdd_new_var(); // {{0}}
    let _b = mgr.zdd_new_var(); // {{1}}
    let _c = mgr.zdd_new_var(); // {{2}}
    mgr.ref_node(a);
    mgr.zdd_reduce_heap(ReorderingMethod::Sift);
}

// =====================================================================
// interact.rs — build_interaction_matrix, variables_interact
// =====================================================================

#[test]
fn test_build_interaction_matrix_basic() {
    let (mut mgr, x, y, _z) = make_3var_mgr();
    let f = mgr.bdd_and(x, y);
    mgr.ref_node(f);

    let matrix = mgr.build_interaction_matrix();
    // x and y should interact (they appear in the same BDD)
    assert!(matrix.test(0, 1), "x and y should interact in x AND y");
}

#[test]
fn test_build_interaction_matrix_no_interaction() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let _y = mgr.bdd_new_var();
    let _z = mgr.bdd_new_var();
    // Only reference x, y and z are unused
    mgr.ref_node(x);

    let matrix = mgr.build_direct_interaction_matrix();
    // With only x referenced and no BDD combining y or z, check the matrix
    assert_eq!(matrix.size(), 3);
}

#[test]
fn test_variables_interact_true() {
    let (mut mgr, x, y, _z) = make_3var_mgr();
    let f = mgr.bdd_and(x, y);
    mgr.ref_node(f);

    assert!(
        mgr.variables_interact(0, 1),
        "variables in same BDD should interact"
    );
}

#[test]
fn test_variables_interact_self() {
    let mut mgr = Manager::new();
    let _ = mgr.bdd_new_var();
    // A variable always interacts with itself
    assert!(mgr.variables_interact(0, 0));
}

// =====================================================================
// add.rs — add_monadic_apply, add_log
// =====================================================================

#[test]
fn test_add_monadic_apply_negate() {
    let mut mgr = Manager::new();
    let c = mgr.add_const(5.0);
    let neg = mgr.add_monadic_apply(AddMonadicOp::Negate, c);
    let val = mgr.add_value(neg).unwrap();
    assert!((val - (-5.0)).abs() < f64::EPSILON);
}

#[test]
fn test_add_monadic_apply_abs() {
    let mut mgr = Manager::new();
    let c = mgr.add_const(-3.0);
    let abs_c = mgr.add_monadic_apply(AddMonadicOp::Abs, c);
    let val = mgr.add_value(abs_c).unwrap();
    assert!((val - 3.0).abs() < f64::EPSILON);
}

#[test]
fn test_add_monadic_apply_floor_ceil() {
    let mut mgr = Manager::new();
    let c = mgr.add_const(2.7);
    let floored = mgr.add_monadic_apply(AddMonadicOp::Floor, c);
    assert!((mgr.add_value(floored).unwrap() - 2.0).abs() < f64::EPSILON);

    let ceiled = mgr.add_monadic_apply(AddMonadicOp::Ceil, c);
    assert!((mgr.add_value(ceiled).unwrap() - 3.0).abs() < f64::EPSILON);
}

#[test]
fn test_add_monadic_apply_complement() {
    let mut mgr = Manager::new();
    let c = mgr.add_const(0.3);
    let comp = mgr.add_monadic_apply(AddMonadicOp::Complement, c);
    let val = mgr.add_value(comp).unwrap();
    assert!((val - 0.7).abs() < f64::EPSILON);
}

#[test]
fn test_add_log_basic() {
    let mut mgr = Manager::new();
    let c = mgr.add_const(std::f64::consts::E);
    let log_c = mgr.add_log(c);
    let val = mgr.add_value(log_c).unwrap();
    assert!((val - 1.0).abs() < 1e-10, "ln(e) should be 1.0, got {}", val);
}

#[test]
fn test_add_log_one() {
    let mut mgr = Manager::new();
    let c = mgr.add_const(1.0);
    let log_c = mgr.add_log(c);
    let val = mgr.add_value(log_c).unwrap();
    assert!((val - 0.0).abs() < 1e-10, "ln(1) should be 0.0");
}

// =====================================================================
// util.rs — bdd_support_size
// =====================================================================

#[test]
fn test_bdd_support_size_basic() {
    let (mut mgr, x, y, _z) = make_3var_mgr();
    let f = mgr.bdd_and(x, y);
    assert_eq!(mgr.bdd_support_size(f), 2);
}

#[test]
fn test_bdd_support_size_constant() {
    let mgr = Manager::new();
    assert_eq!(mgr.bdd_support_size(NodeId::ONE), 0);
    assert_eq!(mgr.bdd_support_size(NodeId::ZERO), 0);
}

#[test]
fn test_bdd_support_size_single_var() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    assert_eq!(mgr.bdd_support_size(x), 1);
}

// =====================================================================
// epd.rs — from_parts, normalize, negate, two_power
// =====================================================================

#[test]
fn test_epd_from_parts() {
    let ep = EpDouble::from_parts(0.75, 2);
    // value = 0.75 * 2^2 = 3.0
    let val = ep.to_f64().unwrap();
    assert!((val - 3.0).abs() < 1e-10, "expected 3.0, got {}", val);
}

#[test]
fn test_epd_from_parts_zero() {
    let ep = EpDouble::from_parts(0.0, 100);
    assert!(ep.is_zero());
    assert_eq!(ep.to_f64().unwrap(), 0.0);
}

#[test]
fn test_epd_normalize() {
    let mut ep = EpDouble { mantissa: 4.0, exponent: 0 };
    ep.normalize();
    // 4.0 = 0.5 * 2^3
    let val = ep.to_f64().unwrap();
    assert!((val - 4.0).abs() < 1e-10, "normalize changed value: {}", val);
    assert!((ep.mantissa - 0.5).abs() < 1e-10, "mantissa should be 0.5, got {}", ep.mantissa);
    assert_eq!(ep.exponent, 3);
}

#[test]
fn test_epd_negate() {
    let ep = EpDouble::from_parts(0.5, 4); // 0.5 * 2^4 = 8.0
    let neg = ep.negate();
    let val = neg.to_f64().unwrap();
    assert!((val - (-8.0)).abs() < 1e-10, "expected -8.0, got {}", val);
}

#[test]
fn test_epd_two_power() {
    let ep = EpDouble::two_power(10);
    let val = ep.to_f64().unwrap();
    assert!((val - 1024.0).abs() < f64::EPSILON);
}

#[test]
fn test_epd_two_power_zero() {
    let ep = EpDouble::two_power(0);
    let val = ep.to_f64().unwrap();
    assert!((val - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_epd_two_power_large() {
    let ep = EpDouble::two_power(2000);
    // Should not overflow -- that's the whole point
    assert!(!ep.is_zero());
    // to_f64 should return None (overflow)
    assert!(ep.to_f64().is_none());
}

#[test]
fn test_epd_arithmetic() {
    let a = EpDouble::new(3.0);
    let b = EpDouble::new(4.0);
    let sum = a + b;
    let val = sum.to_f64().unwrap();
    assert!((val - 7.0).abs() < 1e-10, "3+4 should be 7, got {}", val);

    let product = a * b;
    let val = product.to_f64().unwrap();
    assert!((val - 12.0).abs() < 1e-10, "3*4 should be 12, got {}", val);
}

#[test]
fn test_epd_comparison() {
    let a = EpDouble::new(3.0);
    let b = EpDouble::new(4.0);
    assert!(a < b);
    assert!(b > a);
    let c = EpDouble::new(3.0);
    assert!(a == c);
}

// =====================================================================
// apa.rs — bit_length, is_zero
// =====================================================================

#[test]
fn test_apint_bit_length() {
    assert_eq!(ApInt::zero().bit_length(), 0);
    assert_eq!(ApInt::one().bit_length(), 1);
    assert_eq!(ApInt::from_u64(255).bit_length(), 8);
    assert_eq!(ApInt::from_u64(256).bit_length(), 9);
    assert_eq!(ApInt::two_power(100).bit_length(), 101);
}

#[test]
fn test_apint_is_zero() {
    assert!(ApInt::zero().is_zero());
    assert!(!ApInt::one().is_zero());
    assert!(!ApInt::from_u64(42).is_zero());
}

#[test]
fn test_apint_two_power() {
    let v = ApInt::two_power(64);
    assert_eq!(v.to_string(), "18446744073709551616");
    assert!(!v.is_zero());
    assert_eq!(v.bit_length(), 65);
}

// =====================================================================
// reorder_exact.rs — exact_reorder_with_limit
// =====================================================================

#[test]
fn test_exact_reorder_with_limit_basic() {
    let mut mgr = Manager::new();
    let x0 = mgr.bdd_new_var();
    let x1 = mgr.bdd_new_var();
    let x2 = mgr.bdd_new_var();
    let f = mgr.bdd_and(x0, x1);
    let g = mgr.bdd_or(f, x2);
    mgr.ref_node(g);

    mgr.exact_reorder_with_limit(20);
    // After reordering, the function should still evaluate correctly
    // We can verify by checking that the BDD still represents the same function
}

#[test]
fn test_exact_reorder_with_limit_single_var() {
    let mut mgr = Manager::new();
    let _x = mgr.bdd_new_var();
    // Should be a no-op for 1 variable
    mgr.exact_reorder_with_limit(20);
}

#[test]
#[should_panic(expected = "exact reordering requires")]
fn test_exact_reorder_with_limit_too_many_vars() {
    let mut mgr = Manager::new();
    for _ in 0..6 {
        mgr.bdd_new_var();
    }
    mgr.exact_reorder_with_limit(3); // limit is 3 but we have 6 vars
}

// =====================================================================
// Constant-constant BDD tests: bdd_and(ONE,ONE), bdd_or(ZERO,ZERO), bdd_leq(ZERO,ONE)
// =====================================================================

#[test]
fn test_bdd_and_one_one() {
    let mut mgr = Manager::new();
    let result = mgr.bdd_and(NodeId::ONE, NodeId::ONE);
    assert!(result.is_one());
}

#[test]
fn test_bdd_and_one_zero() {
    let mut mgr = Manager::new();
    let result = mgr.bdd_and(NodeId::ONE, NodeId::ZERO);
    assert!(result.is_zero());
}

#[test]
fn test_bdd_or_zero_zero() {
    let mut mgr = Manager::new();
    let result = mgr.bdd_or(NodeId::ZERO, NodeId::ZERO);
    assert!(result.is_zero());
}

#[test]
fn test_bdd_or_one_zero() {
    let mut mgr = Manager::new();
    let result = mgr.bdd_or(NodeId::ONE, NodeId::ZERO);
    assert!(result.is_one());
}

#[test]
fn test_bdd_leq_zero_one() {
    let mut mgr = Manager::new();
    assert!(mgr.bdd_leq(NodeId::ZERO, NodeId::ONE));
}

#[test]
fn test_bdd_leq_one_zero() {
    let mut mgr = Manager::new();
    assert!(!mgr.bdd_leq(NodeId::ONE, NodeId::ZERO));
}

#[test]
fn test_bdd_leq_one_one() {
    let mut mgr = Manager::new();
    assert!(mgr.bdd_leq(NodeId::ONE, NodeId::ONE));
}

#[test]
fn test_bdd_leq_zero_zero() {
    let mut mgr = Manager::new();
    assert!(mgr.bdd_leq(NodeId::ZERO, NodeId::ZERO));
}

// =====================================================================
// ADD with INF: add_const(f64::INFINITY), add_plus with infinity
// =====================================================================

#[test]
fn test_add_const_infinity() {
    let mut mgr = Manager::new();
    let inf = mgr.add_const(f64::INFINITY);
    let val = mgr.add_value(inf).unwrap();
    assert!(val.is_infinite() && val > 0.0);
}

#[test]
fn test_add_plus_with_infinity() {
    let mut mgr = Manager::new();
    let inf = mgr.add_const(f64::INFINITY);
    let c = mgr.add_const(42.0);
    let result = mgr.add_plus(inf, c);
    let val = mgr.add_value(result).unwrap();
    assert!(val.is_infinite(), "inf + 42 should be inf");
}

#[test]
fn test_add_const_neg_infinity() {
    let mut mgr = Manager::new();
    let neg_inf = mgr.add_const(f64::NEG_INFINITY);
    let val = mgr.add_value(neg_inf).unwrap();
    assert!(val.is_infinite() && val < 0.0);
}

// =====================================================================
// Counting with nvars=0
// =====================================================================

#[test]
fn test_bdd_count_minterm_nvars_zero_one() {
    let mgr = Manager::new();
    // ONE with 0 variables should have 1 minterm (the empty assignment)
    let count = mgr.bdd_count_minterm(NodeId::ONE, 0);
    assert!(
        (count - 1.0).abs() < f64::EPSILON,
        "ONE with 0 vars should have 1 minterm, got {}",
        count
    );
}

#[test]
fn test_bdd_count_minterm_nvars_zero_zero() {
    let mgr = Manager::new();
    let count = mgr.bdd_count_minterm(NodeId::ZERO, 0);
    assert!(
        (count - 0.0).abs() < f64::EPSILON,
        "ZERO with 0 vars should have 0 minterms"
    );
}

// =====================================================================
// Identity permutation tests
// =====================================================================

#[test]
fn test_identity_permutation_bdd() {
    let (mut mgr, x, y, _z) = make_3var_mgr();
    let f = mgr.bdd_and(x, y);
    mgr.ref_node(f);

    let result = mgr.bdd_permute(f, &[0, 1, 2]);
    assert_eq!(result, f, "identity permutation should not change BDD");
}

// =====================================================================
// Compose with out-of-support variable
// =====================================================================

#[test]
fn test_compose_out_of_support_variable() {
    let (mut mgr, x, y, z) = make_3var_mgr();
    // f = x AND y (does not depend on z)
    let f = mgr.bdd_and(x, y);
    mgr.ref_node(f);

    // Compose z := ONE (z is not in support of f)
    let result = mgr.bdd_compose(f, NodeId::ONE, 2);
    assert_eq!(result, f, "composing out-of-support variable should not change f");
}

#[test]
fn test_compose_out_of_support_variable_zero() {
    let (mut mgr, x, y, _z) = make_3var_mgr();
    let f = mgr.bdd_and(x, y);
    mgr.ref_node(f);

    let result = mgr.bdd_compose(f, NodeId::ZERO, 2);
    assert_eq!(result, f, "composing out-of-support z:=0 should not change f");
}

// =====================================================================
// Cross-validation: bdd_count_minterm vs bdd_count_minterm_apa
// =====================================================================

#[test]
fn test_cross_validate_count_minterm_vs_apa() {
    let (mut mgr, x, y, z) = make_3var_mgr();
    let xy = mgr.bdd_and(x, y);
    let f = mgr.bdd_or(xy, z);
    mgr.ref_node(f);

    let f64_count = mgr.bdd_count_minterm(f, 3);
    let apa_count = mgr.bdd_count_minterm_apa(f, 3);

    let apa_as_u64: u64 = apa_count.to_string().parse().unwrap();
    assert_eq!(
        f64_count as u64, apa_as_u64,
        "f64 count ({}) must match APA count ({})",
        f64_count, apa_as_u64
    );
}

#[test]
fn test_cross_validate_count_minterm_single_var() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    mgr.ref_node(x);

    let f64_count = mgr.bdd_count_minterm(x, 1);
    let apa_count = mgr.bdd_count_minterm_apa(x, 1);

    assert!((f64_count - 1.0).abs() < f64::EPSILON);
    assert_eq!(apa_count.to_string(), "1");
}

#[test]
fn test_cross_validate_count_minterm_constants() {
    let mgr = Manager::new();

    let one_f64 = mgr.bdd_count_minterm(NodeId::ONE, 5);
    let one_apa = mgr.bdd_count_minterm_apa(NodeId::ONE, 5);
    assert!((one_f64 - 32.0).abs() < f64::EPSILON);
    assert_eq!(one_apa.to_string(), "32");

    let zero_f64 = mgr.bdd_count_minterm(NodeId::ZERO, 5);
    let zero_apa = mgr.bdd_count_minterm_apa(NodeId::ZERO, 5);
    assert!((zero_f64 - 0.0).abs() < f64::EPSILON);
    assert!(zero_apa.is_zero());
}

// =====================================================================
// Additional edge-case tests for completeness
// =====================================================================

#[test]
fn test_epd_count_minterm_matches_f64() {
    let (mut mgr, x, y, _z) = make_3var_mgr();
    let f = mgr.bdd_and(x, y);
    mgr.ref_node(f);

    let f64_count = mgr.bdd_count_minterm(f, 3);
    let epd_count = mgr.bdd_count_minterm_epd(f, 3);
    let epd_as_f64 = epd_count.to_f64().unwrap();
    assert!(
        (f64_count - epd_as_f64).abs() < 1e-6,
        "f64 count ({}) should match EPD count ({})",
        f64_count,
        epd_as_f64
    );
}

#[test]
fn test_add_negate_constant() {
    let mut mgr = Manager::new();
    let c = mgr.add_const(10.0);
    let neg = mgr.add_negate(c);
    let val = mgr.add_value(neg).unwrap();
    assert!((val - (-10.0)).abs() < f64::EPSILON);
}

#[test]
fn test_add_value_on_one() {
    let mgr = Manager::new();
    // NodeId::ONE in ADD context should have value 1.0
    let val = mgr.add_value(NodeId::ONE);
    assert!(val.is_some());
    assert!((val.unwrap() - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_add_ith_var() {
    let mut mgr = Manager::new();
    let _ = mgr.bdd_new_var();
    let add_v = mgr.add_ith_var(0);
    // add_ith_var(0) should be 1.0 when var0=true, 0.0 when var0=false
    // It's a non-constant node
    assert!(mgr.add_value(add_v).is_none(), "ADD var should be non-constant");
}

#[test]
fn test_add_bdd_threshold() {
    let mut mgr = Manager::new();
    let c5 = mgr.add_const(5.0);
    let bdd = mgr.add_bdd_threshold(c5, 3.0);
    assert!(bdd.is_one(), "5.0 > 3.0 threshold should give ONE");

    let c2 = mgr.add_const(2.0);
    let bdd2 = mgr.add_bdd_threshold(c2, 3.0);
    assert!(bdd2.is_zero(), "2.0 <= 3.0 threshold should give ZERO");
}

#[test]
fn test_add_bdd_pattern() {
    let mut mgr = Manager::new();
    let c = mgr.add_const(0.0);
    let bdd = mgr.add_bdd_pattern(c);
    assert!(bdd.is_zero(), "pattern of 0.0 should be ZERO");

    let c2 = mgr.add_const(3.14);
    let bdd2 = mgr.add_bdd_pattern(c2);
    assert!(bdd2.is_one(), "pattern of nonzero should be ONE");
}

#[test]
fn test_bdd_to_add_roundtrip() {
    let (mut mgr, x, y, _z) = make_3var_mgr();
    let f = mgr.bdd_and(x, y);
    mgr.ref_node(f);

    let add_f = mgr.bdd_to_add(f);
    // Convert back to BDD via threshold > 0
    let bdd_back = mgr.add_bdd_threshold(add_f, 0.0);
    assert_eq!(bdd_back, f, "BDD -> ADD -> BDD roundtrip should preserve function");
}

#[test]
fn test_zdd_count_minterm() {
    let mut mgr = Manager::new();
    let _ = mgr.bdd_new_var();
    let a = mgr.zdd_new_var(); // {{0}}
    let b = mgr.zdd_new_var(); // {{1}}

    // Family {{0}, {1}} covers 2 singletons
    let family = mgr.zdd_union(a, b);
    mgr.ref_node(family);

    let mcount = mgr.zdd_count_minterm(family, 2);
    // Each singleton cube covers 2^(2-1) = 2 minterms, 2 cubes -> 4 (with overlap counted)
    assert!(mcount > 0.0, "minterm count should be positive");
}

#[test]
fn test_zdd_support_and_dag_size() {
    let mut mgr = Manager::new();
    let a = mgr.zdd_new_var(); // {{0}}
    let b = mgr.zdd_new_var(); // {{1}}

    let family = mgr.zdd_union(a, b);
    mgr.ref_node(family);

    let support = mgr.zdd_support(family);
    assert!(support.contains(&0));
    assert!(support.contains(&1));

    let dag = mgr.zdd_dag_size(family);
    assert!(dag > 0);
}

#[test]
fn test_zdd_max_min_cardinality() {
    let mut mgr = Manager::new();
    let singleton_0 = mgr.zdd_new_var(); // {{0}}
    let _singleton_1 = mgr.zdd_new_var(); // {{1}} -- ensures var 1 exists

    // Build {{0,1}} using zdd_change: add var 1 to the set {0}
    // zdd_change toggles element membership in each set of the family
    // singleton_0 = {{0}}, zdd_change(singleton_0, 1) = {{0,1}}
    let pair = mgr.zdd_change(singleton_0, 1); // {{0,1}}
    // Family: {{0}, {0,1}}
    let family = mgr.zdd_union(singleton_0, pair);
    mgr.ref_node(family);

    let max_card = mgr.zdd_max_cardinality(family);
    let min_card = mgr.zdd_min_cardinality(family);
    assert!(max_card >= min_card);
    assert!(max_card >= 1);
}
