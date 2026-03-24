# Quick Start

This tutorial walks through the basic operations of lumindd, building up from creating variables to exporting diagrams for visualization.

## Creating a Manager

All decision diagram operations go through a central `Manager` that owns the node arena:

```rust
use lumindd::Manager;

fn main() {
    let mut mgr = Manager::new();
    println!("Variables: {}", mgr.num_vars()); // 0
}
```

You can also pre-allocate variables and set the computed-table size:

```rust
use lumindd::Manager;

fn main() {
    // 8 BDD variables, 0 ZDD variables, cache of 2^20 entries
    let mut mgr = Manager::with_capacity(8, 0, 20);
    println!("Variables: {}", mgr.num_vars()); // 8
}
```

## Creating Variables

Each call to `bdd_new_var` creates a fresh Boolean variable and returns its `NodeId`:

```rust
use lumindd::Manager;

fn main() {
    let mut mgr = Manager::new();

    let x0 = mgr.bdd_new_var(); // variable index 0
    let x1 = mgr.bdd_new_var(); // variable index 1
    let x2 = mgr.bdd_new_var(); // variable index 2

    println!("Number of variables: {}", mgr.num_vars()); // 3
}
```

You can also request a variable by index with `bdd_ith_var`. If the variable does not yet exist, it is created along with all variables up to that index:

```rust
use lumindd::Manager;

fn main() {
    let mut mgr = Manager::new();

    let x5 = mgr.bdd_ith_var(5); // creates variables 0 through 5
    println!("Number of variables: {}", mgr.num_vars()); // 6
}
```

## Building Boolean Functions

The fundamental Boolean operations are all methods on `Manager`:

```rust
use lumindd::Manager;

fn main() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();

    // Basic gates
    let f_and  = mgr.bdd_and(x, y);     // x AND y
    let f_or   = mgr.bdd_or(x, y);      // x OR y
    let f_not  = mgr.bdd_not(x);         // NOT x
    let f_xor  = mgr.bdd_xor(x, y);     // x XOR y
    let f_nand = mgr.bdd_nand(x, y);     // NOT(x AND y)
    let f_nor  = mgr.bdd_nor(x, y);      // NOT(x OR y)
    let f_xnor = mgr.bdd_xnor(x, y);    // NOT(x XOR y), equivalence

    // ITE (if-then-else) is the universal BDD operation
    let f_ite = mgr.bdd_ite(x, y, f_not); // if x then y else (NOT x)

    // Verify De Morgan's law: NOT(x AND y) = (NOT x) OR (NOT y)
    let lhs = mgr.bdd_nand(x, y);
    let rhs = mgr.bdd_or(mgr.bdd_not(x), mgr.bdd_not(y));
    assert_eq!(lhs, rhs);
}
```

Note that `bdd_not` is O(1) -- it simply flips the complement bit in the `NodeId` without allocating any nodes.

## Evaluating Functions

Given a complete assignment to all variables, you can evaluate a BDD to get a Boolean result:

```rust
use lumindd::Manager;

fn main() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // index 0
    let y = mgr.bdd_new_var(); // index 1

    let f = mgr.bdd_and(x, y);

    // assignment[i] is the value of variable i
    assert_eq!(mgr.bdd_eval(f, &[true, true]),   true);
    assert_eq!(mgr.bdd_eval(f, &[true, false]),  false);
    assert_eq!(mgr.bdd_eval(f, &[false, true]),  false);
    assert_eq!(mgr.bdd_eval(f, &[false, false]), false);
}
```

## Counting Minterms

The number of satisfying assignments (minterms) can be computed efficiently without enumerating them:

```rust
use lumindd::Manager;

fn main() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let z = mgr.bdd_new_var();

    let f = mgr.bdd_or(x, y); // x OR y (does not depend on z)

    // With 2 variables, x OR y has 3 satisfying assignments
    assert_eq!(mgr.bdd_count_minterm(f, 2), 3.0);

    // With 3 variables, each of those 3 assignments pairs with z=0 and z=1
    assert_eq!(mgr.bdd_count_minterm(f, 3), 6.0);

    // Tautology and unsatisfiable
    assert_eq!(mgr.bdd_count_minterm(mgr.one(), 3), 8.0);
    assert_eq!(mgr.bdd_count_minterm(mgr.zero(), 3), 0.0);
}
```

## Existential Quantification

Existential quantification removes a variable from a function by OR-ing its two cofactors. This is essential in model checking for image computation and in logic synthesis for variable elimination.

The quantified variables are specified as a "cube" (a conjunction of variable projections):

```rust
use lumindd::Manager;

fn main() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // index 0
    let y = mgr.bdd_new_var(); // index 1

    let f = mgr.bdd_and(x, y); // x AND y

    // Build a cube containing just variable y
    let cube_y = mgr.bdd_cube(&[1]);

    // Exists y. (x AND y) = x
    let result = mgr.bdd_exist_abstract(f, cube_y);
    assert_eq!(result, x);

    // Universal quantification: Forall y. (x AND y) = ZERO
    let result_univ = mgr.bdd_univ_abstract(f, cube_y);
    assert!(mgr.bdd_is_unsat(result_univ));
}
```

## Composition

Composition substitutes a Boolean function for a variable inside another function:

```rust
use lumindd::Manager;

fn main() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var(); // index 0
    let y = mgr.bdd_new_var(); // index 1
    let z = mgr.bdd_new_var(); // index 2

    let f = mgr.bdd_and(x, y); // x AND y

    // Replace variable 0 (x) with z in f: result is z AND y
    let g = mgr.bdd_compose(f, z, 0);

    let expected = mgr.bdd_and(z, y);
    assert_eq!(g, expected);
}
```

## DOT Export for Visualization

You can export any BDD to Graphviz DOT format for visualization:

```rust
use lumindd::Manager;

fn main() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();
    let z = mgr.bdd_new_var();

    let f = mgr.bdd_ite(x, y, z); // if x then y else z

    // Write DOT to a string
    let mut dot = Vec::new();
    mgr.dump_dot(f, &mut dot).unwrap();
    let dot_str = String::from_utf8(dot).unwrap();

    println!("{}", dot_str);

    // Or write directly to a file
    let mut file = std::fs::File::create("bdd.dot").unwrap();
    mgr.dump_dot(f, &mut file).unwrap();
}
```

Render the DOT file with Graphviz:

```sh
dot -Tpng bdd.dot -o bdd.png
```

## Putting It All Together

Here is a complete example that builds a 2-bit adder and analyzes its carry output:

```rust
use lumindd::Manager;

fn main() {
    let mut mgr = Manager::new();

    // Input bits: a0, a1, b0, b1
    let a0 = mgr.bdd_new_var(); // index 0
    let a1 = mgr.bdd_new_var(); // index 1
    let b0 = mgr.bdd_new_var(); // index 2
    let b1 = mgr.bdd_new_var(); // index 3

    // Half adder for bit 0
    let sum0 = mgr.bdd_xor(a0, b0);
    let carry0 = mgr.bdd_and(a0, b0);

    // Full adder for bit 1
    let xor1 = mgr.bdd_xor(a1, b1);
    let sum1 = mgr.bdd_xor(xor1, carry0);
    let carry1 = mgr.bdd_or(
        mgr.bdd_and(a1, b1),
        mgr.bdd_and(xor1, carry0),
    );

    // How many input combinations produce a carry out?
    let count = mgr.bdd_count_minterm(carry1, 4);
    println!("Carry-out minterms: {}", count); // 6 out of 16

    // What variables does the carry depend on?
    let support = mgr.bdd_support(carry1);
    println!("Carry support variables: {:?}", support);

    // DAG size (number of nodes in the BDD)
    let nodes = mgr.bdd_dag_size(carry1);
    println!("Carry BDD nodes: {}", nodes);

    // Check: carry1 implies (a1 OR b1 OR carry0)
    let upper = mgr.bdd_or(mgr.bdd_or(a1, b1), carry0);
    assert!(mgr.bdd_leq(carry1, upper));

    // Iterate over satisfying cubes
    let cubes = mgr.bdd_iter_cubes(carry1);
    println!("Number of cubes: {}", cubes.len());
}
```

## Next Steps

- Read [Core Concepts](./concepts.md) to understand the data structures behind the API.
- See the [BDD Operations Guide](../guide/bdd.md) for the complete set of BDD functions.
- Explore [ADD Operations](../guide/add.md) for real-valued functions.
- Learn about [ZDD Operations](../guide/zdd.md) for families of sets.
