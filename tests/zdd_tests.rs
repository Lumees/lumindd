// lumindd — ZDD operation tests
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use lumindd::Manager;

#[test]
fn test_zdd_constants() {
    let mgr = Manager::new();
    let one = mgr.one();
    let zero = mgr.zero();

    // In ZDD: ONE = {∅} (family containing the empty set), ZERO = ∅ (empty family)
    assert_eq!(mgr.zdd_count(one), 1);
    assert_eq!(mgr.zdd_count(zero), 0);
}

#[test]
fn test_zdd_variable_creation() {
    let mut mgr = Manager::new();
    let x = mgr.zdd_new_var();
    let y = mgr.zdd_new_var();

    assert_eq!(mgr.num_zdd_vars(), 2);
    assert!(!x.is_constant());
    assert!(!y.is_constant());

    // Each ZDD variable represents {{var}} — a family with one singleton set
    assert_eq!(mgr.zdd_count(x), 1);
    assert_eq!(mgr.zdd_count(y), 1);
}

#[test]
fn test_zdd_union() {
    let mut mgr = Manager::new();
    let x = mgr.zdd_new_var(); // {{0}}
    let y = mgr.zdd_new_var(); // {{1}}

    // Union: {{0}} ∪ {{1}} = {{0}, {1}}
    let u = mgr.zdd_union(x, y);
    assert_eq!(mgr.zdd_count(u), 2);

    // Union with empty = identity
    let zero = mgr.zero();
    let r = mgr.zdd_union(x, zero);
    assert_eq!(r, x);

    // Union with self = identity
    let r = mgr.zdd_union(x, x);
    assert_eq!(r, x);
}

#[test]
fn test_zdd_intersect() {
    let mut mgr = Manager::new();
    let x = mgr.zdd_new_var();
    let y = mgr.zdd_new_var();

    // {{0}} ∩ {{1}} = ∅
    let i = mgr.zdd_intersect(x, y);
    assert!(i.is_zero());

    // Intersect with self = self
    let r = mgr.zdd_intersect(x, x);
    assert_eq!(r, x);
}

#[test]
fn test_zdd_diff() {
    let mut mgr = Manager::new();
    let x = mgr.zdd_new_var();
    let y = mgr.zdd_new_var();

    let u = mgr.zdd_union(x, y); // {{0}, {1}}

    // {{0}, {1}} \ {{0}} = {{1}}
    let d = mgr.zdd_diff(u, x);
    assert_eq!(mgr.zdd_count(d), 1);

    // Diff with empty = self
    let zero = mgr.zero();
    let r = mgr.zdd_diff(x, zero);
    assert_eq!(r, x);

    // Diff with self = empty
    let r = mgr.zdd_diff(x, x);
    assert!(r.is_zero());
}

#[test]
fn test_zdd_change() {
    let mut mgr = Manager::new();
    let _x = mgr.zdd_new_var(); // var 0
    let _y = mgr.zdd_new_var(); // var 1

    // Change on {∅}: toggle var 0 in ∅ => {{0}}
    let one = mgr.one();
    let result = mgr.zdd_change(one, 0);
    assert_eq!(mgr.zdd_count(result), 1);
}

#[test]
fn test_zdd_subset() {
    let mut mgr = Manager::new();
    let x = mgr.zdd_new_var(); // var 0: represents {{0}}

    // Subset1({{0}}, 0) = the then-child when var matches
    let s1 = mgr.zdd_subset1(x, 0);
    // Sets containing var 0 from {{0}}: the set {0} contains 0, so result is {∅} = ONE
    assert!(s1.is_one());

    // Subset0({{0}}, 0) = sets NOT containing var 0 from {{0}}: none, so ZERO
    let s0 = mgr.zdd_subset0(x, 0);
    assert!(s0.is_zero());
}

#[test]
fn test_zdd_count() {
    let mut mgr = Manager::new();
    let x = mgr.zdd_new_var();
    let y = mgr.zdd_new_var();
    let z = mgr.zdd_new_var();

    // Build {{0}, {1}, {2}}
    let xy = mgr.zdd_union(x, y);
    let xyz = mgr.zdd_union(xy, z);
    assert_eq!(mgr.zdd_count(xyz), 3);
}

#[test]
fn test_zdd_product() {
    let mut mgr = Manager::new();
    let x = mgr.zdd_new_var(); // {{0}}
    let y = mgr.zdd_new_var(); // {{1}}

    // Product of {{0}} and {{1}} = {{0,1}} — cross product
    let p = mgr.zdd_product(x, y);
    assert_eq!(mgr.zdd_count(p), 1); // one set: {0,1}
}

#[test]
fn test_zdd_bdd_roundtrip() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();

    let bdd = mgr.bdd_and(x, y);

    // BDD -> ZDD -> BDD roundtrip
    let zdd = mgr.zdd_from_bdd(bdd);
    let back = mgr.zdd_to_bdd(zdd);

    // Should be equivalent
    assert_eq!(bdd, back);
}
