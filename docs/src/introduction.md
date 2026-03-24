# Introduction

**lumindd** is a pure Rust library for manipulating decision diagrams -- compact data structures that represent Boolean functions, real-valued functions, and families of sets. It is developed by [Lumees Lab](https://lumeeslab.com) and authored by Hasan Kursun.

The library provides three diagram types under a unified `Manager` interface:

- **BDD** (Binary Decision Diagrams) -- represent and manipulate Boolean functions.
- **ADD** (Algebraic Decision Diagrams) -- represent functions from Boolean domains to real values.
- **ZDD** (Zero-suppressed Decision Diagrams) -- represent families of sets with automatic suppression of absent elements.

## What Are Decision Diagrams?

A decision diagram is a directed acyclic graph (DAG) that encodes a function by branching on input variables. At each internal node the diagram tests a variable and follows one of two edges depending on whether the variable is true or false. Terminal nodes hold the function's value (0/1 for BDDs, an arbitrary real for ADDs).

Because structurally identical subgraphs are shared and redundant nodes are eliminated, decision diagrams are often exponentially more compact than truth tables or sum-of-products representations. This makes them the backbone of several important areas in computer science and engineering:

- **Formal verification** -- model checking hardware and software systems against temporal logic specifications.
- **Logic synthesis** -- two-level and multi-level optimization of digital circuits.
- **Combinatorial optimization** -- encoding constraints, counting solutions, and enumerating feasible sets.
- **Symbolic model checking** -- representing state spaces and transition relations of finite-state systems.
- **Reliability analysis** -- computing system failure probabilities via algebraic decision diagrams.
- **Constraint satisfaction** -- compactly representing solution sets and performing set operations.

## Design Decisions

lumindd is a from-scratch Rust implementation inspired by the architecture of the [CUDD](https://github.com/The-OpenROAD-Project/cudd) library (University of Colorado Decision Diagram package by Fabio Somenzi). The key design choices are:

### Arena-based node storage

All decision diagram nodes live in a single flat `Vec<DdNode>` arena owned by the `Manager`. Nodes are referenced by lightweight `NodeId` handles (32-bit integers) rather than pointers. This avoids the complexity of pointer management and plays well with Rust's ownership model.

### Complemented edges

The lowest bit of every `NodeId` serves as a complement flag. Negating a Boolean function is therefore an O(1) bit flip rather than a recursive traversal. This technique, pioneered in BDD packages like CUDD, roughly halves the number of nodes required for many practical functions because `f` and `NOT f` share the same graph structure.

### Computed table caching

Every recursive operation (AND, OR, ITE, quantification, composition, and so on) checks a direct-mapped hash table before recursing. If the same subproblem has been solved before, the cached result is returned immediately. The computed table is lossy (newer entries can evict older ones) to bound memory usage without requiring explicit invalidation.

### Unique tables for canonicity

Each variable level has its own hash table that maps `(then_child, else_child)` pairs to existing nodes. Before allocating a new node, the manager checks the unique table to ensure that every structurally distinct function has exactly one node. This guarantees that two BDDs are equal if and only if they have the same `NodeId`, making equality checking O(1).

### Reference counting with saturation

Nodes track how many external references point to them. When a node's count drops to zero it becomes eligible for garbage collection. The count saturates at `u32::MAX` so that constant nodes and other permanently-live nodes never overflow.

## Feature Highlights

lumindd exposes **402 public API functions** across the `Manager` type, covering:

| Category | Highlights |
|---|---|
| BDD operations | ITE, AND, OR, XOR, NAND, NOR, XNOR, implication, quantification, composition, restrict, constrain, compaction |
| ADD operations | binary apply (12 operators), monadic apply (6 operators), ITE, abstraction, matrix multiply, Walsh/Hadamard, residue |
| ZDD operations | union, intersect, difference, product, weak/strong division, complement, ISOP, BDD conversion |
| Variable reordering | **17 algorithms** -- sifting, symmetric sifting, group sifting, linear sifting, window-2/3/4 (each with convergence), simulated annealing, genetic algorithm, exact DP, random |
| Approximation | 8 BDD approximation methods -- under/over-approximation, heavy-branch subsetting, short-path subsetting, remap, biased, squeeze, compress |
| Serialization | DDDMP-compatible text, binary, and CNF (DIMACS) formats |
| Export | DOT (Graphviz), BLIF, DaVinci, factored form, truth table |
| Counting | Standard `f64`, extended precision (EPD), and arbitrary precision (APA) minterm counting |
| Decomposition | Conjunctive/disjunctive decomposition, equation solving, essential variable extraction |
| Comparison | Equality, inequality, interval, Hamming distance, correlation, agreement |
| Clause extraction | Two-literal clauses, implication pairs |
| Simulation | Bit-parallel simulation, signature-based equivalence checking |
| Safe wrappers | RAII types (`Bdd`, `Add`, `Zdd`) with operator overloading and automatic reference counting |
| Variable grouping | MTR (Multi-way Tree) for constrained reordering |
| Matrix operations | ADD-based matrix multiply, triangle, outer sum, Harwell-Boeing sparse I/O |

## Comparison with CUDD

CUDD (Colorado University Decision Diagram package) is the most widely used C library for decision diagram manipulation. lumindd is directly inspired by CUDD's architecture and aims for API-level familiarity, but differs in several important ways:

| Aspect | CUDD | lumindd |
|---|---|---|
| Language | C (with C++ wrappers) | Pure Rust |
| Memory safety | Manual pointer management | Arena-based, no unsafe pointer arithmetic |
| Complemented edges | Yes | Yes |
| BDD/ADD/ZDD | Yes | Yes |
| Reordering algorithms | 17+ | 17 (full parity) |
| DDDMP serialization | Yes | Yes (compatible format) |
| Thread safety | Not thread-safe | Single-threaded (safe Rust guarantees) |
| Dependencies | None | `bitflags` only |

If you are migrating from CUDD, most function names have a direct counterpart in lumindd. For example, `Cudd_bddAnd` becomes `mgr.bdd_and()`, `Cudd_bddExistAbstract` becomes `mgr.bdd_exist_abstract()`, and `Cudd_ReadSize` becomes `mgr.read_size()`.

## License

lumindd is released under the **BSD-3-Clause** license. See the [LICENSE](https://github.com/Lumees/lumindd/blob/main/LICENSE) file for the full text.

## Links

- **Repository**: <https://github.com/Lumees/lumindd>
- **Crates.io**: <https://crates.io/crates/lumindd>
- **Lumees Lab**: <https://lumeeslab.com>
