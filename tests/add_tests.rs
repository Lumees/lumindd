// lumindd — ADD operation tests
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use lumindd::Manager;

#[test]
fn test_add_constants() {
    let mut mgr = Manager::new();

    let c5 = mgr.add_const(5.0);
    let c3 = mgr.add_const(3.0);

    assert_eq!(mgr.add_value(c5), Some(5.0));
    assert_eq!(mgr.add_value(c3), Some(3.0));

    // ONE is 1.0
    assert_eq!(mgr.add_value(mgr.one()), Some(1.0));
}

#[test]
fn test_add_plus() {
    let mut mgr = Manager::new();

    let c2 = mgr.add_const(2.0);
    let c3 = mgr.add_const(3.0);

    let result = mgr.add_plus(c2, c3);
    assert_eq!(mgr.add_value(result), Some(5.0));
}

#[test]
fn test_add_times() {
    let mut mgr = Manager::new();

    let c4 = mgr.add_const(4.0);
    let c5 = mgr.add_const(5.0);

    let result = mgr.add_times(c4, c5);
    assert_eq!(mgr.add_value(result), Some(20.0));
}

#[test]
fn test_add_minus() {
    let mut mgr = Manager::new();

    let c10 = mgr.add_const(10.0);
    let c3 = mgr.add_const(3.0);

    let result = mgr.add_minus(c10, c3);
    assert_eq!(mgr.add_value(result), Some(7.0));
}

#[test]
fn test_add_min_max() {
    let mut mgr = Manager::new();

    let c2 = mgr.add_const(2.0);
    let c7 = mgr.add_const(7.0);

    let min_result = mgr.add_min(c2, c7);
    assert_eq!(mgr.add_value(min_result), Some(2.0));

    let max_result = mgr.add_max(c2, c7);
    assert_eq!(mgr.add_value(max_result), Some(7.0));
}

#[test]
fn test_add_negate() {
    let mut mgr = Manager::new();

    let c5 = mgr.add_const(5.0);
    let neg = mgr.add_negate(c5);
    assert_eq!(mgr.add_value(neg), Some(-5.0));
}

#[test]
fn test_add_variable() {
    let mut mgr = Manager::new();

    // ADD variable x0: maps to 1.0 when true, 0.0 when false
    let x = mgr.add_ith_var(0);
    assert!(!x.is_constant());
}

#[test]
fn test_add_ite_with_variable() {
    let mut mgr = Manager::new();

    let x = mgr.add_ith_var(0);
    let c10 = mgr.add_const(10.0);
    let c20 = mgr.add_const(20.0);

    // ITE(x, 10, 20): returns 10 when x=1, 20 when x=0
    let result = mgr.add_ite(x, c10, c20);
    assert!(!result.is_constant());
}

#[test]
fn test_bdd_to_add() {
    let mut mgr = Manager::new();

    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();

    let f = mgr.bdd_and(x, y);
    let add_f = mgr.bdd_to_add(f);

    // The ADD should be non-constant (depends on x and y)
    assert!(!add_f.is_constant());
}

#[test]
fn test_add_bdd_threshold() {
    let mut mgr = Manager::new();

    let c5 = mgr.add_const(5.0);
    let c3 = mgr.add_const(3.0);

    // 5.0 > 4.0 => ONE
    let bdd5 = mgr.add_bdd_threshold(c5, 4.0);
    assert!(bdd5.is_one());

    // 3.0 > 4.0 => ZERO
    let bdd3 = mgr.add_bdd_threshold(c3, 4.0);
    assert!(bdd3.is_zero());
}
