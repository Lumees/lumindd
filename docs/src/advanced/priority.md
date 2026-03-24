# Priority and Comparison Functions

lumindd provides a set of BDD construction methods for building decision diagrams that encode arithmetic comparisons, intervals, and distance functions over bit-vector variables. These are essential building blocks for hardware verification, constraint solving, and optimization problems.

## Variable Conventions

All methods in this module use the MSB-first convention: `vars[0]` is the most significant bit and `vars[n-1]` is the least significant bit. Variables are specified by their variable index (a `u16`).

## Inequality: x > y

`bdd_inequality` builds a BDD that is true exactly when the unsigned integer `x` is strictly greater than the unsigned integer `y`.

```rust
let mut mgr = Manager::new();
for _ in 0..8 { mgr.bdd_new_var(); }

let x_vars: Vec<u16> = (0..4).collect();  // 4-bit x
let y_vars: Vec<u16> = (4..8).collect();  // 4-bit y

let x_gt_y = mgr.bdd_inequality(4, &x_vars, &y_vars);
```

The algorithm processes bits from MSB to LSB, building the comparison incrementally: at each bit position, `x > y` if either `x_i > y_i` at this position, or `x_i == y_i` and the comparison holds for the remaining lower bits.

## Interval: lower <= x <= upper

`bdd_interval` builds a BDD that is true when the unsigned integer encoded by `x_vars` falls within the range `[lower, upper]`.

```rust
let x_vars: Vec<u16> = (0..8).collect();  // 8-bit x

// x is between 10 and 200 (inclusive)
let in_range = mgr.bdd_interval(&x_vars, 10, 200);
```

Internally, this constructs `x >= lower AND x <= upper` using dedicated helper methods `bdd_ge_const` and `bdd_le_const`.

**Special cases:**
- `lower > upper` returns ZERO (empty interval)
- Zero-width variables with `lower == 0` returns ONE

## Disequality: x != y

`bdd_disequality` builds a BDD that is true when the bit-vectors `x` and `y` differ in at least one position.

```rust
let x_ne_y = mgr.bdd_disequality(4, &x_vars, &y_vars);
```

This is implemented as the OR of XOR at each bit position: `(x_0 XOR y_0) OR (x_1 XOR y_1) OR ...`

## Equality and Greater-Than (bdd_xeqy, bdd_xgty)

These are available in the `bdd_extra` module:

- `bdd_xeqy(x_vars, y_vars)` -- builds a BDD for `x == y` (bitwise equality)
- `bdd_xgty(x_vars, y_vars)` -- builds a BDD for `x > y` (unsigned greater-than)

```rust
let x_eq_y = mgr.bdd_xeqy(&x_vars, &y_vars);
let x_gt_y = mgr.bdd_xgty(&x_vars, &y_vars);
```

## Hamming Distance

### BDD Hamming Ball

`bdd_hamming_distance` expands a BDD `f` to include all assignments within Hamming distance `dist` of any satisfying assignment.

```rust
let x_vars: Vec<u16> = (0..4).collect();

// All assignments within Hamming distance 2 of solutions to f
let ball = mgr.bdd_hamming_distance(f, &x_vars, 2);
```

For each distance step, the method considers flipping each variable in `x_vars` and taking the union of the original and flipped results.

### ADD Hamming Distance

`add_hamming` builds an ADD (Algebraic Decision Diagram) that computes the Hamming distance between two variable vectors as an integer.

```rust
let x_vars: Vec<u16> = (0..4).collect();
let y_vars: Vec<u16> = (4..8).collect();

// ADD with integer values 0..4 representing Hamming distance
let dist_add = mgr.add_hamming(&x_vars, &y_vars);
```

The result is an ADD where each terminal value is the number of positions where `x` and `y` differ.

## Distance Comparisons

### d(x,y) > d(x,z)

`bdd_dxygtdxz` builds a BDD that is true when the Hamming distance between `x` and `y` exceeds the Hamming distance between `x` and `z`.

```rust
let x_vars: Vec<u16> = (0..4).collect();
let y_vars: Vec<u16> = (4..8).collect();
let z_vars: Vec<u16> = (8..12).collect();

let result = mgr.bdd_dxygtdxz(&x_vars, &y_vars, &z_vars);
```

**Implementation:** Computes `add_hamming(x, y)` and `add_hamming(x, z)`, subtracts them as ADDs, and converts the result to a BDD by thresholding at zero.

### d(x,y) > d(y,z)

`bdd_dxygtdyz` is similar but compares `d(x,y)` with `d(y,z)`.

```rust
let result = mgr.bdd_dxygtdyz(&x_vars, &y_vars, &z_vars);
```

## Applications

### Hardware Verification

These functions are the building blocks for verifying arithmetic circuits:

```rust
// Verify a 4-bit comparator circuit
let x_vars: Vec<u16> = (0..4).collect();
let y_vars: Vec<u16> = (4..8).collect();

// Build the specification
let spec = mgr.bdd_inequality(4, &x_vars, &y_vars);

// Build the implementation BDD (from circuit description)
let impl_bdd = /* ... circuit BDD ... */;

// Verify equivalence
let diff = mgr.bdd_xor(spec, impl_bdd);
assert!(diff.is_zero(), "Comparator implementation is incorrect");
```

### Constraint Solving

Build complex constraints compositionally:

```rust
// x is in [10, 20] AND y is in [5, 15] AND x > y
let c1 = mgr.bdd_interval(&x_vars, 10, 20);
let c2 = mgr.bdd_interval(&y_vars, 5, 15);
let c3 = mgr.bdd_inequality(4, &x_vars, &y_vars);

let solution_set = mgr.bdd_and(c1, mgr.bdd_and(c2, c3));
```

### Error-Correcting Codes

Use Hamming distance functions to reason about code distances:

```rust
// All codewords within distance 2 of any valid codeword
let valid_codewords = /* BDD of valid codewords */;
let decodable = mgr.bdd_hamming_distance(valid_codewords, &bit_vars, 2);
```

## API Reference

| Method | Description |
|---|---|
| `bdd_inequality(n, x_vars, y_vars)` | BDD for x > y (n-bit unsigned) |
| `bdd_interval(x_vars, lower, upper)` | BDD for lower <= x <= upper |
| `bdd_disequality(n, x_vars, y_vars)` | BDD for x != y |
| `bdd_xeqy(x_vars, y_vars)` | BDD for x == y |
| `bdd_xgty(x_vars, y_vars)` | BDD for x > y |
| `bdd_hamming_distance(f, x_vars, dist)` | Expand f by Hamming ball of radius dist |
| `add_hamming(x_vars, y_vars)` | ADD computing Hamming distance |
| `bdd_dxygtdxz(x, y, z)` | BDD for d(x,y) > d(x,z) |
| `bdd_dxygtdyz(x, y, z)` | BDD for d(x,y) > d(y,z) |
