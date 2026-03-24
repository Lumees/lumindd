# Decomposition and Equation Solving

lumindd provides methods for decomposing Boolean functions into simpler components and for solving Boolean equations. These operations are fundamental in logic synthesis, formal verification, and constraint solving.

## Conjunctive Decomposition

Conjunctive decomposition finds `g` and `h` such that `f = g AND h`, where neither `g` nor `h` is trivially ONE. This splits a complex function into two independent (or nearly independent) parts.

```rust
let mut mgr = Manager::new();
// ... build BDD f ...

let (g, h) = mgr.bdd_conjunctive_decomp(f);

// Verify: g AND h == f
let product = mgr.bdd_and(g, h);
assert_eq!(product, f);
```

### Algorithm

The method tries several strategies to find a good decomposition:

1. **Cofactor implication check:** For each top variable `v`, if `f|_{v=0}` implies `f|_{v=1}`, then `f = (v OR f|_{v=0}) AND f|_{v=1}`. Similarly for the reverse implication.

2. **Support splitting:** If no cofactor implication exists, the method splits the support set in half, existentially quantifies one half to get `g`, and computes `h = constrain(f, g)`.

3. **Optimization:** Among all candidate decompositions, the one minimizing `max(|g|, |h|)` is chosen.

If no non-trivial decomposition exists, returns `(f, ONE)`.

## Disjunctive Decomposition

Disjunctive decomposition finds `g` and `h` such that `f = g OR h`, using duality with conjunctive decomposition.

```rust
let (g, h) = mgr.bdd_disjunctive_decomp(f);

// Verify: g OR h == f
let sum = mgr.bdd_or(g, h);
assert_eq!(sum, f);
```

**Implementation:** `f = g OR h` if and only if `NOT(f) = NOT(g) AND NOT(h)`. So the method computes the conjunctive decomposition of `NOT(f)` and complements the results.

## Iterative Decomposition

For maximum decomposition, use `bdd_iterative_conjunctive_decomp` to repeatedly decompose until no further splitting is possible (or a maximum number of parts is reached).

```rust
let parts = mgr.bdd_iterative_conjunctive_decomp(f, 8);

// Verify: AND of all parts == f
let mut product = mgr.one();
for &part in &parts {
    product = mgr.bdd_and(product, part);
}
assert_eq!(product, f);

println!("Decomposed into {} parts", parts.len());
```

Each part depends on a (hopefully) smaller subset of variables than the original function, making subsequent operations on the parts more efficient.

## Equation Solving

`bdd_solve_eqn` solves the Boolean equation `f(x, var) = 0` for a specific variable `var`. It returns a particular solution and a care set.

```rust
let mut mgr = Manager::new();
let x = mgr.bdd_new_var();  // var 0
let y = mgr.bdd_new_var();  // var 1

// f = x XOR y (we want to find y such that f = 0)
let f = mgr.bdd_xor(x, y);

let (particular, care) = mgr.bdd_solve_eqn(f, 1); // solve for var 1
```

### Understanding the Result

- **particular**: A BDD `g` over the remaining variables such that substituting `g` for `var` makes `f = 0`.
- **care**: The set of assignments to the remaining variables where a solution exists.

The general solution for `var` is: `(particular AND care) OR (anything AND NOT(care))`.

On the care set, the particular solution is the only one that works. Off the care set, any value for `var` is acceptable (the equation is trivially satisfied or unsatisfiable regardless).

### Theory

Given `f(var, others)`, let `f_pos = f|_{var=1}` and `f_neg = f|_{var=0}`.

The equation `f = 0` is solvable when `f_pos AND f_neg = 0` (for each assignment to the other variables, at least one cofactor is zero).

The particular solution is `NOT(f_pos)`: where `f_pos = 0`, set `var = 1`; where `f_pos = 1`, set `var = 0`.

### Verification

Use `bdd_verify_sol` to check that solutions are correct:

```rust
let vars = [1u16]; // variable 1
let solutions = [particular];

let ok = mgr.bdd_verify_sol(f, &vars, &solutions);
assert!(ok, "Solution should satisfy f = 0");
```

This substitutes each solution for its corresponding variable and checks that the result is the zero function.

## Essential Variables

`bdd_essential_vars` identifies variables that appear on every path from the root to the ONE terminal -- variables that the function truly depends on and that cannot be projected away without changing the function.

```rust
let essential = mgr.bdd_essential_vars(f);
println!("Essential variables: {:?}", essential);
```

A variable `v` is essential in `f` if both cofactors `f|_{v=0}` and `f|_{v=1}` differ from `f`. The result is sorted by variable level.

### Use Cases

- **Logic minimization:** Essential variables cannot be removed; optimization should focus on non-essential variables.
- **Abstraction:** When simplifying a function, essential variables must be preserved.

## Compatible Projection

`bdd_compatible_projection` projects a BDD onto a subset of its variables by existentially quantifying away all other variables.

```rust
// Keep only variables in the cube, quantify the rest
let projected = mgr.bdd_compatible_projection(f, cube);
```

The `cube` parameter is a conjunction of the variables to keep. All variables in `f`'s support that do not appear in `cube` are existentially quantified.

```rust
// Keep only variables 0 and 2; quantify variable 1
let keep = mgr.bdd_cube(&[0, 2]);
let projected = mgr.bdd_compatible_projection(f, keep);
```

### Special Cases

- If `cube` is ONE (keep no variables), the result is ONE if `f` is satisfiable, ZERO otherwise.
- If `f`'s support is a subset of the cube's variables, the result is `f` unchanged.

## API Reference

| Method | Description |
|---|---|
| `bdd_conjunctive_decomp(f)` | Decompose f = g AND h |
| `bdd_disjunctive_decomp(f)` | Decompose f = g OR h |
| `bdd_iterative_conjunctive_decomp(f, max)` | Decompose into up to `max` conjuncts |
| `bdd_solve_eqn(f, var)` | Solve f = 0 for `var` |
| `bdd_verify_sol(f, vars, solutions)` | Verify equation solutions |
| `bdd_essential_vars(f)` | Find essential variables of f |
| `bdd_compatible_projection(f, cube)` | Project f onto cube variables |
