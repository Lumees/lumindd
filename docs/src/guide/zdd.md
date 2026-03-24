# ZDD Operations Guide

This chapter covers all operations on Zero-suppressed Decision Diagrams (ZDDs). A ZDD represents a **family of sets** -- a collection of subsets of a finite universe. ZDDs are the right tool when you need to enumerate, count, or manipulate combinatorial objects such as independent sets, paths, covers, or solutions to constraint problems.

All operations are methods on `Manager`.

## What Are ZDDs?

A ZDD is structurally similar to a BDD but uses a different reduction rule. In a standard BDD, a node is eliminated when both children are the same (redundant test). In a ZDD, a node is eliminated when its **then-child is zero** -- meaning the variable does not appear in any set in the family.

This "zero suppression" makes ZDDs naturally compact for **sparse** set families where most elements are absent from most sets.

### Terminals

| Terminal | Meaning |
|---|---|
| **1** (`NodeId::ONE`) | The family containing exactly the empty set: `{ {} }` |
| **0** (`NodeId::ZERO`) | The empty family: `{}` (no sets at all) |

### Example

Consider a universe `{a, b, c}` and the family `{ {a}, {a,c}, {b} }`. As a ZDD:

```text
          a
         / \
        b   c
       / \ / \
      0   1 1  0
```

Reading from the root:
- **a=1 branch**: the `c` node encodes `{a}` (c=0) and `{a,c}` (c=1).
- **a=0 branch**: the `b` node encodes `{b}` (b=1) and the empty consideration (b=0 leads to 0, meaning no set without `a` or `b`).

### When to use ZDDs instead of BDDs

Use ZDDs when:
- You are representing **families of sets** (collections of subsets).
- The universe is large but individual sets are small (sparse families).
- You need set-family operations like union, intersection, cross product, or division.
- You are computing irredundant covers (ISOP) or enumerating combinatorial structures.

Use BDDs when:
- You are representing a single Boolean function.
- The function is "dense" (many variables matter in most branches).
- You need operations like quantification, composition, or restrict that are BDD-native.

## Variables

### `zdd_new_var`

Creates a new ZDD variable at the next available ZDD index:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let v0 = mgr.zdd_new_var(); // ZDD variable index 0
let v1 = mgr.zdd_new_var(); // ZDD variable index 1

println!("ZDD variables: {}", mgr.num_zdd_vars()); // 2
```

Note: ZDD variables are managed separately from BDD/ADD variables. They have their own permutation arrays and unique tables.

### `zdd_ith_var`

Returns the ZDD representing the singleton family `{ {v_i} }` -- the family containing exactly the set with one element. Creates the variable if it does not yet exist:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let v3 = mgr.zdd_ith_var(3); // creates ZDD variables 0..=3

// v3 represents the family { {3} }
let count = mgr.zdd_count(v3);
assert_eq!(count, 1); // one set in the family
```

### `zdd_vars_from_bdd_vars`

Creates ZDD variables corresponding to existing BDD variables. The `multiplicity` parameter specifies how many ZDD variables to create per BDD variable (useful for encoding multi-valued attributes).

## Set Operations

### `zdd_union`

Computes the union of two set families (all sets that appear in either family):

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let a = mgr.zdd_ith_var(0); // { {0} }
let b = mgr.zdd_ith_var(1); // { {1} }

let ab = mgr.zdd_union(a, b); // { {0}, {1} }
assert_eq!(mgr.zdd_count(ab), 2);
```

### `zdd_intersect`

Computes the intersection of two set families (sets present in both):

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let a = mgr.zdd_ith_var(0);
let b = mgr.zdd_ith_var(1);

let ab = mgr.zdd_union(a, b);
let result = mgr.zdd_intersect(ab, a); // { {0} } -- only {0} is in both
assert_eq!(mgr.zdd_count(result), 1);
```

### `zdd_diff`

Computes the set difference: all sets in `P` that are not in `Q`:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let a = mgr.zdd_ith_var(0); // { {0} }
let b = mgr.zdd_ith_var(1); // { {1} }
let ab = mgr.zdd_union(a, b); // { {0}, {1} }

let result = mgr.zdd_diff(ab, a); // { {1} }
assert_eq!(mgr.zdd_count(result), 1);
```

### `zdd_product`

Computes the cross product (set-theoretic product) of two families. For each pair of sets (one from each family), their union is included in the result:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let a = mgr.zdd_ith_var(0); // { {0} }
let b = mgr.zdd_ith_var(1); // { {1} }

// Product: { {0} } x { {1} } = { {0, 1} }
let product = mgr.zdd_product(a, b);
assert_eq!(mgr.zdd_count(product), 1);
```

### `zdd_weak_div`

Weak division of families: `F / G` returns the family of sets `s` such that for **some** set `g` in `G`, `s union g` is in `F`.

### `zdd_strong_div`

Strong division: `F / G` returns the family of sets `s` such that for **every** set `g` in `G`, `s union g` is in `F`.

### `zdd_unate_product`

Unate product of two covers. Used in logic synthesis for combining unate covers.

### `zdd_dot_product`

Dot product of two ZDD covers: computes the intersection of all pairwise products.

## Subset and Change Operations

### `zdd_change`

Toggles a variable in every set of the family. If `var` is present in a set, it is removed; if absent, it is added:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let a = mgr.zdd_ith_var(0); // { {0} }

// Toggle variable 1: { {0} } -> { {0, 1} }
let result = mgr.zdd_change(a, 1);
assert_eq!(mgr.zdd_count(result), 1);
```

### `zdd_subset1`

Returns the sub-family of sets that contain a given variable (with the variable removed from each set):

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let a = mgr.zdd_ith_var(0); // { {0} }
let b = mgr.zdd_ith_var(1); // { {1} }
let ab = mgr.zdd_union(a, b); // { {0}, {1} }

// Sets containing variable 0, with variable 0 removed: { {} }
let result = mgr.zdd_subset1(ab, 0);
assert_eq!(mgr.zdd_count(result), 1); // just the empty set
```

### `zdd_subset0`

Returns the sub-family of sets that do **not** contain a given variable:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let a = mgr.zdd_ith_var(0);
let b = mgr.zdd_ith_var(1);
let ab = mgr.zdd_union(a, b); // { {0}, {1} }

// Sets not containing variable 0: { {1} }
let result = mgr.zdd_subset0(ab, 0);
assert_eq!(mgr.zdd_count(result), 1);
```

## Complement and Universe

### `zdd_complement`

Computes the complement of a set family with respect to a universe of `num_vars` elements. The result contains all subsets of `{0, ..., num_vars-1}` that are **not** in the original family:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let empty_fam = mgr.zdd_ith_var(0); // { {0} }

// Complement with 2 variables: all 4 subsets minus { {0} }
let comp = mgr.zdd_complement(empty_fam, 2);
assert_eq!(mgr.zdd_count(comp), 3); // { {}, {1}, {0,1} }
```

### `zdd_universe`

Builds the ZDD representing the powerset of `{0, ..., num_vars-1}` (all possible subsets):

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
// Make sure ZDD variables exist
for _ in 0..3 { mgr.zdd_new_var(); }

let univ = mgr.zdd_universe(3);
assert_eq!(mgr.zdd_count(univ), 8); // 2^3 = 8 subsets
```

## ITE

### `zdd_ite`

ZDD if-then-else: `ITE(f, g, h)` returns sets that are in `g` if they are in `f`, and in `h` otherwise. This is the universal ZDD operation.

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let a = mgr.zdd_ith_var(0);
let b = mgr.zdd_ith_var(1);

let result = mgr.zdd_ite(a, b, a);
```

## ISOP (Irredundant Sum of Products)

### `zdd_isop`

Computes an irredundant sum of products (ISOP) for a Boolean function specified by its lower and upper bounds (both given as BDDs). Returns both the ISOP as a BDD and the corresponding ZDD cover:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();
// Also create ZDD variables
mgr.zdd_new_var();
mgr.zdd_new_var();

let f = mgr.bdd_or(x, y);

// ISOP of f (lower = upper = f for exact cover)
let (bdd_result, zdd_cover) = mgr.zdd_isop(f, f);
assert_eq!(bdd_result, f);
```

### `zdd_make_from_bdd_cover`

Converts a BDD representing a sum-of-products cover into a ZDD set family.

## BDD Conversion

### `zdd_from_bdd`

Converts a BDD to a ZDD. The resulting ZDD represents the set of minterms (complete assignments) that satisfy the BDD:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();
mgr.zdd_new_var();
mgr.zdd_new_var();

let bdd_f = mgr.bdd_and(x, y); // x AND y
let zdd_f = mgr.zdd_from_bdd(bdd_f);

assert_eq!(mgr.zdd_count(zdd_f), 1); // one minterm: {x, y}
```

### `zdd_to_bdd`

Converts a ZDD back to a BDD. The resulting BDD is true for exactly the minterms encoded by the ZDD:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();
mgr.zdd_new_var();
mgr.zdd_new_var();

let bdd_f = mgr.bdd_and(x, y);
let zdd_f = mgr.zdd_from_bdd(bdd_f);
let bdd_back = mgr.zdd_to_bdd(zdd_f);

assert_eq!(bdd_f, bdd_back);
```

## Counting

### `zdd_count`

Counts the number of sets in the family:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let a = mgr.zdd_ith_var(0);
let b = mgr.zdd_ith_var(1);
let c = mgr.zdd_ith_var(2);

// { {0}, {1}, {2} }
let family = mgr.zdd_union(mgr.zdd_union(a, b), c);
assert_eq!(mgr.zdd_count(family), 3);
```

### `zdd_count_double`

Counts the number of sets as an `f64`, which can handle larger counts without overflow (at the cost of precision for very large values).

### `zdd_count_minterm`

Counts the total number of minterms encoded by the ZDD, given a total number of variables. Each set in the family contributes `2^(num_vars - |set|)` minterms (the don't-care variables can take any value).

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let a = mgr.zdd_ith_var(0);

// { {0} } with 3 total variables: 2^(3-1) = 4 minterms
let count = mgr.zdd_count_minterm(a, 3);
assert_eq!(count, 4.0);
```

## Utilities

### `zdd_support`

Returns the sorted list of ZDD variable indices that appear in the family:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let a = mgr.zdd_ith_var(0);
let c = mgr.zdd_ith_var(2);

let family = mgr.zdd_union(a, c);
let support = mgr.zdd_support(family);
assert_eq!(support, vec![0, 2]);
```

### `zdd_dag_size`

Returns the number of nodes in the ZDD DAG:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let a = mgr.zdd_ith_var(0);
let b = mgr.zdd_ith_var(1);
let family = mgr.zdd_product(a, b);

println!("ZDD DAG size: {}", mgr.zdd_dag_size(family));
```

### `zdd_max_cardinality`

Returns the size of the largest set in the family:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let a = mgr.zdd_ith_var(0);
let b = mgr.zdd_ith_var(1);

let singleton = a;                    // { {0} } -- max cardinality 1
let pair = mgr.zdd_product(a, b);    // { {0, 1} } -- max cardinality 2
let family = mgr.zdd_union(singleton, pair);

assert_eq!(mgr.zdd_max_cardinality(family), 2);
```

### `zdd_min_cardinality`

Returns the size of the smallest set in the family.

### `zdd_print_cover`

Prints the ZDD cover to stdout in a human-readable format showing each set.

### `zdd_print_minterm`

Prints all minterms of a ZDD to stdout.

## ZDD-Specific Reordering

ZDD variables have their own ordering that is independent of BDD/ADD variable ordering. lumindd provides reordering operations specific to ZDDs.

### `zdd_reduce_heap`

Triggers ZDD variable reordering using a `ReorderingMethod`:

```rust
use lumindd::{Manager, ReorderingMethod};

let mut mgr = Manager::new();
// ... build ZDD structures ...

mgr.zdd_reduce_heap(ReorderingMethod::Sift);
```

### `zdd_shuffle_heap`

Applies an explicit permutation to ZDD variable levels:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
for _ in 0..4 { mgr.zdd_new_var(); }

// Reverse the variable order
mgr.zdd_shuffle_heap(&[3, 2, 1, 0]);
```

### `zdd_sift_reorder`

Directly invokes the sifting algorithm on ZDD variables, with optional convergence (repeated sifting until no improvement):

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
// ... build ZDDs ...

mgr.zdd_sift_reorder(true); // sift with convergence
```

## Complete Example: Enumerating Graph Independent Sets

Here is an example that uses ZDDs to represent and count independent sets of a small graph:

```rust
use lumindd::Manager;

fn main() {
    let mut mgr = Manager::new();

    // Graph: triangle on 3 vertices (0-1, 1-2, 0-2)
    // Create ZDD variables for each vertex
    let v0 = mgr.zdd_ith_var(0);
    let v1 = mgr.zdd_ith_var(1);
    let v2 = mgr.zdd_ith_var(2);

    // Start with all subsets (universe)
    let all = mgr.zdd_universe(3); // 8 subsets

    // Remove sets that contain both endpoints of each edge
    // Edge 0-1: remove sets containing both 0 and 1
    let e01 = mgr.zdd_product(v0, v1); // { {0,1} }
    // Expand e01 to all supersets containing {0,1}
    let bad01 = mgr.zdd_product(e01, mgr.zdd_union(v2, mgr.zdd_base()));
    let remaining = mgr.zdd_diff(all, bad01);

    // Edge 1-2: remove sets containing both 1 and 2
    let e12 = mgr.zdd_product(v1, v2);
    let bad12 = mgr.zdd_product(e12, mgr.zdd_union(v0, mgr.zdd_base()));
    let remaining = mgr.zdd_diff(remaining, bad12);

    // Edge 0-2: remove sets containing both 0 and 2
    let e02 = mgr.zdd_product(v0, v2);
    let bad02 = mgr.zdd_product(e02, mgr.zdd_union(v1, mgr.zdd_base()));
    let independent_sets = mgr.zdd_diff(remaining, bad02);

    // For a triangle: independent sets are {}, {0}, {1}, {2} = 4 sets
    println!("Independent sets: {}", mgr.zdd_count(independent_sets));
    assert_eq!(mgr.zdd_count(independent_sets), 4);

    mgr.zdd_print_cover(independent_sets);
}
```
