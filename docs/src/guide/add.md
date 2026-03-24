# ADD Operations Guide

This chapter covers all operations on Algebraic Decision Diagrams (ADDs). An ADD represents a function `f : {0,1}^n -> R` where terminal nodes hold real numbers (`f64` values) instead of just 0 and 1. ADDs are used for probability computations, cost functions, matrix operations, signal processing, and anywhere a real-valued function over Boolean inputs is needed.

All operations are methods on `Manager`. Unlike BDDs, ADDs do **not** use complemented edges -- every `NodeId` referencing an ADD node has its complement bit clear.

## Constants and Variables

### `add_const`

Creates or retrieves an ADD terminal node with a given value:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();

let two = mgr.add_const(2.0);
let pi  = mgr.add_const(std::f64::consts::PI);
let one = mgr.add_const(1.0); // same node as NodeId::ONE
```

### `add_zero`

Returns the ADD constant for 0.0. Note that ADD zero is a distinct terminal node, not the same as `NodeId::ZERO` (which is the complemented BDD one):

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let z = mgr.add_zero();
assert_eq!(mgr.add_value(z), Some(0.0));
```

### `add_value`

Retrieves the `f64` value of an ADD terminal node:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let c = mgr.add_const(3.14);
assert_eq!(mgr.add_value(c), Some(3.14));
```

Returns `None` if the node is not a terminal or is an invalid complemented ADD reference.

### `add_ith_var`

Returns the ADD projection function for variable `i`. The resulting ADD maps `x_i = 1` to 1.0 and `x_i = 0` to 0.0:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.add_ith_var(0);
// x maps (x0=1) -> 1.0 and (x0=0) -> 0.0
```

## ITE Operation

### `add_ite`

ADD if-then-else: for each minterm, returns `g` where `f > 0` and `h` where `f = 0`:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.add_ith_var(0);
let a = mgr.add_const(10.0);
let b = mgr.add_const(20.0);

// if x then 10.0 else 20.0
let result = mgr.add_ite(x, a, b);
```

## Binary Apply Operations

The `add_apply` function applies a binary operator element-wise to two ADDs. The `AddOp` enum provides the available operators:

| Operator | Semantics |
|---|---|
| `AddOp::Plus` | `a + b` |
| `AddOp::Times` | `a * b` |
| `AddOp::Minus` | `a - b` |
| `AddOp::Divide` | `a / b` (infinity if b = 0) |
| `AddOp::Minimum` | `min(a, b)` |
| `AddOp::Maximum` | `max(a, b)` |
| `AddOp::Or` | `1.0` if either is nonzero, else `0.0` |
| `AddOp::And` | `1.0` if both are nonzero, else `0.0` |
| `AddOp::Xor` | `1.0` if exactly one is nonzero, else `0.0` |
| `AddOp::Nand` | `0.0` if both are nonzero, else `1.0` |
| `AddOp::Nor` | `0.0` if either is nonzero, else `1.0` |
| `AddOp::Agree` | `a` if `a == b`, else infinity |

### `add_apply`

```rust
use lumindd::{Manager, AddOp};

let mut mgr = Manager::new();

let x = mgr.add_ith_var(0);
let two = mgr.add_const(2.0);

// Multiply the indicator of x by 2: maps x=1 -> 2.0, x=0 -> 0.0
let result = mgr.add_apply(AddOp::Times, x, two);
```

### Convenience shortcuts

For the most common operations, dedicated methods are provided:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let a = mgr.add_const(3.0);
let b = mgr.add_const(4.0);

let sum  = mgr.add_plus(a, b);   // 7.0
let prod = mgr.add_times(a, b);  // 12.0
let diff = mgr.add_minus(a, b);  // -1.0
let quot = mgr.add_divide(a, b); // 0.75
let lo   = mgr.add_min(a, b);    // 3.0
let hi   = mgr.add_max(a, b);    // 4.0
```

## Monadic Apply Operations

The `add_monadic_apply` function applies a unary operator to every terminal in an ADD:

| Operator | Semantics |
|---|---|
| `AddMonadicOp::Log` | natural logarithm |
| `AddMonadicOp::Negate` | `-x` |
| `AddMonadicOp::Complement` | `1 - x` |
| `AddMonadicOp::Abs` | absolute value |
| `AddMonadicOp::Floor` | floor |
| `AddMonadicOp::Ceil` | ceiling |

```rust
use lumindd::{Manager, AddMonadicOp};

let mut mgr = Manager::new();

let x = mgr.add_ith_var(0);
let three = mgr.add_const(3.0);
let f = mgr.add_plus(x, three); // x=1 -> 4.0, x=0 -> 3.0

let neg = mgr.add_monadic_apply(AddMonadicOp::Negate, f);
// x=1 -> -4.0, x=0 -> -3.0

let negated = mgr.add_negate(f);  // convenience shortcut
let logged  = mgr.add_log(f);     // convenience shortcut: ln(f)
```

## Abstraction

ADD abstraction computes a "summary" over quantified variables using an associative operator.

### `add_exist_abstract`

Sums the ADD over the quantified variables (existential = summation for ADDs):

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.add_ith_var(0);
let y = mgr.add_ith_var(1);

let f = mgr.add_times(x, y); // x AND y as ADD: 1.0 when both true

let cube_y = mgr.bdd_cube(&[1]);
let result = mgr.add_exist_abstract(f, cube_y);
// Sum over y: for x=1, sum is 0+1=1; for x=0, sum is 0+0=0
// So result equals the ADD for variable x
```

### `add_univ_abstract`

Multiplies the ADD over the quantified variables (universal = product):

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.add_ith_var(0);
let y = mgr.add_ith_var(1);

let f = mgr.add_plus(x, y);
let cube_y = mgr.bdd_cube(&[1]);

// Product over y: for x=1, product is 1*2=2; for x=0, product is 0*1=0
let result = mgr.add_univ_abstract(f, cube_y);
```

### `add_or_abstract`

Takes the maximum (OR) over quantified variables:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let f = mgr.add_ith_var(0);
let cube = mgr.bdd_cube(&[0]);

// OR over x: max of f(x=0) and f(x=1) = max(0, 1) = 1
let result = mgr.add_or_abstract(f, cube);
```

## Composition

### `add_compose`

Substitutes an ADD `g` for variable `var` in ADD `f`:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.add_ith_var(0);
let y = mgr.add_ith_var(1);
let three = mgr.add_const(3.0);

let f = mgr.add_plus(x, three); // x + 3

// Replace x (var 0) with y: result is y + 3
let result = mgr.add_compose(f, y, 0);
```

### `add_vector_compose`

Simultaneously substitutes a vector of functions for all variables. `compose_vec[i]` is the replacement for variable `i`.

### `add_permute`

Renames variables according to a permutation (analogous to `bdd_permute`).

## Matrix Operations

ADDs can represent matrices where row indices are encoded by one set of Boolean variables and column indices by another.

### `add_matrix_multiply`

Multiplies two ADD-encoded matrices. The `z_vars` parameter specifies the "summation" variables (shared between the columns of A and rows of B):

```rust
use lumindd::{Manager, AddOp};

let mut mgr = Manager::new();

// Create variables: x for rows of A, z for shared, y for cols of B
let _x0 = mgr.bdd_new_var(); // 0: row bit
let z0 = mgr.bdd_new_var();  // 1: shared bit
let _y0 = mgr.bdd_new_var(); // 2: col bit

// A simple 2x2 identity-like matrix as an ADD
let x = mgr.add_ith_var(0);
let z = mgr.add_ith_var(1);
let a = mgr.add_apply(AddOp::And, x, z);

let y = mgr.add_ith_var(2);
let b = mgr.add_apply(AddOp::And, z, y);

// Matrix multiply A * B, summing over z
let result = mgr.add_matrix_multiply(a, b, &[1]);
```

### `add_times_plus`

A fused multiply-add operation for matrix computations: computes the element-wise product and then sums over the specified variables. This is the same as `add_matrix_multiply` but with a more explicit name.

### `add_triangle`

Computes a triangular matrix operation: multiplies two ADD matrices and returns only the entries where `A[i][k] * B[k][j]` with `i <= j`.

### `add_outer_sum`

Computes the outer sum of two vectors: given ADDs `a(x)` and `b(y)`, produces `a(x) + b(y)` as a matrix ADD.

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.add_ith_var(0);
let y = mgr.add_ith_var(1);

let outer = mgr.add_outer_sum(x, y);
// outer(0,0)=0, outer(0,1)=1, outer(1,0)=1, outer(1,1)=2
```

## Walsh Matrix and Residue

### `add_walsh`

Constructs a Walsh matrix (Hadamard-like) as an ADD. The Walsh matrix has entries +1 and -1:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let _x0 = mgr.bdd_new_var();
let _x1 = mgr.bdd_new_var();
let _y0 = mgr.bdd_new_var();
let _y1 = mgr.bdd_new_var();

let walsh = mgr.add_walsh(&[0, 1], &[2, 3]);
// 4x4 Walsh matrix encoded as an ADD
```

### `add_hadamard`

Constructs a Hadamard matrix of order `2^num_vars`.

### `add_residue`

Builds an ADD representing `(sum of x_vars) mod modulus`:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
for _ in 0..4 { mgr.bdd_new_var(); }

// Parity function: sum of 4 bits mod 2
let parity = mgr.add_residue(&[0, 1, 2, 3], 2);
```

### `add_xor_indicator`

Builds an ADD that is 1.0 where `x_i XOR y_i` for corresponding variable pairs, and 0.0 otherwise.

## BDD Conversion

### `bdd_to_add`

Converts a BDD to an ADD where the ONE terminal becomes 1.0 and the ZERO terminal becomes 0.0:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();

let bdd_f = mgr.bdd_and(x, y);
let add_f = mgr.bdd_to_add(bdd_f);

assert_eq!(mgr.add_value(mgr.add_const(1.0)), Some(1.0));
```

### `add_bdd_pattern`

Converts an ADD to a BDD where all nonzero terminals become 1 (true) and zero terminals become 0 (false):

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let f = mgr.add_const(42.0);
let bdd = mgr.add_bdd_pattern(f);
assert!(mgr.bdd_is_tautology(bdd)); // 42.0 != 0, so it's true everywhere
```

### `add_bdd_threshold`

Converts an ADD to a BDD where terminals >= threshold become 1 and terminals < threshold become 0:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.add_ith_var(0);
let five = mgr.add_const(5.0);
let f = mgr.add_plus(x, five); // x=1 -> 6.0, x=0 -> 5.0

let bdd = mgr.add_bdd_threshold(f, 5.5);
// Only x=1 (value 6.0) exceeds 5.5
```

### `add_bdd_strict_threshold`

Like `add_bdd_threshold` but with strict comparison: terminals > threshold become 1.

### `add_bdd_interval`

Converts an ADD to a BDD where terminals in `[lower, upper]` become 1.

## Terminal Queries

### `add_find_min`, `add_find_max`

Find the minimum or maximum terminal value in an ADD:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.add_ith_var(0);
let three = mgr.add_const(3.0);
let seven = mgr.add_const(7.0);

let f = mgr.add_ite(x, seven, three);

assert_eq!(mgr.add_find_min(f), 3.0);
assert_eq!(mgr.add_find_max(f), 7.0);
```

### `add_scalar_inverse`

Computes the element-wise multiplicative inverse `1/x` of every terminal:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let four = mgr.add_const(4.0);
let inv = mgr.add_scalar_inverse(four);
assert_eq!(mgr.add_value(inv), Some(0.25));
```

### `add_round_off`

Rounds every terminal value to a specified number of decimal places.

### `add_count_paths_to_nonzero`

Counts the number of paths from the root to nonzero terminals.

## Comparison

### `add_equal_sup_norm`

Tests whether two ADDs are equal within a tolerance (supremum norm):

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let a = mgr.add_const(1.0);
let b = mgr.add_const(1.001);

assert!(mgr.add_equal_sup_norm(a, b, 0.01));   // within tolerance
assert!(!mgr.add_equal_sup_norm(a, b, 0.0001)); // not within tolerance
```

### `add_agreement`

Computes the agreement of two ADDs: returns `f` where `f == g` and infinity elsewhere.

## Sparse Matrix I/O (Harwell-Boeing)

ADDs can be converted to and from sparse matrices in Harwell-Boeing format for interoperability with numerical libraries.

### `HarwellMatrix`

The `HarwellMatrix` type represents a sparse matrix in compressed column format:

```rust
use lumindd::HarwellMatrix;

// Create a 4x4 sparse matrix
let mut m = HarwellMatrix::new(4, 4);
```

### Reading and writing

```rust
use lumindd::HarwellMatrix;
use std::io::BufReader;

// Read from a Harwell-Boeing file
let data = b"..."; // HB format data
let mut reader = BufReader::new(&data[..]);
// let matrix = HarwellMatrix::from_reader(&mut reader).unwrap();

// Write to HB format
// matrix.to_writer(&mut std::io::stdout()).unwrap();
```

### ADD <-> Sparse matrix conversion

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
// ... build an ADD representing a matrix ...

// Convert ADD to sparse matrix
// let sparse = mgr.add_to_sparse_matrix(add_matrix, &row_vars, &col_vars);

// Convert sparse matrix back to ADD
// let add = mgr.add_from_sparse_matrix(&sparse, &row_vars, &col_vars);
```

### `add_from_sparse_matrix`

Converts a `HarwellMatrix` to an ADD, using row variables and column variables to encode matrix indices.

### `add_to_sparse_matrix`

Converts an ADD-encoded matrix back to a `HarwellMatrix` for export or numerical computation.
