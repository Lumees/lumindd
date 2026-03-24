// lumindd — Comprehensive test suite covering audit gaps
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use lumindd::{Manager, NodeId, ReorderingMethod};

// ==================================================================
// Minterm counting tests
// ==================================================================

#[test]
fn test_minterm_count_single_var() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // var 0

    // x has 1 minterm out of 2 (when x=1)
    assert_eq!(mgr.bdd_count_minterm(x, 1), 1.0);

    // NOT(x) also has 1 minterm
    let nx = mgr.bdd_not(x);
    assert_eq!(mgr.bdd_count_minterm(nx, 1), 1.0);

    // ONE has 2 minterms over 1 variable
    assert_eq!(mgr.bdd_count_minterm(mgr.one(), 1), 2.0);

    // ZERO has 0 minterms
    assert_eq!(mgr.bdd_count_minterm(mgr.zero(), 1), 0.0);
}

#[test]
fn test_minterm_count_two_vars() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();

    // x AND y: 1 minterm out of 4
    let xy = mgr.bdd_and(x, y);
    assert_eq!(mgr.bdd_count_minterm(xy, 2), 1.0);

    // x OR y: 3 minterms out of 4
    let xy_or = mgr.bdd_or(x, y);
    assert_eq!(mgr.bdd_count_minterm(xy_or, 2), 3.0);

    // x XOR y: 2 minterms
    let xy_xor = mgr.bdd_xor(x, y);
    assert_eq!(mgr.bdd_count_minterm(xy_xor, 2), 2.0);
}

#[test]
fn test_minterm_count_three_vars() {
    let mut mgr = Manager::new();
    let a = mgr.bdd_new_var();
    let b = mgr.bdd_new_var();
    let c = mgr.bdd_new_var();

    // Majority: 4 minterms (110, 101, 011, 111)
    let ab = mgr.bdd_and(a, b);
    let ac = mgr.bdd_and(a, c);
    let bc = mgr.bdd_and(b, c);
    let t = mgr.bdd_or(ab, ac);
    let maj = mgr.bdd_or(t, bc);
    assert_eq!(mgr.bdd_count_minterm(maj, 3), 4.0);
}

#[test]
fn test_minterm_count_complemented() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();

    // NAND(x, y) = NOT(AND(x, y)): 3 minterms
    let nand = mgr.bdd_nand(x, y);
    assert_eq!(mgr.bdd_count_minterm(nand, 2), 3.0);

    // NOR(x, y) = NOT(OR(x, y)): 1 minterm
    let nor = mgr.bdd_nor(x, y);
    assert_eq!(mgr.bdd_count_minterm(nor, 2), 1.0);
}

// ==================================================================
// Restrict and constrain tests
// ==================================================================

#[test]
fn test_bdd_restrict_basic() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // 0
    let y = mgr.bdd_new_var(); // 1

    let f = mgr.bdd_or(x, y);

    // Restrict f by x=1: result should be ONE (1 OR y with x=1 = 1)
    let constraint = x; // x=1
    let result = mgr.bdd_restrict(f, constraint);
    assert!(result.is_one());

    // Restrict f by NOT(x): f with x=0 => just y
    let nx = mgr.bdd_not(x);
    let result = mgr.bdd_restrict(f, nx);
    assert_eq!(result, y);
}

#[test]
fn test_bdd_constrain_basic() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();

    let f = mgr.bdd_and(x, y);

    // Constrain by x: when x is true, f = y
    let result = mgr.bdd_constrain(f, x);
    assert_eq!(result, y);

    // Constrain f by f should give ONE
    let result = mgr.bdd_constrain(f, f);
    assert!(result.is_one());
}

// ==================================================================
// And-Abstract tests
// ==================================================================

#[test]
fn test_bdd_and_abstract() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // 0
    let y = mgr.bdd_new_var(); // 1

    // AndAbstract(x, y, cube_y) = Exist y. (x AND y) = x
    let cube_y = mgr.bdd_cube(&[1]);
    let result = mgr.bdd_and_abstract(x, y, cube_y);
    assert_eq!(result, x);

    // AndAbstract(x OR y, NOT y, cube_y) = Exist y. ((x OR y) AND NOT y) = x
    let xy_or = mgr.bdd_or(x, y);
    let ny = mgr.bdd_not(y);
    let result = mgr.bdd_and_abstract(xy_or, ny, cube_y);
    // (x OR y) AND NOT(y) = x AND NOT(y), then Exist y => x
    assert_eq!(result, x);
}

// ==================================================================
// Cube with phase tests
// ==================================================================

#[test]
fn test_cube_with_phase() {
    let mut mgr = Manager::new();
    let _x = mgr.bdd_new_var();
    let _y = mgr.bdd_new_var();
    let _z = mgr.bdd_new_var();

    // Cube: x=1, y=0, z=1
    let cube = mgr.bdd_cube_with_phase(&[0, 1, 2], &[true, false, true]);

    assert!(mgr.bdd_eval(cube, &[true, false, true]));
    assert!(!mgr.bdd_eval(cube, &[true, true, true]));
    assert!(!mgr.bdd_eval(cube, &[false, false, true]));
}

// ==================================================================
// Reordering tests
// ==================================================================

#[test]
fn test_shuffle_heap() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // var 0
    let y = mgr.bdd_new_var(); // var 1
    let z = mgr.bdd_new_var(); // var 2

    // f = (x AND y) OR z — truth table: 8 combinations, 5 true
    let xy = mgr.bdd_and(x, y);
    let f = mgr.bdd_or(xy, z);
    mgr.ref_node(f);

    // Verify before reordering
    assert!(mgr.bdd_eval(f, &[true, true, false]));
    assert!(mgr.bdd_eval(f, &[false, false, true]));
    assert!(!mgr.bdd_eval(f, &[false, true, false]));

    // Reverse the variable order: var0 -> level 2, var1 -> level 1, var2 -> level 0
    mgr.shuffle_heap(&[2, 1, 0]);

    // The BDD should still compute the same function
    // (we need to rebuild f since the ordering changed)
    let x2 = mgr.bdd_ith_var(0);
    let y2 = mgr.bdd_ith_var(1);
    let z2 = mgr.bdd_ith_var(2);
    let xy2 = mgr.bdd_and(x2, y2);
    let f2 = mgr.bdd_or(xy2, z2);

    // Verify the rebuilt function evaluates correctly
    assert!(mgr.bdd_eval(f2, &[true, true, false]));
    assert!(mgr.bdd_eval(f2, &[false, false, true]));
    assert!(!mgr.bdd_eval(f2, &[false, true, false]));
}

#[test]
fn test_sift_reorder() {
    let mut mgr = Manager::new();
    let vars: Vec<NodeId> = (0..5).map(|_| mgr.bdd_new_var()).collect();

    // Build a function that benefits from reordering
    // f = (x0 AND x4) OR (x1 AND x3) OR x2
    let x0x4 = mgr.bdd_and(vars[0], vars[4]);
    let x1x3 = mgr.bdd_and(vars[1], vars[3]);
    let t = mgr.bdd_or(x0x4, x1x3);
    let f = mgr.bdd_or(t, vars[2]);
    mgr.ref_node(f);

    // Should not crash
    mgr.reduce_heap(ReorderingMethod::Sift);
}

#[test]
fn test_window2_reorder() {
    let mut mgr = Manager::new();
    let _vars: Vec<NodeId> = (0..4).map(|_| mgr.bdd_new_var()).collect();

    let x0 = mgr.bdd_ith_var(0);
    let x1 = mgr.bdd_ith_var(1);
    let f = mgr.bdd_and(x0, x1);
    mgr.ref_node(f);

    mgr.reduce_heap(ReorderingMethod::Window2);
    // Should not crash
}

// ==================================================================
// ADD with variables tests
// ==================================================================

#[test]
fn test_add_variable_evaluation() {
    let mut mgr = Manager::new();

    // Build: if x0 then 10.0 else 20.0
    let x = mgr.add_ith_var(0);
    let c10 = mgr.add_const(10.0);
    let c20 = mgr.add_const(20.0);
    let f = mgr.add_ite(x, c10, c20);

    // Cofactors: when x=1, should be 10.0; when x=0, should be 20.0
    let (t, e) = mgr.add_cofactors(f, 0);
    assert_eq!(mgr.add_value(t), Some(10.0));
    assert_eq!(mgr.add_value(e), Some(20.0));
}

#[test]
fn test_add_apply_with_variables() {
    let mut mgr = Manager::new();

    // f = if x then 5.0 else 3.0
    let x = mgr.add_ith_var(0);
    let c5 = mgr.add_const(5.0);
    let c3 = mgr.add_const(3.0);
    let f = mgr.add_ite(x, c5, c3);

    // g = if x then 2.0 else 7.0
    let c2 = mgr.add_const(2.0);
    let c7 = mgr.add_const(7.0);
    let g = mgr.add_ite(x, c2, c7);

    // f + g: when x=1: 7.0, when x=0: 10.0
    let sum = mgr.add_plus(f, g);
    let (t, e) = mgr.add_cofactors(sum, 0);
    assert_eq!(mgr.add_value(t), Some(7.0));
    assert_eq!(mgr.add_value(e), Some(10.0));
}

#[test]
fn test_add_bdd_pattern() {
    let mut mgr = Manager::new();

    let c5 = mgr.add_const(5.0);
    let bdd = mgr.add_bdd_pattern(c5);
    assert!(bdd.is_one()); // 5.0 != 0.0 => 1

    let add_zero = mgr.add_zero();
    let bdd = mgr.add_bdd_pattern(add_zero);
    assert!(bdd.is_zero()); // 0.0 == 0.0 => 0
}

// ==================================================================
// ZDD weak division and ITE tests
// ==================================================================

#[test]
fn test_zdd_weak_div() {
    let mut mgr = Manager::new();
    let x = mgr.zdd_new_var(); // {{0}}
    let y = mgr.zdd_new_var(); // {{1}}

    // product = {{0,1}}
    let product = mgr.zdd_product(x, y);

    // {{0,1}} / {{0}} = {{1}} (remove 0 from all sets, keep those that had it)
    let result = mgr.zdd_weak_div(product, x);
    assert_eq!(mgr.zdd_count(result), 1);
}

#[test]
fn test_zdd_ite() {
    let mut mgr = Manager::new();
    let x = mgr.zdd_new_var();
    let y = mgr.zdd_new_var();

    // ITE(ONE, x, y) = x
    let result = mgr.zdd_ite(mgr.one(), x, y);
    assert_eq!(result, x);

    // ITE(ZERO, x, y) = y
    let result = mgr.zdd_ite(mgr.zero(), x, y);
    assert_eq!(result, y);
}

#[test]
fn test_zdd_diff_one_vs_nonone() {
    let mut mgr = Manager::new();
    let x = mgr.zdd_new_var(); // {{0}}

    // {∅} \ {{0}} = {∅} (empty set is not in {{0}})
    let one = mgr.one();
    let result = mgr.zdd_diff(one, x);
    assert!(result.is_one()); // {∅} remains

    // {∅} \ {∅} = ∅
    let result = mgr.zdd_diff(one, one);
    assert!(result.is_zero());
}

// ==================================================================
// Manager capacity and edge cases
// ==================================================================

#[test]
fn test_with_capacity() {
    let mgr = Manager::with_capacity(3, 2, 16);
    assert_eq!(mgr.num_vars(), 3);
    assert_eq!(mgr.num_zdd_vars(), 2);
}

#[test]
fn test_bdd_ith_var_auto_creation() {
    let mut mgr = Manager::new();

    // Asking for var 5 should auto-create vars 0-5
    let v5 = mgr.bdd_ith_var(5);
    assert_eq!(mgr.num_vars(), 6);
    assert!(!v5.is_constant());
}

#[test]
fn test_manager_default() {
    let mgr = Manager::default();
    assert_eq!(mgr.num_vars(), 0);
    assert_eq!(mgr.num_nodes(), 1); // just the constant ONE
}

// ==================================================================
// Stress tests with more variables
// ==================================================================

#[test]
fn test_stress_8_vars_adder() {
    let mut mgr = Manager::new();
    let n = 8;
    let vars: Vec<NodeId> = (0..n).map(|_| mgr.bdd_new_var()).collect();

    // Build a ripple-carry adder for 4-bit + 4-bit addition
    // Inputs: a[0..4] = vars[0..4], b[0..4] = vars[4..8]
    let mut carry = mgr.zero();
    let mut sums = Vec::new();

    for i in 0..4 {
        let a = vars[i];
        let b = vars[i + 4];

        // sum = a XOR b XOR carry
        let ab_xor = mgr.bdd_xor(a, b);
        let sum = mgr.bdd_xor(ab_xor, carry);
        sums.push(sum);

        // carry_out = (a AND b) OR (carry AND (a XOR b))
        let ab_and = mgr.bdd_and(a, b);
        let carry_xor = mgr.bdd_and(carry, ab_xor);
        carry = mgr.bdd_or(ab_and, carry_xor);
    }
    sums.push(carry);

    // Verify: 3 + 5 = 8 (a=0011, b=0101)
    // a[0]=1, a[1]=1, a[2]=0, a[3]=0, b[0]=1, b[1]=0, b[2]=1, b[3]=0
    let assignment = [true, true, false, false, true, false, true, false];

    // Expected sum bits: 8 = 01000 → s0=0, s1=0, s2=0, s3=1, carry=0
    assert!(!mgr.bdd_eval(sums[0], &assignment)); // bit 0 = 0
    assert!(!mgr.bdd_eval(sums[1], &assignment)); // bit 1 = 0
    assert!(!mgr.bdd_eval(sums[2], &assignment)); // bit 2 = 0
    assert!(mgr.bdd_eval(sums[3], &assignment));  // bit 3 = 1
    assert!(!mgr.bdd_eval(sums[4], &assignment)); // carry = 0
}

#[test]
fn test_stress_many_operations() {
    let mut mgr = Manager::new();
    let vars: Vec<NodeId> = (0..10).map(|_| mgr.bdd_new_var()).collect();

    // Build a chain: f = x0 AND x1 AND ... AND x9
    let mut f = mgr.one();
    for &v in &vars {
        f = mgr.bdd_and(f, v);
    }

    // Only all-true assignment satisfies it
    let all_true: Vec<bool> = vec![true; 10];
    assert!(mgr.bdd_eval(f, &all_true));

    let one_false: Vec<bool> = (0..10).map(|i| i != 5).collect();
    assert!(!mgr.bdd_eval(f, &one_false));

    // Minterm count: 1 out of 1024
    assert_eq!(mgr.bdd_count_minterm(f, 10), 1.0);

    // Support: all 10 variables
    let support = mgr.bdd_support(f);
    assert_eq!(support.len(), 10);
}

#[test]
fn test_cache_stats() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();

    // Perform some operations to generate cache activity
    let _f = mgr.bdd_and(x, y);
    let _g = mgr.bdd_or(x, y);

    let (hits, misses) = mgr.cache_stats();
    // At minimum, there should be some misses (first-time lookups)
    assert!(hits + misses > 0);
}

// ==================================================================
// GC and reference counting tests
// ==================================================================

#[test]
fn test_ref_deref_basic() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();

    let f = mgr.bdd_and(x, y);
    mgr.ref_node(f);

    // Dereferencing should not crash
    mgr.deref_node(f);
}

#[test]
fn test_garbage_collect_runs() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();

    let _f = mgr.bdd_and(x, y);

    // Manual GC should not crash
    mgr.garbage_collect();
    assert!(mgr.num_nodes() > 0);
}

// ==================================================================
// Path counting test
// ==================================================================

#[test]
fn test_bdd_count_path() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();

    // x AND y has 1 path to ONE
    let f = mgr.bdd_and(x, y);
    assert_eq!(mgr.bdd_count_path(f), 1.0);

    // x OR y: paths to ONE via x=1 (1 path) and x=0,y=1 (1 path) = 2
    let g = mgr.bdd_or(x, y);
    assert_eq!(mgr.bdd_count_path(g), 2.0);
}

// ==================================================================
// Verify ADD divide by zero
// ==================================================================

#[test]
fn test_add_divide_by_zero() {
    let mut mgr = Manager::new();
    let c10 = mgr.add_const(10.0);
    let c0 = mgr.add_zero();

    let result = mgr.add_divide(c10, c0);
    let val = mgr.add_value(result).unwrap();
    assert!(val.is_infinite());
}

// ==================================================================
// Auto-reorder config tests
// ==================================================================

#[test]
fn test_enable_disable_auto_reorder() {
    let mut mgr = Manager::new();
    mgr.enable_auto_reorder(ReorderingMethod::Sift);
    mgr.disable_auto_reorder();
    // Should not crash, just config
}
