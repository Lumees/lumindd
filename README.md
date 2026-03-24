# lumindd

A pure Rust decision diagram library for manipulating **BDD**, **ADD**, and **ZDD** data structures.

Built by [Lumees Lab](https://lumeeslab.com) ([GitHub](https://github.com/Lumees)).

[![License: BSD-3-Clause](https://img.shields.io/badge/License-BSD_3--Clause-blue.svg)](LICENSE)

## Overview

**lumindd** is a from-scratch Rust implementation inspired by the [CUDD](https://github.com/The-OpenROAD-Project/cudd) library architecture. It provides:

- **BDD** (Binary Decision Diagrams) — Boolean function manipulation with complemented edges
- **ADD** (Algebraic Decision Diagrams) — functions mapping Boolean domains to real values
- **ZDD** (Zero-suppressed Decision Diagrams) — families of sets with zero-suppressed reduction

### Key Features

- **402 public API functions** covering BDD/ADD/ZDD operations, reordering, serialization, approximation, decomposition, and more
- **17 variable reordering algorithms**: sifting, symmetric sifting, group sifting, linear sifting, window (2/3/4), simulated annealing, genetic algorithm, exact DP — all with convergence variants
- **DDDMP-compatible serialization** in text, binary, and CNF (DIMACS) formats
- **Variable grouping** via MTR (Multi-way Tree) for constrained reordering
- **8 BDD approximation methods**: under/over-approximation, heavy-branch/short-path subsetting, remap, biased, squeeze, compress
- **Extended precision counting**: EPD (extended double) and APA (arbitrary precision integer)
- **5 export formats**: DOT, BLIF, DaVinci, factored form, truth table
- **Safe RAII wrapper types** (`Bdd`, `Add`, `Zdd`) with automatic reference counting
- **Zero dependencies** beyond `bitflags`

## Quick Start

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();

let f = mgr.bdd_and(x, y);     // x AND y
let g = mgr.bdd_or(x, y);      // x OR y
let h = mgr.bdd_not(f);         // NOT(x AND y)

let taut = mgr.bdd_or(f, h);
assert!(mgr.bdd_is_tautology(taut)); // f OR NOT(f) = 1

// Count satisfying assignments
assert_eq!(mgr.bdd_count_minterm(f, 2), 1.0);  // x AND y: 1 of 4
assert_eq!(mgr.bdd_count_minterm(g, 2), 3.0);  // x OR y: 3 of 4
```

## Architecture

All decision diagram nodes live in a central `Manager` arena. Nodes are referenced by `NodeId` handles that encode a **complemented-edge bit** in the LSB for O(1) negation.

- **Unique tables** ensure canonical representation (one hash table per variable level)
- **Computed table** provides operation result caching (direct-mapped, lossy)
- **Reference counting** with saturation prevents overflow

## Modules

| Category | Description |
|----------|-------------|
| `bdd` | Core BDD operations: ITE, AND, OR, XOR, quantification, composition, restrict, constrain |
| `add` | ADD operations: apply, monadic, ITE, abstraction, matrix multiply, Walsh |
| `zdd` | ZDD operations: union, intersect, diff, product, ISOP, complement |
| `reorder` | 17 reordering algorithms with unified dispatch via `ExtReorderMethod` |
| `mtr` | Variable grouping trees for constrained reordering |
| `dddmp` | Serialization: text, binary, CNF formats |
| `epd` / `apa` | Extended and arbitrary precision arithmetic |
| `export` | DOT, BLIF, DaVinci, factored form, truth table |
| `bdd_approx` | 8 approximation/subsetting methods |
| `bdd_priority` | Inequality, interval, Hamming distance comparisons |
| `bdd_decomp` | Conjunctive/disjunctive decomposition, equation solving |
| `hooks` | Pre/post reorder and GC callbacks |
| `wrapper` | Safe RAII types with operator overloading |
| `debug` | Invariant checking and statistics |

## License

BSD-3-Clause. See [LICENSE](LICENSE) for details.

Inspired by the CUDD library by Fabio Somenzi, University of Colorado Boulder.

## Author

**Hasan Kurşun** — [Lumees Lab](https://lumeeslab.com)
