# Serialization (DDDMP)

lumindd supports saving and loading BDDs using the DDDMP format, the de facto standard for decision diagram interchange. Three serialization modes are available:

- **Text format** -- human-readable, includes variable names
- **Binary format** -- compact, suitable for large diagrams
- **CNF export** -- DIMACS format for SAT solver interoperability

All serialization functions are methods on `Manager`.

## Text Format

The text format writes one line per node in a topological (bottom-up) order, preceded by a header that records variable names, the permutation, and root node references.

### Saving

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();
let f = mgr.bdd_and(x, y);

// Save with symbolic variable names
let mut output = Vec::new();
mgr.dddmp_save_text(f, Some(&["a", "b"]), &mut output).unwrap();

let text = String::from_utf8(output).unwrap();
println!("{}", text);
```

Output:

```
.ver DDDMP-2.0
.mode A
.varinfo 0
.nnodes 3
.nvars 2
...
.nodes
1 T 1 0 0
2 1 1 -1 0
3 0 2 -1 0
.end
```

If you omit variable names, default names `x0, x1, ...` are used:

```rust
mgr.dddmp_save_text(f, None, &mut output).unwrap();
```

### Loading

```rust
use std::io::BufReader;

let mut mgr2 = Manager::new();
let mut reader = BufReader::new(saved_bytes.as_slice());
let loaded = mgr2.dddmp_load_text(&mut reader).unwrap();

// The loaded BDD is functionally identical to the original
```

Variables are created automatically in the target manager if it does not already have enough. The loaded root node is automatically referenced (its reference count is incremented).

## Binary Format

The binary format uses a text header (identical structure to the text format, but with `.mode B`) followed by a compact binary encoding of nodes. Node children are stored using variable-length integer encoding (LEB128), making the format efficient for large diagrams.

### Saving

```rust
let mut output = Vec::new();
mgr.dddmp_save_binary(f, &mut output).unwrap();
```

### Loading

```rust
let mut mgr2 = Manager::new();
let loaded = mgr2.dddmp_load_binary(&mut output.as_slice()).unwrap();
```

### Text vs. Binary

| Aspect | Text | Binary |
|---|---|---|
| Human-readable | Yes | No (header only) |
| Size | Larger | Smaller |
| Variable names | Preserved in header | Default names only |
| Roundtrip fidelity | Exact | Exact |

Both formats preserve the BDD structure exactly. Choose text format for debugging and interop with other tools; choose binary format for storage efficiency.

## CNF Export (DIMACS)

The CNF export converts a BDD into a conjunctive normal form (CNF) formula in DIMACS format, suitable for input to any SAT solver.

The encoding introduces auxiliary variables for each BDD node. For each internal node, clauses encode the ITE relationship between the node, its decision variable, and its children. The root node is asserted, making the CNF equisatisfiable with the original BDD.

```rust
let mut output = Vec::new();
mgr.dddmp_save_cnf(f, &mut output).unwrap();

let cnf = String::from_utf8(output).unwrap();
println!("{}", cnf);
```

Output:

```
p cnf 5 9
3 0
-4 -1 3 0
-4 1 -3 0
4 -1 -3 0
4 1 3 0
...
```

### Variable Numbering

- BDD variable `i` maps to DIMACS variable `i + 1`
- Auxiliary node variables are numbered starting from `num_vars + 1`

### Special Cases

- Constant ONE produces an empty CNF: `p cnf 0 0`
- Constant ZERO produces a single empty clause: `p cnf 0 1` followed by `0`

## Complete Roundtrip Example

```rust
use lumindd::Manager;
use std::io::BufReader;

fn main() {
    // Build a BDD
    let mut mgr = Manager::new();
    let a = mgr.bdd_new_var();
    let b = mgr.bdd_new_var();
    let c = mgr.bdd_new_var();
    let f = mgr.bdd_or(mgr.bdd_and(a, b), c);
    mgr.ref_node(f);

    // Save to text format
    let mut buf = Vec::new();
    mgr.dddmp_save_text(f, Some(&["a", "b", "c"]), &mut buf).unwrap();

    // Load into a new manager
    let mut mgr2 = Manager::new();
    let mut reader = BufReader::new(buf.as_slice());
    let f2 = mgr2.dddmp_load_text(&mut reader).unwrap();

    // Verify equivalence
    let a2 = mgr2.bdd_ith_var(0);
    let b2 = mgr2.bdd_ith_var(1);
    let c2 = mgr2.bdd_ith_var(2);
    let expected = mgr2.bdd_or(mgr2.bdd_and(a2, b2), c2);
    assert_eq!(f2, expected);
}
```

## API Reference

| Method | Description |
|---|---|
| `dddmp_save_text(f, var_names, out)` | Save BDD `f` in DDDMP text format |
| `dddmp_load_text(input)` | Load a BDD from DDDMP text format |
| `dddmp_save_binary(f, out)` | Save BDD `f` in DDDMP binary format |
| `dddmp_load_binary(input)` | Load a BDD from DDDMP binary format |
| `dddmp_save_cnf(f, out)` | Export BDD `f` as a DIMACS CNF formula |
