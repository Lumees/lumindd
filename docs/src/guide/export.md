# Export Formats

lumindd supports exporting decision diagrams in multiple formats for visualization, analysis, and interoperability with external tools.

## DOT (Graphviz)

The DOT format produces graph descriptions that can be rendered by Graphviz tools (`dot`, `neato`, etc.) into images.

### Standard DOT Export

lumindd provides a standard `dump_dot` method (see the core BDD operations) and an enhanced version with node highlighting.

### DOT with Highlighting

The `dump_dot_color` method renders a BDD with specific nodes highlighted in a distinct color. This is useful for visualizing critical paths, debugging, or showing the result of approximation.

```rust
use lumindd::Manager;
use lumindd::node::NodeId;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();
let f = mgr.bdd_and(x, y);

// Highlight the root node
let mut output = Vec::new();
mgr.dump_dot_color(f, &[f], &mut output).unwrap();

let dot = String::from_utf8(output).unwrap();
println!("{}", dot);
```

Visual conventions:
- **Terminal nodes**: boxes -- ONE is green, ZERO is pink
- **Internal nodes**: ellipses -- normal nodes are light blue, highlighted nodes are gold
- **Then-edges**: solid lines (red if complemented)
- **Else-edges**: dashed lines (blue normally, red if complemented)

To render:

```bash
echo "$dot_output" | dot -Tpng -o bdd.png
```

## BLIF (Berkeley Logic Interchange Format)

BLIF is a standard format for representing logic networks. Each BDD node becomes a `.names` table entry that encodes the ITE (if-then-else) relationship.

```rust
let mut output = Vec::new();
mgr.dump_blif(f, Some(&["a", "b"]), "output", &mut output).unwrap();

let blif = String::from_utf8(output).unwrap();
println!("{}", blif);
```

Output:

```
.model bdd
.inputs a b
.outputs output
.names a n2
11 1
.names n2 output
1 1
.end
```

### Parameters

- `f` -- the BDD root
- `var_names` -- optional symbolic names for variables (`Some(&["a", "b"])` or `None` for default `x0, x1, ...`)
- `output_name` -- name for the primary output signal
- `out` -- any `Write` implementor

### Use Cases

- Import into logic synthesis tools (ABC, SIS, VIS)
- Structural analysis of the logic network
- Technology mapping workflows

## DaVinci Graph Format

DaVinci is a graph visualization tool that uses a term-based representation. Each node is described as a nested structure with attributes for labels, colors, and shapes.

```rust
let mut output = Vec::new();
mgr.dump_davinci(f, &mut output).unwrap();
```

The output uses DaVinci's `l(..., n(..., [...], [...]))` syntax with:
- Internal nodes colored blue with variable labels
- Terminal nodes as boxes (green for ONE, red for ZERO)
- Then-edges in black (red if complemented)
- Else-edges in blue, dashed (red if complemented)

## Factored Form (Boolean Expressions)

The factored form export converts a BDD into a human-readable Boolean expression string. This is useful for displaying small functions in documentation, logs, or user interfaces.

```rust
let expr = mgr.dump_factored_form(f);
println!("{}", expr);
// Output: (x0 & x1)
```

### Expression Syntax

| Symbol | Meaning |
|---|---|
| `&` | AND |
| `\|` | OR |
| `!` | NOT |
| `x0`, `x1`, ... | Variable names |
| `1` | Constant TRUE |
| `0` | Constant FALSE |

### Simplifications

The method recognizes and simplifies common patterns:

- `ITE(x, ONE, ZERO)` becomes `x`
- `ITE(x, ZERO, ONE)` becomes `!x`
- `ITE(x, t, ZERO)` becomes `(x & t)`
- `ITE(x, ZERO, e)` becomes `(!x & e)`
- `ITE(x, ONE, e)` becomes `(x | e)`
- `ITE(x, t, ONE)` becomes `(!x | t)`
- General case: `(x & t | !x & e)`

### Example

```rust
let mut mgr = Manager::new();
let a = mgr.bdd_new_var();
let b = mgr.bdd_new_var();

let f = mgr.bdd_or(a, b);
assert_eq!(mgr.dump_factored_form(f), "(x0 | x1)");

let g = mgr.bdd_and(a, b.not());
// Displays as: (x0 & !x1)
println!("{}", mgr.dump_factored_form(g));
```

## Truth Table

The truth table export enumerates all 2^n variable assignments and their function values. This is only practical for small functions.

```rust
let mut output = Vec::new();
mgr.dump_truth_table(f, &mut output).unwrap();
println!("{}", String::from_utf8(output).unwrap());
```

Output for `x0 AND x1`:

```
x0 x1 | f
-- ---+--
 0  0 | 0
 0  1 | 0
 1  0 | 0
 1  1 | 1
```

The method refuses to produce output for functions with more than 24 variables (to avoid generating tables with more than 16 million rows).

## API Summary

| Method | Format | Output |
|---|---|---|
| `dump_dot_color(f, highlight, out)` | DOT (Graphviz) | Graph with highlighted nodes |
| `dump_blif(f, var_names, output_name, out)` | BLIF | Logic network |
| `dump_davinci(f, out)` | DaVinci | Term-based graph |
| `dump_factored_form(f)` | String | Boolean expression |
| `dump_truth_table(f, out)` | Text | Complete truth table |
| `dddmp_save_cnf(f, out)` | DIMACS CNF | SAT solver input |
