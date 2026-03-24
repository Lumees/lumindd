# BDD Operations Guide

This chapter is a comprehensive reference for all BDD (Binary Decision Diagram) operations available in lumindd. BDDs represent Boolean functions `f : {0,1}^n -> {0,1}` and are the most commonly used diagram type.

All operations are methods on `Manager`.

## Variable Creation

### `bdd_new_var`

Creates a new BDD variable at the next available index and level.

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x0 = mgr.bdd_new_var(); // variable index 0, level 0
let x1 = mgr.bdd_new_var(); // variable index 1, level 1
```

### `bdd_ith_var`

Returns the projection function for variable `i`. If variable `i` does not exist yet, all variables up to `i` are created.

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x5 = mgr.bdd_ith_var(5); // creates variables 0..=5
assert_eq!(mgr.num_vars(), 6);
```

### `bdd_new_var_at_level`

Creates a new variable and inserts it at a specific level in the current ordering, shifting existing variables down.

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x0 = mgr.bdd_new_var(); // level 0
let x1 = mgr.bdd_new_var(); // level 1

// Insert a new variable at level 1, pushing x1 down to level 2
let x_mid = mgr.bdd_new_var_at_level(1);
assert_eq!(mgr.read_perm(mgr.read_var_index(x_mid)), 1);
```

## Boolean Operations

All Boolean operations are derived from the ITE (if-then-else) kernel or implemented with dedicated algorithms for better caching.

### `bdd_ite` -- If-Then-Else

The universal BDD operation: `ITE(f, g, h)` returns `g` where `f` is true and `h` where `f` is false.

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();
let z = mgr.bdd_new_var();

// Multiplexer: if x then y else z
let mux = mgr.bdd_ite(x, y, z);
assert!(mgr.bdd_eval(mux, &[true, true, false]));   // x=1 -> y=1
assert!(!mgr.bdd_eval(mux, &[false, true, false]));  // x=0 -> z=0
```

Every other Boolean operation can be expressed as ITE:

| Operation | ITE form |
|---|---|
| AND(f, g) | ITE(f, g, 0) |
| OR(f, g)  | ITE(f, 1, g) |
| XOR(f, g) | ITE(f, NOT g, g) |
| NOT(f)    | ITE(f, 0, 1) |

### `bdd_and`, `bdd_or`, `bdd_xor`, `bdd_not`

The four fundamental gates:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();

let f_and = mgr.bdd_and(x, y);   // x AND y
let f_or  = mgr.bdd_or(x, y);    // x OR y
let f_xor = mgr.bdd_xor(x, y);   // x XOR y
let f_not = mgr.bdd_not(x);       // NOT x (O(1), no allocation)
```

### `bdd_nand`, `bdd_nor`, `bdd_xnor`

Complemented gate outputs:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();

let f_nand = mgr.bdd_nand(x, y);  // NOT(x AND y)
let f_nor  = mgr.bdd_nor(x, y);   // NOT(x OR y)
let f_xnor = mgr.bdd_xnor(x, y); // NOT(x XOR y) = equivalence
```

## Quantification (Abstraction)

Quantification eliminates variables from a Boolean function. The variables to quantify over are specified as a **cube** -- a conjunction of variable projections built with `bdd_cube`.

### `bdd_exist_abstract` -- Existential quantification

Computes `EXISTS vars . f`, which is equivalent to OR-ing the positive and negative cofactors of `f` with respect to each variable in the cube.

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();

let f = mgr.bdd_and(x, y);
let cube_y = mgr.bdd_cube(&[1]); // quantify over y

// EXISTS y . (x AND y) = x
let result = mgr.bdd_exist_abstract(f, cube_y);
assert_eq!(result, x);
```

### `bdd_univ_abstract` -- Universal quantification

Computes `FORALL vars . f`, equivalent to AND-ing the cofactors.

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();

let f = mgr.bdd_or(x, y);
let cube_y = mgr.bdd_cube(&[1]);

// FORALL y . (x OR y) = x  (true when x=1 regardless of y)
let result = mgr.bdd_univ_abstract(f, cube_y);
assert_eq!(result, x);
```

### `bdd_and_abstract` -- Fused AND + existential

Computes `EXISTS vars . (f AND g)` in a single traversal, which is more efficient than computing the AND first and then quantifying. This is the core operation in relational product (image computation) for model checking.

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();

let f = mgr.bdd_ith_var(0);
let g = mgr.bdd_ith_var(1);
let cube = mgr.bdd_cube(&[0, 1]);

// EXISTS x,y . (x AND y) = 1 (the function is satisfiable)
let result = mgr.bdd_and_abstract(f, g, cube);
assert!(mgr.bdd_is_tautology(result));
```

### `bdd_xor_exist_abstract`

Computes `EXISTS vars . (f XOR g)`.

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();

let cube_y = mgr.bdd_cube(&[1]);

// EXISTS y . (x XOR y) = 1  (for any x, there exists y making it true)
let result = mgr.bdd_xor_exist_abstract(x, y, cube_y);
assert!(mgr.bdd_is_tautology(result));
```

## Composition

Composition replaces variables inside a BDD with other Boolean functions.

### `bdd_compose`

Substitutes function `g` for variable `v` in function `f`: computes `f[v := g]`.

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();
let z = mgr.bdd_new_var();

let f = mgr.bdd_or(x, y);

// Replace x with (y AND z): result is (y AND z) OR y = y
let g = mgr.bdd_and(y, z);
let result = mgr.bdd_compose(f, g, 0);
assert_eq!(result, y);
```

### `bdd_vector_compose`

Simultaneously substitutes a vector of functions for variables. `compose_vec[i]` is the replacement for variable `i`. Pass the variable's own projection function (`bdd_ith_var(i)`) to leave it unchanged.

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let a = mgr.bdd_new_var(); // 0
let b = mgr.bdd_new_var(); // 1
let c = mgr.bdd_new_var(); // 2

let f = mgr.bdd_and(a, b); // a AND b

// Swap a and b: replace var 0 with var 1 and var 1 with var 0
let compose = vec![
    mgr.bdd_ith_var(1), // var 0 -> var 1
    mgr.bdd_ith_var(0), // var 1 -> var 0
    mgr.bdd_ith_var(2), // var 2 unchanged
];
let result = mgr.bdd_vector_compose(f, &compose);
// a AND b with swap is still b AND a = a AND b
assert_eq!(result, f);
```

### `bdd_permute`

Renames variables according to a permutation. `perm_map[i]` is the new variable index for variable `i`. This is more efficient than `bdd_vector_compose` when only renaming variables (no general function substitution).

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var(); // 0
let y = mgr.bdd_new_var(); // 1
let z = mgr.bdd_new_var(); // 2

let f = mgr.bdd_and(x, y); // var0 AND var1

// Rename: var0->var2, var1->var0, var2->var1
let perm = vec![2, 0, 1];
let result = mgr.bdd_permute(f, &perm);

let expected = mgr.bdd_and(z, y.not().not()); // var2 AND var0
// (result depends on the actual renaming)
```

### `bdd_swap_variables`

Swaps two sets of variables in a BDD. `x_vars` and `y_vars` must have the same length; variable `x_vars[i]` is swapped with `y_vars[i]`.

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let a = mgr.bdd_new_var(); // 0
let b = mgr.bdd_new_var(); // 1

let f = mgr.bdd_ith_var(0); // just variable a

let result = mgr.bdd_swap_variables(f, &[0], &[1]);
let expected = mgr.bdd_ith_var(1); // should now be variable b
assert_eq!(result, expected);
```

## Simplification

These operations simplify a BDD given a constraint (care set or don't-care set).

### `bdd_restrict`

Generalized cofactor (Coudert's restrict): simplifies `f` under the assumption that constraint `c` is true. The result agrees with `f` for all assignments where `c` is true. Often produces a smaller BDD than `f`.

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();

let f = mgr.bdd_or(x, y);

// Restrict f assuming x is true
let c = mgr.bdd_ith_var(0); // x
let result = mgr.bdd_restrict(f, c);
assert!(mgr.bdd_is_tautology(result)); // x OR y with x=1 is always 1
```

### `bdd_constrain`

Constrain (Coudert & Madre): another simplification operator. Given `f` and constraint `c`, returns a function that agrees with `f` wherever `c` is true. May produce different (sometimes smaller) results than `restrict`.

### `bdd_li_compaction`

The Li compaction method: a simplification operator that may produce results smaller than both `restrict` and `constrain`.

### `bdd_squeeze`

Computes a function between lower bound `lb` and upper bound `ub` (where `lb` implies `ub`) that is as small as possible.

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();

let lb = mgr.bdd_and(x, y);      // lower bound
let ub = mgr.bdd_or(x, y);       // upper bound
let squeezed = mgr.bdd_squeeze(lb, ub);

// squeezed is between lb and ub
assert!(mgr.bdd_leq(lb, squeezed));
assert!(mgr.bdd_leq(squeezed, ub));
```

## Queries

### `bdd_leq` -- Implication check

Tests whether `f` implies `g` (i.e., `f` is a subset of `g` as a set of minterms).

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();

let f = mgr.bdd_and(x, y);
let g = mgr.bdd_or(x, y);
assert!(mgr.bdd_leq(f, g)); // (x AND y) implies (x OR y)
```

### `bdd_is_tautology`, `bdd_is_unsat`

Check if a BDD is the constant ONE or ZERO:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();

let taut = mgr.bdd_or(x, mgr.bdd_not(x));
assert!(mgr.bdd_is_tautology(taut));

let unsat = mgr.bdd_and(x, mgr.bdd_not(x));
assert!(mgr.bdd_is_unsat(unsat));
```

### `bdd_eval`

Evaluates a BDD on a complete variable assignment:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();

let f = mgr.bdd_xor(x, y);
assert!(mgr.bdd_eval(f, &[true, false]));
assert!(!mgr.bdd_eval(f, &[true, true]));
```

### `bdd_support`

Returns the sorted list of variable indices that a BDD depends on:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var(); // 0
let y = mgr.bdd_new_var(); // 1
let z = mgr.bdd_new_var(); // 2

let f = mgr.bdd_and(x, z); // depends on x and z, not y
let support = mgr.bdd_support(f);
assert_eq!(support, vec![0, 2]);
```

### `bdd_dag_size`

Returns the number of nodes in the BDD DAG (excluding shared subtrees counted once):

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();

let f = mgr.bdd_and(x, y);
println!("DAG size: {}", mgr.bdd_dag_size(f));
```

### `bdd_count_minterm`

Counts the number of satisfying assignments given a total number of variables. See the [Quick Start](../getting-started/quick-start.md) for examples.

For very large counts that overflow `f64`, use `bdd_count_minterm_epd` (extended double precision) or `bdd_count_minterm_apa` (arbitrary precision integer).

### `bdd_count_path`

Counts the number of paths from root to the ONE terminal in the BDD DAG (not the same as minterms -- a single path may correspond to multiple minterms if variables are skipped).

### `bdd_density`

Returns the fraction of minterms that satisfy the function: `count_minterm(f, n) / 2^n`.

### `bdd_is_var_essential`

Tests whether a variable is essential to a function (i.e., the function's value depends on the variable for at least one assignment).

## Cube Operations

A "cube" is a conjunction of literals (variables or their negations), representing a single row in a truth table or a product term.

### `bdd_cube`

Builds a cube (conjunction) from a list of variable indices (all positive polarity):

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
mgr.bdd_ith_var(2); // ensure 3 variables exist

let cube = mgr.bdd_cube(&[0, 2]); // x0 AND x2
assert_eq!(mgr.bdd_count_minterm(cube, 3), 2.0); // 2 minterms (x1 can be 0 or 1)
```

### `bdd_cube_with_phase`

Builds a cube with specified literal polarities:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
mgr.bdd_ith_var(1);

let cube = mgr.bdd_cube_with_phase(&[0, 1], &[true, false]);
// x0 AND (NOT x1)
```

### `bdd_pick_one_cube`

Extracts a single satisfying assignment from a non-zero BDD:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();

let f = mgr.bdd_or(x, y);
if let Some(cube) = mgr.bdd_pick_one_cube(f) {
    println!("Satisfying assignment: {:?}", cube);
    assert!(mgr.bdd_eval(f, &cube));
}
```

### `bdd_pick_one_minterm`

Picks a single minterm (complete assignment) from a BDD, returning it as a cube BDD that includes all specified variables.

### `bdd_make_prime`

Expands a cube to a prime implicant of function `f`:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();

let f = mgr.bdd_or(x, y);
let cube = mgr.bdd_cube_with_phase(&[0, 1], &[true, true]); // x AND y

let prime = mgr.bdd_make_prime(cube, f);
// The prime implicant might be just x or just y
```

### `bdd_largest_cube`

Returns the largest cube (fewest literals) contained in the function, along with its length.

### `bdd_shortest_path`

Returns the shortest path (fewest variable tests) from root to the ONE terminal as a list of `(variable, phase)` pairs.

## Iteration

### `bdd_iter_cubes`

Returns all cubes (disjoint cover) of a BDD. Each cube is a vector where `Some(true)` means the variable is positive, `Some(false)` means negative, and `None` means don't-care:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();

let f = mgr.bdd_or(x, y);
for cube in mgr.bdd_iter_cubes(f) {
    let desc: Vec<String> = cube.iter().enumerate().map(|(i, v)| {
        match v {
            Some(true) => format!("x{}=1", i),
            Some(false) => format!("x{}=0", i),
            None => format!("x{}=*", i),
        }
    }).collect();
    println!("{}", desc.join(", "));
}
```

### `bdd_foreach_prime`

Iterates over all prime implicants of a function (those between a lower and upper bound) and invokes a callback for each:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();

let f = mgr.bdd_or(x, y);
mgr.bdd_foreach_prime(f, mgr.one(), |prime_cube| {
    println!("Prime: {:?}", prime_cube);
});
```

### `bdd_foreach_node`

Visits every node in a BDD DAG and invokes a callback with the node ID:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();
let f = mgr.bdd_and(x, y);

let mut count = 0;
mgr.bdd_foreach_node(f, |_node_id| {
    count += 1;
});
println!("Visited {} nodes", count);
```

### `bdd_print_minterms`

Prints all minterms to stdout in a human-readable format.

## Comparison Functions

These build BDDs that compare two binary-encoded integers.

### `bdd_xeqy`

Builds a BDD for `X == Y` where X and Y are bit-vectors:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
// Create variables for two 3-bit numbers
for _ in 0..6 { mgr.bdd_new_var(); }

// X = {x0, x1, x2}, Y = {x3, x4, x5}
let eq = mgr.bdd_xeqy(&[0, 1, 2], &[3, 4, 5]);
assert_eq!(mgr.bdd_count_minterm(eq, 6), 8.0); // 8 matching pairs out of 64
```

### `bdd_xgty`

Builds a BDD for `X > Y` where X and Y are bit-vectors (most significant bit first).

### `bdd_inequality`

Builds a BDD representing the inequality `sum(x_i * 2^i) <= threshold` for a set of Boolean variables.

### `bdd_interval`

Builds a BDD for `lower <= X <= upper` where X is a binary-encoded integer.

### `bdd_disequality`

Builds a BDD for `X != c` where X is a bit-vector and c is a constant.

### `bdd_hamming_distance`

Builds a BDD/ADD representing the Hamming distance between two bit-vectors.

## Correlation

### `bdd_correlation`

Computes the fraction of minterms where two functions agree:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();

let f = mgr.bdd_and(x, y);
let g = mgr.bdd_or(x, y);

let corr = mgr.bdd_correlation(f, g, 2);
println!("Correlation: {}", corr); // 0.25 (they agree on 1 out of 4 minterms where both are 1, but correlation counts differently)
```

### `bdd_correlation_weights`

Computes weighted correlation where each variable has an associated weight (probability of being 1).

## Clipping

Clipping operations compute an approximation of AND/OR by limiting the recursion depth.

### `bdd_clip_and`

Computes an approximation of `f AND g` within a recursion depth limit. The `direction` parameter controls whether the approximation is an under-approximation or over-approximation.

### `bdd_clip_or`

Computes an approximation of `f OR g` within a recursion depth limit.

```rust
use lumindd::{Manager, ClipDirection};

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();

let result = mgr.bdd_clip_and(x, y, 100, ClipDirection::Under);
// With sufficient depth, this equals exact bdd_and
let exact = mgr.bdd_and(x, y);
assert_eq!(result, exact);
```

## Clause Extraction

### `bdd_two_literal_clauses`

Extracts all two-literal clauses implied by a BDD. Each clause is a pair of `Literal` values:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();

let f = mgr.bdd_or(x, y); // equivalent to clause (x OR y)
let clauses = mgr.bdd_two_literal_clauses(f);
println!("Two-literal clauses: {:?}", clauses);
```

### `bdd_implication_pairs`

Extracts all implication relationships `(a => b)` between variable literals that hold for every satisfying assignment of the BDD.

## Simulation

### `bdd_signature`

Computes a hash-like signature of a BDD using random simulation with the specified number of samples. Two BDDs with different signatures are guaranteed to represent different functions:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();

let f = mgr.bdd_and(x, y);
let g = mgr.bdd_or(x, y);
let sig_f = mgr.bdd_signature(f, 1000);
let sig_g = mgr.bdd_signature(g, 1000);
assert_ne!(sig_f, sig_g); // different functions -> different signatures (with high probability)
```

### `bdd_signatures_match`

Tests whether two BDDs have matching signatures (high probability of equivalence).

### `bdd_simulate`

Performs bit-parallel simulation of a BDD on a batch of input vectors:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();

let f = mgr.bdd_and(x, y);

let inputs = vec![
    vec![true, false],   // pattern 0
    vec![true, true],    // pattern 1
];
let outputs = mgr.bdd_simulate(f, &inputs);
assert_eq!(outputs, vec![false, true]);
```

## Additional Operations

### `bdd_boolean_diff`

Computes the Boolean difference of `f` with respect to a variable: `f(x=1) XOR f(x=0)`. The result is 1 wherever the function is sensitive to the variable.

### `bdd_intersect`

Computes a function that is true only where both `f` and `g` are true and that may be simpler than `bdd_and`. Unlike `bdd_and`, the result is not guaranteed to be exact -- it is a simplification useful when an exact AND is too expensive.

### `bdd_split_set`

Splits the minterms of a BDD into two roughly equal halves.

### `bdd_equiv_dc`

Tests if `f` and `g` are equivalent under don't-care set `dc`.

### `bdd_leq_unless`

Tests if `f` implies `g` under don't-care set `dc`.

### `bdd_increasing`, `bdd_decreasing`

Tests whether a BDD is a monotone increasing (or decreasing) function in a given variable.

### `bdd_np_and`

Computes the AND of `f` and `g` with negative polarity on `g` (i.e., `f AND NOT g`).

### `bdd_cofactor_ratio`

Returns the ratio of minterm counts of the positive and negative cofactors of a function with respect to a variable.

### `bdd_random_minterms`

Generates a specified number of random satisfying assignments from a BDD.

### `bdd_sharing_size`

Computes the total number of shared nodes across a vector of BDDs.

### `bdd_count_leaves`

Counts the number of terminal (leaf) nodes reachable from a BDD.

### `bdd_estimate_cofactor`

Estimates the BDD size of a cofactor without actually computing it.

### `bdd_priority_select`

Priority-based function selection from a vector of BDDs.

### `bdd_transfer`

Copies a BDD from another manager into this one, preserving its structure.
