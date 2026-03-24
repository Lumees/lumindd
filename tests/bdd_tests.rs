// lumindd — BDD operation tests
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use lumindd::Manager;

#[test]
fn test_constants() {
    let mgr = Manager::new();
    let one = mgr.one();
    let zero = mgr.zero();

    assert!(one.is_one());
    assert!(zero.is_zero());
    assert!(!one.is_zero());
    assert!(!zero.is_one());
    assert!(one.is_constant());
    assert!(zero.is_constant());
    assert_eq!(one.not(), zero);
    assert_eq!(zero.not(), one);
}

#[test]
fn test_variable_creation() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();

    assert_eq!(mgr.num_vars(), 2);
    assert!(!x.is_constant());
    assert!(!y.is_constant());
    assert_ne!(x, y);
}

#[test]
fn test_not() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();

    let nx = mgr.bdd_not(x);
    assert_ne!(x, nx);
    assert_eq!(mgr.bdd_not(nx), x); // double negation
}

#[test]
fn test_and() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();

    // x AND 1 = x
    let r = mgr.bdd_and(x, mgr.one());
    assert_eq!(r, x);

    // x AND 0 = 0
    let r = mgr.bdd_and(x, mgr.zero());
    assert!(r.is_zero());

    // x AND x = x
    let r = mgr.bdd_and(x, x);
    assert_eq!(r, x);

    // x AND NOT(x) = 0
    let nx = mgr.bdd_not(x);
    let r = mgr.bdd_and(x, nx);
    assert!(r.is_zero());

    // Evaluate x AND y
    let xy = mgr.bdd_and(x, y);
    assert!(mgr.bdd_eval(xy, &[true, true]));
    assert!(!mgr.bdd_eval(xy, &[true, false]));
    assert!(!mgr.bdd_eval(xy, &[false, true]));
    assert!(!mgr.bdd_eval(xy, &[false, false]));
}

#[test]
fn test_or() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();

    let xy_or = mgr.bdd_or(x, y);
    assert!(mgr.bdd_eval(xy_or, &[true, true]));
    assert!(mgr.bdd_eval(xy_or, &[true, false]));
    assert!(mgr.bdd_eval(xy_or, &[false, true]));
    assert!(!mgr.bdd_eval(xy_or, &[false, false]));
}

#[test]
fn test_xor() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();

    let xy_xor = mgr.bdd_xor(x, y);
    assert!(!mgr.bdd_eval(xy_xor, &[true, true]));
    assert!(mgr.bdd_eval(xy_xor, &[true, false]));
    assert!(mgr.bdd_eval(xy_xor, &[false, true]));
    assert!(!mgr.bdd_eval(xy_xor, &[false, false]));
}

#[test]
fn test_nand_nor_xnor() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();

    let nand = mgr.bdd_nand(x, y);
    assert!(!mgr.bdd_eval(nand, &[true, true]));
    assert!(mgr.bdd_eval(nand, &[true, false]));

    let nor = mgr.bdd_nor(x, y);
    assert!(!mgr.bdd_eval(nor, &[true, false]));
    assert!(mgr.bdd_eval(nor, &[false, false]));

    let xnor = mgr.bdd_xnor(x, y);
    assert!(mgr.bdd_eval(xnor, &[true, true]));
    assert!(!mgr.bdd_eval(xnor, &[true, false]));
    assert!(mgr.bdd_eval(xnor, &[false, false]));
}

#[test]
fn test_ite() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let z = mgr.bdd_new_var();

    // ITE(x, y, z) = (x AND y) OR (NOT(x) AND z)
    let ite = mgr.bdd_ite(x, y, z);

    assert!(mgr.bdd_eval(ite, &[true, true, false]));   // x=1 => y=1
    assert!(!mgr.bdd_eval(ite, &[true, false, true]));   // x=1 => y=0
    assert!(mgr.bdd_eval(ite, &[false, false, true]));   // x=0 => z=1
    assert!(!mgr.bdd_eval(ite, &[false, true, false]));  // x=0 => z=0
}

#[test]
fn test_tautology_and_unsat() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();

    // x OR NOT(x) = 1
    let nx = mgr.bdd_not(x);
    let taut = mgr.bdd_or(x, nx);
    assert!(mgr.bdd_is_tautology(taut));

    // x AND NOT(x) = 0
    let unsat = mgr.bdd_and(x, nx);
    assert!(mgr.bdd_is_unsat(unsat));
}

#[test]
fn test_leq() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();

    let xy = mgr.bdd_and(x, y);

    // (x AND y) implies x
    assert!(mgr.bdd_leq(xy, x));

    // x does NOT imply (x AND y) in general
    assert!(!mgr.bdd_leq(x, xy));
}

#[test]
fn test_exist_abstract() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // var 0
    let y = mgr.bdd_new_var(); // var 1

    let xy = mgr.bdd_and(x, y);

    // Exist y. (x AND y) = x
    let cube_y = mgr.bdd_cube(&[1]);
    let result = mgr.bdd_exist_abstract(xy, cube_y);
    assert_eq!(result, x);

    // Exist x. (x AND y) = y
    let cube_x = mgr.bdd_cube(&[0]);
    let result = mgr.bdd_exist_abstract(xy, cube_x);
    assert_eq!(result, y);
}

#[test]
fn test_univ_abstract() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();

    let xy_or = mgr.bdd_or(x, y);

    // Forall y. (x OR y) = x (because when y=0, result is x; when y=1, result is 1)
    let cube_y = mgr.bdd_cube(&[1]);
    let result = mgr.bdd_univ_abstract(xy_or, cube_y);
    assert_eq!(result, x);
}

#[test]
fn test_compose() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // var 0
    let y = mgr.bdd_new_var(); // var 1
    let z = mgr.bdd_new_var(); // var 2

    // f = x AND y, substitute z for x => z AND y
    let f = mgr.bdd_and(x, y);
    let result = mgr.bdd_compose(f, z, 0);

    let expected = mgr.bdd_and(z, y);
    assert_eq!(result, expected);
}

#[test]
fn test_support() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // var 0
    let y = mgr.bdd_new_var(); // var 1
    let _z = mgr.bdd_new_var(); // var 2

    let f = mgr.bdd_and(x, y);
    let support = mgr.bdd_support(f);
    assert_eq!(support, vec![0, 1]);
}

#[test]
fn test_dag_size() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();

    let f = mgr.bdd_and(x, y);
    let size = mgr.dag_size(f);
    // x AND y: x -> y -> ONE, plus x -> ZERO (else), y -> ZERO (else)
    // Nodes: x, y, constant = 3
    assert!(size >= 2); // at least x, y internal nodes
}

#[test]
fn test_pick_one_cube() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();

    let f = mgr.bdd_and(x, y);

    let cube = mgr.bdd_pick_one_cube(f).unwrap();
    assert!(mgr.bdd_eval(f, &cube));

    // ZERO has no satisfying assignment
    assert!(mgr.bdd_pick_one_cube(mgr.zero()).is_none());
}

#[test]
fn test_iter_cubes() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();

    let f = mgr.bdd_xor(x, y);
    let cubes = mgr.bdd_iter_cubes(f);
    assert_eq!(cubes.len(), 2);
}

#[test]
fn test_eval() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let z = mgr.bdd_new_var();

    // f = (x AND y) OR z
    let xy = mgr.bdd_and(x, y);
    let f = mgr.bdd_or(xy, z);

    assert!(mgr.bdd_eval(f, &[true, true, false]));
    assert!(mgr.bdd_eval(f, &[false, false, true]));
    assert!(!mgr.bdd_eval(f, &[false, true, false]));
    assert!(mgr.bdd_eval(f, &[true, true, true]));
}

#[test]
fn test_de_morgan() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();

    // NOT(x AND y) == NOT(x) OR NOT(y)
    let lhs = mgr.bdd_nand(x, y);
    let nx = mgr.bdd_not(x);
    let ny = mgr.bdd_not(y);
    let rhs = mgr.bdd_or(nx, ny);
    assert_eq!(lhs, rhs);

    // NOT(x OR y) == NOT(x) AND NOT(y)
    let lhs = mgr.bdd_nor(x, y);
    let rhs = mgr.bdd_and(nx, ny);
    assert_eq!(lhs, rhs);
}

#[test]
fn test_dot_export() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();

    let f = mgr.bdd_and(x, y);
    let mut buf = Vec::new();
    mgr.dump_dot(f, &mut buf).unwrap();
    let dot = String::from_utf8(buf).unwrap();
    assert!(dot.contains("digraph BDD"));
    assert!(dot.contains("x0"));
    assert!(dot.contains("x1"));
}

#[test]
fn test_cube_construction() {
    let mut mgr = Manager::new();
    let _x = mgr.bdd_new_var(); // var 0
    let _y = mgr.bdd_new_var(); // var 1
    let _z = mgr.bdd_new_var(); // var 2

    // Cube of all 3 vars
    let cube = mgr.bdd_cube(&[0, 1, 2]);

    // Only assignment 1,1,1 satisfies the cube
    assert!(mgr.bdd_eval(cube, &[true, true, true]));
    assert!(!mgr.bdd_eval(cube, &[true, true, false]));
    assert!(!mgr.bdd_eval(cube, &[false, true, true]));
}

#[test]
fn test_three_var_majority() {
    let mut mgr = Manager::new();
    let a = mgr.bdd_new_var();
    let b = mgr.bdd_new_var();
    let c = mgr.bdd_new_var();

    // Majority function: at least 2 of 3 are true
    let ab = mgr.bdd_and(a, b);
    let ac = mgr.bdd_and(a, c);
    let bc = mgr.bdd_and(b, c);
    let ab_or_ac = mgr.bdd_or(ab, ac);
    let maj = mgr.bdd_or(ab_or_ac, bc);

    assert!(!mgr.bdd_eval(maj, &[false, false, false]));
    assert!(!mgr.bdd_eval(maj, &[true, false, false]));
    assert!(!mgr.bdd_eval(maj, &[false, true, false]));
    assert!(!mgr.bdd_eval(maj, &[false, false, true]));
    assert!(mgr.bdd_eval(maj, &[true, true, false]));
    assert!(mgr.bdd_eval(maj, &[true, false, true]));
    assert!(mgr.bdd_eval(maj, &[false, true, true]));
    assert!(mgr.bdd_eval(maj, &[true, true, true]));
}
