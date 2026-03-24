// lumindd — Safe RAII wrapper types for decision diagrams
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

//! Safe RAII wrapper types that automatically manage reference counting.
//!
//! These types mirror the pattern used in CUDD's C++ interface: each wrapper
//! holds a shared reference to the [`Manager`] and a [`NodeId`], and the
//! `Clone` / `Drop` implementations take care of incrementing and decrementing
//! the reference count so users never need to call `ref_node` / `deref_node`
//! manually.
//!
//! # Example
//!
//! ```rust
//! use lumindd::wrapper::CuddManager;
//!
//! let mgr = CuddManager::new();
//! let x = mgr.bdd_var(0);
//! let y = mgr.bdd_var(1);
//!
//! let f = x.and(&y);        // x AND y
//! let g = x.or(&y);         // x OR y
//! let h = f.not();           // NOT(x AND y)
//!
//! assert!((f | h).is_one()); // tautology
//! ```

use std::cell::RefCell;
use std::fmt;
use std::ops;
use std::rc::Rc;

use crate::add::{AddMonadicOp, AddOp};
use crate::manager::Manager;
use crate::node::NodeId;

/// Shared reference to a [`Manager`].
///
/// Used internally by the RAII wrappers so that every DD object can reach the
/// manager for reference-counting and operations.
pub type ManagerRef = Rc<RefCell<Manager>>;

// ===========================================================================
// CuddManager — managed DD context
// ===========================================================================

/// A managed decision diagram context.
///
/// Wraps a [`Manager`] in an `Rc<RefCell<…>>` so that BDD/ADD/ZDD objects
/// produced from it can share ownership and invoke operations.
pub struct CuddManager {
    inner: ManagerRef,
}

impl CuddManager {
    /// Create a new managed context with default settings.
    pub fn new() -> Self {
        CuddManager {
            inner: Rc::new(RefCell::new(Manager::new())),
        }
    }

    /// Create a managed context wrapping an existing [`Manager`].
    pub fn from_manager(mgr: Manager) -> Self {
        CuddManager {
            inner: Rc::new(RefCell::new(mgr)),
        }
    }

    /// Get a clone of the inner [`ManagerRef`].
    pub fn manager_ref(&self) -> ManagerRef {
        Rc::clone(&self.inner)
    }

    /// Get the i-th BDD variable (creating it if necessary).
    pub fn bdd_var(&self, i: u16) -> Bdd {
        let node = self.inner.borrow_mut().bdd_ith_var(i);
        Bdd::new(Rc::clone(&self.inner), node)
    }

    /// The BDD constant ONE.
    pub fn bdd_one(&self) -> Bdd {
        Bdd::new(Rc::clone(&self.inner), NodeId::ONE)
    }

    /// The BDD constant ZERO.
    pub fn bdd_zero(&self) -> Bdd {
        Bdd::new(Rc::clone(&self.inner), NodeId::ZERO)
    }

    /// Create an ADD constant with the given value.
    pub fn add_const(&self, val: f64) -> Add {
        let node = self.inner.borrow_mut().add_const(val);
        Add::new(Rc::clone(&self.inner), node)
    }

    /// The number of BDD/ADD variables currently in the manager.
    pub fn num_vars(&self) -> u16 {
        self.inner.borrow().num_vars()
    }

    /// Get the i-th ZDD variable (creating it if necessary).
    pub fn zdd_var(&self, i: u16) -> Zdd {
        let mut mgr = self.inner.borrow_mut();
        while mgr.num_zdd_vars() <= i {
            mgr.zdd_new_var();
        }
        // Build the ZDD singleton for variable i.
        let node = mgr.zdd_unique_inter(i, NodeId::ONE, NodeId::ZERO);
        mgr.ref_node(node);
        drop(mgr);
        Zdd::new(Rc::clone(&self.inner), node)
    }

    /// The ZDD empty family (ZERO).
    pub fn zdd_empty(&self) -> Zdd {
        Zdd::new(Rc::clone(&self.inner), NodeId::ZERO)
    }

    /// The ZDD base (family containing only the empty set).
    pub fn zdd_base(&self) -> Zdd {
        Zdd::new(Rc::clone(&self.inner), NodeId::ONE)
    }
}

impl Default for CuddManager {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Bdd — safe BDD wrapper
// ===========================================================================

/// A BDD with automatic reference counting.
///
/// When cloned, the underlying node's reference count is incremented.
/// When dropped, it is decremented. All operations borrow the shared
/// [`Manager`] through the internal [`ManagerRef`].
pub struct Bdd {
    mgr: ManagerRef,
    node: NodeId,
}

impl Bdd {
    /// Wrap a raw [`NodeId`] and increment its reference count.
    fn new(mgr: ManagerRef, node: NodeId) -> Self {
        mgr.borrow_mut().ref_node(node);
        Bdd { mgr, node }
    }

    /// The underlying [`NodeId`].
    pub fn node_id(&self) -> NodeId {
        self.node
    }

    /// Returns `true` if this BDD is the constant ONE.
    pub fn is_one(&self) -> bool {
        self.node.is_one()
    }

    /// Returns `true` if this BDD is the constant ZERO.
    pub fn is_zero(&self) -> bool {
        self.node.is_zero()
    }

    /// Logical AND of `self` and `other`.
    pub fn and(&self, other: &Bdd) -> Bdd {
        let node = self.mgr.borrow_mut().bdd_and(self.node, other.node);
        Bdd::new(Rc::clone(&self.mgr), node)
    }

    /// Logical OR of `self` and `other`.
    pub fn or(&self, other: &Bdd) -> Bdd {
        let node = self.mgr.borrow_mut().bdd_or(self.node, other.node);
        Bdd::new(Rc::clone(&self.mgr), node)
    }

    /// Logical XOR of `self` and `other`.
    pub fn xor(&self, other: &Bdd) -> Bdd {
        let node = self.mgr.borrow_mut().bdd_xor(self.node, other.node);
        Bdd::new(Rc::clone(&self.mgr), node)
    }

    /// Logical NOT (complement).
    pub fn not(&self) -> Bdd {
        let node = self.mgr.borrow().bdd_not(self.node);
        Bdd::new(Rc::clone(&self.mgr), node)
    }

    /// If-then-else: `if self then then_ else else_`.
    pub fn ite(&self, then_: &Bdd, else_: &Bdd) -> Bdd {
        let node = self
            .mgr
            .borrow_mut()
            .bdd_ite(self.node, then_.node, else_.node);
        Bdd::new(Rc::clone(&self.mgr), node)
    }

    /// Existential abstraction: exists `cube` . `self`.
    pub fn exist_abstract(&self, cube: &Bdd) -> Bdd {
        let node = self
            .mgr
            .borrow_mut()
            .bdd_exist_abstract(self.node, cube.node);
        Bdd::new(Rc::clone(&self.mgr), node)
    }

    /// Substitute `g` for variable `var` in `self`.
    pub fn compose(&self, g: &Bdd, var: u16) -> Bdd {
        let node = self
            .mgr
            .borrow_mut()
            .bdd_compose(self.node, g.node, var);
        Bdd::new(Rc::clone(&self.mgr), node)
    }

    /// Count the number of minterms (satisfying assignments) over `num_vars` variables.
    pub fn count_minterm(&self, num_vars: u32) -> f64 {
        self.mgr.borrow().bdd_count_minterm(self.node, num_vars)
    }

    /// Return the set of variable indices in the support of this BDD.
    pub fn support(&self) -> Vec<u16> {
        self.mgr.borrow().bdd_support(self.node)
    }
}

impl Clone for Bdd {
    fn clone(&self) -> Self {
        self.mgr.borrow_mut().ref_node(self.node);
        Bdd {
            mgr: Rc::clone(&self.mgr),
            node: self.node,
        }
    }
}

impl Drop for Bdd {
    fn drop(&mut self) {
        self.mgr.borrow_mut().deref_node(self.node);
    }
}

impl PartialEq for Bdd {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}

impl Eq for Bdd {}

impl fmt::Debug for Bdd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bdd({:?})", self.node)
    }
}

impl ops::Not for Bdd {
    type Output = Bdd;
    fn not(self) -> Bdd {
        let node = self.mgr.borrow().bdd_not(self.node);
        Bdd::new(Rc::clone(&self.mgr), node)
    }
}

impl ops::Not for &Bdd {
    type Output = Bdd;
    fn not(self) -> Bdd {
        Bdd::not(self)
    }
}

impl ops::BitAnd for Bdd {
    type Output = Bdd;
    fn bitand(self, rhs: Bdd) -> Bdd {
        self.and(&rhs)
    }
}

impl ops::BitAnd for &Bdd {
    type Output = Bdd;
    fn bitand(self, rhs: &Bdd) -> Bdd {
        self.and(rhs)
    }
}

impl ops::BitOr for Bdd {
    type Output = Bdd;
    fn bitor(self, rhs: Bdd) -> Bdd {
        self.or(&rhs)
    }
}

impl ops::BitOr for &Bdd {
    type Output = Bdd;
    fn bitor(self, rhs: &Bdd) -> Bdd {
        self.or(rhs)
    }
}

impl ops::BitXor for Bdd {
    type Output = Bdd;
    fn bitxor(self, rhs: Bdd) -> Bdd {
        self.xor(&rhs)
    }
}

impl ops::BitXor for &Bdd {
    type Output = Bdd;
    fn bitxor(self, rhs: &Bdd) -> Bdd {
        self.xor(rhs)
    }
}

// ===========================================================================
// Add — safe ADD wrapper
// ===========================================================================

/// An ADD (Algebraic Decision Diagram) with automatic reference counting.
pub struct Add {
    mgr: ManagerRef,
    node: NodeId,
}

impl Add {
    /// Wrap a raw [`NodeId`] and increment its reference count.
    fn new(mgr: ManagerRef, node: NodeId) -> Self {
        mgr.borrow_mut().ref_node(node);
        Add { mgr, node }
    }

    /// The underlying [`NodeId`].
    pub fn node_id(&self) -> NodeId {
        self.node
    }

    /// Returns the terminal value if this ADD node is a constant, or `None`.
    pub fn value(&self) -> Option<f64> {
        self.mgr.borrow().add_value(self.node)
    }

    /// ADD addition: `self + other`.
    pub fn plus(&self, other: &Add) -> Add {
        let node = self
            .mgr
            .borrow_mut()
            .add_apply(AddOp::Plus, self.node, other.node);
        Add::new(Rc::clone(&self.mgr), node)
    }

    /// ADD multiplication: `self * other`.
    pub fn times(&self, other: &Add) -> Add {
        let node = self
            .mgr
            .borrow_mut()
            .add_apply(AddOp::Times, self.node, other.node);
        Add::new(Rc::clone(&self.mgr), node)
    }

    /// ADD subtraction: `self - other`.
    pub fn minus(&self, other: &Add) -> Add {
        let node = self
            .mgr
            .borrow_mut()
            .add_apply(AddOp::Minus, self.node, other.node);
        Add::new(Rc::clone(&self.mgr), node)
    }

    /// ADD division: `self / other`.
    pub fn divide(&self, other: &Add) -> Add {
        let node = self
            .mgr
            .borrow_mut()
            .add_apply(AddOp::Divide, self.node, other.node);
        Add::new(Rc::clone(&self.mgr), node)
    }

    /// Element-wise minimum: `min(self, other)`.
    pub fn minimum(&self, other: &Add) -> Add {
        let node = self
            .mgr
            .borrow_mut()
            .add_apply(AddOp::Minimum, self.node, other.node);
        Add::new(Rc::clone(&self.mgr), node)
    }

    /// Element-wise maximum: `max(self, other)`.
    pub fn maximum(&self, other: &Add) -> Add {
        let node = self
            .mgr
            .borrow_mut()
            .add_apply(AddOp::Maximum, self.node, other.node);
        Add::new(Rc::clone(&self.mgr), node)
    }

    /// General binary apply with an arbitrary [`AddOp`].
    pub fn apply(&self, op: AddOp, other: &Add) -> Add {
        let node = self
            .mgr
            .borrow_mut()
            .add_apply(op, self.node, other.node);
        Add::new(Rc::clone(&self.mgr), node)
    }

    /// General unary apply with an [`AddMonadicOp`].
    pub fn monadic_apply(&self, op: AddMonadicOp) -> Add {
        let node = self
            .mgr
            .borrow_mut()
            .add_monadic_apply(op, self.node);
        Add::new(Rc::clone(&self.mgr), node)
    }

    /// Negate: `-self`.
    pub fn negate(&self) -> Add {
        self.monadic_apply(AddMonadicOp::Negate)
    }
}

impl Clone for Add {
    fn clone(&self) -> Self {
        self.mgr.borrow_mut().ref_node(self.node);
        Add {
            mgr: Rc::clone(&self.mgr),
            node: self.node,
        }
    }
}

impl Drop for Add {
    fn drop(&mut self) {
        self.mgr.borrow_mut().deref_node(self.node);
    }
}

impl PartialEq for Add {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}

impl Eq for Add {}

impl fmt::Debug for Add {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Add({:?})", self.node)
    }
}

impl ops::Add for &Add {
    type Output = crate::wrapper::Add;
    fn add(self, rhs: &Add) -> crate::wrapper::Add {
        self.plus(rhs)
    }
}

impl ops::Sub for &Add {
    type Output = crate::wrapper::Add;
    fn sub(self, rhs: &Add) -> crate::wrapper::Add {
        self.minus(rhs)
    }
}

impl ops::Mul for &Add {
    type Output = crate::wrapper::Add;
    fn mul(self, rhs: &Add) -> crate::wrapper::Add {
        self.times(rhs)
    }
}

impl ops::Div for &Add {
    type Output = crate::wrapper::Add;
    fn div(self, rhs: &Add) -> crate::wrapper::Add {
        self.divide(rhs)
    }
}

impl ops::Neg for &Add {
    type Output = crate::wrapper::Add;
    fn neg(self) -> crate::wrapper::Add {
        self.negate()
    }
}

// ===========================================================================
// Zdd — safe ZDD wrapper
// ===========================================================================

/// A ZDD (Zero-suppressed Decision Diagram) with automatic reference counting.
pub struct Zdd {
    mgr: ManagerRef,
    node: NodeId,
}

impl Zdd {
    /// Wrap a raw [`NodeId`] and increment its reference count.
    fn new(mgr: ManagerRef, node: NodeId) -> Self {
        mgr.borrow_mut().ref_node(node);
        Zdd { mgr, node }
    }

    /// The underlying [`NodeId`].
    pub fn node_id(&self) -> NodeId {
        self.node
    }

    /// Returns `true` if this is the empty family.
    pub fn is_empty(&self) -> bool {
        self.node.is_zero()
    }

    /// Returns `true` if this is the base family (contains only the empty set).
    pub fn is_base(&self) -> bool {
        self.node.is_one()
    }

    /// Set-family union: `self | other`.
    pub fn union(&self, other: &Zdd) -> Zdd {
        let node = self
            .mgr
            .borrow_mut()
            .zdd_union(self.node, other.node);
        Zdd::new(Rc::clone(&self.mgr), node)
    }

    /// Set-family intersection: `self & other`.
    pub fn intersect(&self, other: &Zdd) -> Zdd {
        let node = self
            .mgr
            .borrow_mut()
            .zdd_intersect(self.node, other.node);
        Zdd::new(Rc::clone(&self.mgr), node)
    }

    /// Set-family difference: `self \ other`.
    pub fn diff(&self, other: &Zdd) -> Zdd {
        let node = self
            .mgr
            .borrow_mut()
            .zdd_diff(self.node, other.node);
        Zdd::new(Rc::clone(&self.mgr), node)
    }

    /// Cross-product of two set families.
    pub fn product(&self, other: &Zdd) -> Zdd {
        let node = self
            .mgr
            .borrow_mut()
            .zdd_product(self.node, other.node);
        Zdd::new(Rc::clone(&self.mgr), node)
    }

    /// Count the number of sets in the family.
    pub fn count(&self) -> u64 {
        self.mgr.borrow().zdd_count(self.node)
    }
}

impl Clone for Zdd {
    fn clone(&self) -> Self {
        self.mgr.borrow_mut().ref_node(self.node);
        Zdd {
            mgr: Rc::clone(&self.mgr),
            node: self.node,
        }
    }
}

impl Drop for Zdd {
    fn drop(&mut self) {
        self.mgr.borrow_mut().deref_node(self.node);
    }
}

impl PartialEq for Zdd {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}

impl Eq for Zdd {}

impl fmt::Debug for Zdd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Zdd({:?})", self.node)
    }
}

impl ops::BitOr for &Zdd {
    type Output = Zdd;
    fn bitor(self, rhs: &Zdd) -> Zdd {
        self.union(rhs)
    }
}

impl ops::BitOr for Zdd {
    type Output = Zdd;
    fn bitor(self, rhs: Zdd) -> Zdd {
        self.union(&rhs)
    }
}

impl ops::BitAnd for &Zdd {
    type Output = Zdd;
    fn bitand(self, rhs: &Zdd) -> Zdd {
        self.intersect(rhs)
    }
}

impl ops::BitAnd for Zdd {
    type Output = Zdd;
    fn bitand(self, rhs: Zdd) -> Zdd {
        self.intersect(&rhs)
    }
}

impl ops::Sub for &Zdd {
    type Output = Zdd;
    fn sub(self, rhs: &Zdd) -> Zdd {
        self.diff(rhs)
    }
}

impl ops::Sub for Zdd {
    type Output = Zdd;
    fn sub(self, rhs: Zdd) -> Zdd {
        self.diff(&rhs)
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bdd_basic_ops() {
        let mgr = CuddManager::new();
        let x = mgr.bdd_var(0);
        let y = mgr.bdd_var(1);

        let f = x.and(&y);
        let g = x.or(&y);

        // x AND y is not ONE
        assert!(!f.is_one());
        // x OR y is not ZERO
        assert!(!g.is_zero());

        // f OR NOT(f) = ONE
        let nf = f.not();
        let taut = f.or(&nf);
        assert!(taut.is_one());
    }

    #[test]
    fn bdd_operator_overloads() {
        let mgr = CuddManager::new();
        let x = mgr.bdd_var(0);
        let y = mgr.bdd_var(1);

        // Test operator overloads on references
        let and_ref = &x & &y;
        let or_ref = &x | &y;
        let xor_ref = &x ^ &y;

        // AND via method should equal AND via operator
        let and_method = x.and(&y);
        assert_eq!(and_ref, and_method);

        // OR via method
        let or_method = x.or(&y);
        assert_eq!(or_ref, or_method);

        // XOR via method
        let xor_method = x.xor(&y);
        assert_eq!(xor_ref, xor_method);
    }

    #[test]
    fn bdd_clone_and_drop() {
        let mgr = CuddManager::new();
        let x = mgr.bdd_var(0);
        let x2 = x.clone();
        assert_eq!(x, x2);
        drop(x2);
        // x should still be valid
        assert!(!x.is_one());
    }

    #[test]
    fn bdd_compose() {
        let mgr = CuddManager::new();
        let x = mgr.bdd_var(0);
        let y = mgr.bdd_var(1);

        // Compose x[x := y] should give y
        let result = x.compose(&y, 0);
        assert_eq!(result, y);
    }

    #[test]
    fn bdd_count_minterm() {
        let mgr = CuddManager::new();
        let x = mgr.bdd_var(0);
        let y = mgr.bdd_var(1);
        let f = x.and(&y);
        // x AND y has 1 minterm over 2 variables
        assert!((f.count_minterm(2) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn bdd_support() {
        let mgr = CuddManager::new();
        let x = mgr.bdd_var(0);
        let y = mgr.bdd_var(1);
        let _z = mgr.bdd_var(2);
        let f = x.and(&y);
        let mut sup = f.support();
        sup.sort();
        assert_eq!(sup, vec![0, 1]);
    }

    #[test]
    fn bdd_exist_abstract() {
        let mgr = CuddManager::new();
        let x = mgr.bdd_var(0);
        let y = mgr.bdd_var(1);

        let f = x.and(&y);
        // Exist y . (x AND y) = x
        let cube_y = mgr.bdd_var(1);
        let result = f.exist_abstract(&cube_y);
        assert_eq!(result, x);
    }

    #[test]
    fn add_arithmetic() {
        let mgr = CuddManager::new();
        let a = mgr.add_const(3.0);
        let b = mgr.add_const(4.0);

        let sum = a.plus(&b);
        assert!((sum.value().unwrap() - 7.0).abs() < f64::EPSILON);

        let prod = a.times(&b);
        assert!((prod.value().unwrap() - 12.0).abs() < f64::EPSILON);

        let diff = a.minus(&b);
        assert!((diff.value().unwrap() - (-1.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn add_operator_overloads() {
        let mgr = CuddManager::new();
        let a = mgr.add_const(5.0);
        let b = mgr.add_const(2.0);

        let sum = &a + &b;
        assert!((sum.value().unwrap() - 7.0).abs() < f64::EPSILON);

        let diff = &a - &b;
        assert!((diff.value().unwrap() - 3.0).abs() < f64::EPSILON);

        let prod = &a * &b;
        assert!((prod.value().unwrap() - 10.0).abs() < f64::EPSILON);

        let quot = &a / &b;
        assert!((quot.value().unwrap() - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn zdd_basic_ops() {
        let mgr = CuddManager::new();
        let a = mgr.zdd_var(0);
        let b = mgr.zdd_var(1);

        // Union of {0} and {1}
        let u = a.union(&b);
        assert_eq!(u.count(), 2); // {{0}, {1}}

        // Intersection of {0} and {1} is empty
        let i = a.intersect(&b);
        assert!(i.is_empty());

        // Difference
        let d = u.diff(&a);
        assert_eq!(d, b);
    }

    #[test]
    fn zdd_operator_overloads() {
        let mgr = CuddManager::new();
        let a = mgr.zdd_var(0);
        let b = mgr.zdd_var(1);

        let union_op = &a | &b;
        let union_method = a.union(&b);
        assert_eq!(union_op, union_method);
    }

    #[test]
    fn cudd_manager_num_vars() {
        let mgr = CuddManager::new();
        assert_eq!(mgr.num_vars(), 0);
        let _x = mgr.bdd_var(0);
        assert_eq!(mgr.num_vars(), 1);
        let _y = mgr.bdd_var(3);
        assert_eq!(mgr.num_vars(), 4);
    }
}
