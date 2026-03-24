# Safe RAII Wrapper Types

lumindd provides safe wrapper types that automatically manage reference counting through Rust's RAII (Resource Acquisition Is Initialization) pattern. These wrappers eliminate the need to manually call `ref_node` and `deref_node`, preventing memory leaks and use-after-free bugs.

## Overview

The wrapper module provides four types:

| Type | Purpose |
|---|---|
| `CuddManager` | Managed context wrapping a `Manager` in `Rc<RefCell<Manager>>` |
| `Bdd` | BDD node with automatic reference counting |
| `Add` | ADD node with automatic reference counting |
| `Zdd` | ZDD node with automatic reference counting |

## CuddManager

`CuddManager` wraps a `Manager` in shared ownership (`Rc<RefCell<Manager>>`), allowing all BDD/ADD/ZDD objects to share access to the same manager.

```rust
use lumindd::wrapper::CuddManager;

let mgr = CuddManager::new();

// Create variables
let x = mgr.bdd_var(0);
let y = mgr.bdd_var(1);

// Constants
let one = mgr.bdd_one();
let zero = mgr.bdd_zero();

// ADD constants
let pi = mgr.add_const(3.14159);

// ZDD variables
let z = mgr.zdd_var(0);

// Query
println!("Variables: {}", mgr.num_vars());
```

You can also wrap an existing `Manager`:

```rust
use lumindd::Manager;
use lumindd::wrapper::CuddManager;

let raw_mgr = Manager::new();
let mgr = CuddManager::from_manager(raw_mgr);
```

## Bdd

The `Bdd` type wraps a `NodeId` and automatically manages its reference count:

- On creation: `ref_node` is called
- On clone: `ref_node` is called for the copy
- On drop: `deref_node` is called

### Operations

```rust
let mgr = CuddManager::new();
let x = mgr.bdd_var(0);
let y = mgr.bdd_var(1);

// Named methods
let f = x.and(&y);         // AND
let g = x.or(&y);          // OR
let h = x.xor(&y);         // XOR
let nx = x.not();           // NOT

// If-then-else
let ite = x.ite(&y, &nx);  // if x then y else !x

// Quantification
let cube = mgr.bdd_var(1);
let exists = f.exist_abstract(&cube);  // exists y. (x AND y) = x

// Composition
let composed = f.compose(&y, 0);  // substitute y for variable 0 in f

// Counting
let count = f.count_minterm(2);  // number of satisfying assignments

// Support
let support = f.support();  // variable indices in the BDD
```

### Operator Overloading

`Bdd` supports Rust's standard operators for both owned and borrowed values:

```rust
let f = &x & &y;   // AND
let g = &x | &y;   // OR
let h = &x ^ &y;   // XOR
let n = !&x;        // NOT

// Works on owned values too
let f2 = x.clone() & y.clone();  // consumes the clones
```

### Comparison

```rust
assert_eq!(x.and(&y), x.and(&y));  // structural equality
assert!(x.and(&y).is_one() == false);
```

## Add

The `Add` type wraps an ADD node with the same automatic reference counting.

### Operations

```rust
let mgr = CuddManager::new();
let a = mgr.add_const(3.0);
let b = mgr.add_const(4.0);

// Named methods
let sum = a.plus(&b);       // 7.0
let prod = a.times(&b);     // 12.0
let diff = a.minus(&b);     // -1.0
let quot = a.divide(&b);    // 0.75
let lo = a.minimum(&b);     // 3.0
let hi = a.maximum(&b);     // 4.0
let neg = a.negate();        // -3.0

// Get terminal value
let val: Option<f64> = sum.value();
assert_eq!(val, Some(7.0));
```

### Operator Overloading

```rust
let sum = &a + &b;    // addition
let diff = &a - &b;   // subtraction
let prod = &a * &b;   // multiplication
let quot = &a / &b;   // division
let neg = -&a;         // negation
```

### General Apply

For operations beyond the built-in set:

```rust
use lumindd::add::{AddOp, AddMonadicOp};

let result = a.apply(AddOp::Maximum, &b);
let floored = a.monadic_apply(AddMonadicOp::Floor);
```

## Zdd

The `Zdd` type wraps a ZDD node for set-family operations.

### Operations

```rust
let mgr = CuddManager::new();
let a = mgr.zdd_var(0);  // family {{0}}
let b = mgr.zdd_var(1);  // family {{1}}

let u = a.union(&b);       // {{0}, {1}}
let i = a.intersect(&b);   // {} (empty)
let d = u.diff(&a);        // {{1}}
let p = a.product(&b);     // cross-product

let n = u.count();          // 2

// Special constants
let empty = mgr.zdd_empty();  // empty family
let base = mgr.zdd_base();    // family containing only the empty set
```

### Operator Overloading

```rust
let u = &a | &b;   // union
let i = &a & &b;   // intersection
let d = &a - &b;   // difference
```

## When to Use Wrappers vs. Raw Manager

### Use Wrappers When

- You want safe, ergonomic code with no manual reference counting
- You are building an application where BDD objects are passed around and stored in data structures
- You want operator overloading for concise expressions
- Memory safety is a priority

### Use Raw Manager When

- You need maximum performance and want to avoid `Rc<RefCell<...>>` overhead
- You are writing library internals that manage references explicitly
- You need access to methods not exposed through the wrapper API
- You are doing batch operations where reference counting overhead would be noticeable

### Example: Raw vs. Wrapper

**Raw Manager:**
```rust
let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();
let f = mgr.bdd_and(x, y);
mgr.ref_node(f);

// ... use f ...

mgr.deref_node(f);  // manual cleanup
```

**Wrapper:**
```rust
let mgr = CuddManager::new();
let x = mgr.bdd_var(0);
let y = mgr.bdd_var(1);
let f = x.and(&y);

// ... use f ...
// cleanup happens automatically when f is dropped
```

## Thread Safety

The wrapper types use `Rc<RefCell<Manager>>` for shared ownership, which means they are **not** `Send` or `Sync`. They are designed for single-threaded use. If you need multi-threaded access, use the raw `Manager` API with your own synchronization.

## API Reference

### CuddManager

| Method | Description |
|---|---|
| `new()` | Create a new managed context |
| `from_manager(mgr)` | Wrap an existing Manager |
| `bdd_var(i)` | Get/create the i-th BDD variable |
| `bdd_one()` / `bdd_zero()` | BDD constants |
| `add_const(val)` | Create an ADD constant |
| `zdd_var(i)` | Get/create the i-th ZDD variable |
| `zdd_empty()` / `zdd_base()` | ZDD constants |
| `num_vars()` | Number of BDD/ADD variables |

### Bdd

| Method | Description |
|---|---|
| `and`, `or`, `xor`, `not` | Boolean operations |
| `ite(then, else)` | If-then-else |
| `exist_abstract(cube)` | Existential quantification |
| `compose(g, var)` | Variable substitution |
| `count_minterm(n)` | Count satisfying assignments |
| `support()` | Support variable set |
| `node_id()` | Underlying NodeId |
| `is_one()`, `is_zero()` | Constant tests |

### Add

| Method | Description |
|---|---|
| `plus`, `minus`, `times`, `divide` | Arithmetic |
| `minimum`, `maximum` | Element-wise min/max |
| `negate` | Negation |
| `apply(op, other)` | General binary apply |
| `monadic_apply(op)` | General unary apply |
| `value()` | Terminal value (Option) |
| `node_id()` | Underlying NodeId |

### Zdd

| Method | Description |
|---|---|
| `union`, `intersect`, `diff` | Set-family operations |
| `product` | Cross-product |
| `count()` | Number of sets in family |
| `is_empty()`, `is_base()` | Special family tests |
| `node_id()` | Underlying NodeId |
