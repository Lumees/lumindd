# Core Concepts

This chapter explains the fundamental data structures and ideas behind lumindd. Understanding these concepts will help you use the library effectively and reason about performance.

## Decision Diagrams

A decision diagram is a rooted directed acyclic graph (DAG) that represents a function by branching on input variables. There are three variants in lumindd:

### BDD (Binary Decision Diagram)

A BDD represents a Boolean function `f : {0,1}^n -> {0,1}`. Each internal node tests a variable and has two outgoing edges: the **then-edge** (variable is 1) and the **else-edge** (variable is 0). Terminal nodes are **1** (true) or **0** (false).

```text
        x0
       /  \
     x1    x1
    / \   / \
   1   0 0   0
```

This diagram represents `x0 AND x1`. Following the then-edges (left) for both variables leads to the terminal 1; all other paths lead to 0.

Two rules make BDDs canonical (unique for each function):

1. **No redundant nodes**: if both children of a node are identical, the node is eliminated.
2. **No duplicate nodes**: structurally identical subgraphs are shared via the unique table.

With these rules, two Boolean functions are equal if and only if their BDD root nodes are identical. This makes equivalence checking O(1).

### ADD (Algebraic Decision Diagram)

An ADD represents a function `f : {0,1}^n -> R` where terminals hold arbitrary real numbers (stored as `f64`). The structure is the same as a BDD except that terminal nodes can have any floating-point value, not just 0 and 1.

```text
        x0
       /  \
     x1    3.0
    / \
  2.0  0.0
```

This ADD maps `(x0=1, x1=1)` to 2.0, `(x0=1, x1=0)` to 0.0, and `(x0=0, *)` to 3.0.

ADDs do **not** use complemented edges. They are used for probability computations, cost functions, matrix representations, and any domain where the function range extends beyond Boolean values.

### ZDD (Zero-suppressed Decision Diagram)

A ZDD represents a family of sets (a collection of subsets of a universe). The key difference from a BDD is the **zero-suppressed reduction rule**: a node is eliminated when its **then-child** (not its else-child) is 0. This makes ZDDs naturally compact for sparse set families where most elements are absent from most sets.

```text
     Standard BDD reduction:         ZDD reduction:
     skip if then == else            skip if then == 0
```

The terminal **1** in a ZDD represents the family containing only the empty set `{{}}`. The terminal **0** represents the empty family `{}`.

ZDDs are the right choice when you are working with:

- Combinatorial enumeration (e.g., all independent sets of a graph)
- Sparse Boolean function covers
- Set families where the universe is large but individual sets are small

## The Manager

The `Manager` is the central object that owns all nodes and provides every operation:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
```

Internally, the manager contains:

- **Node arena** (`Vec<DdNode>`) -- a flat vector of all allocated nodes. Nodes are never moved or compacted; their index is stable for the lifetime of the manager.
- **Unique tables** -- one hash table per variable level, ensuring canonical node sharing.
- **Computed table** -- a direct-mapped cache of operation results.
- **Permutation arrays** (`perm` and `inv_perm`) -- mapping between variable indices and their current levels in the ordering.

All BDD, ADD, and ZDD operations are methods on `Manager`. There is no global state.

## NodeId and Complemented Edges

Every node in the arena is referenced by a `NodeId`, which is a 32-bit integer with the following encoding:

```text
  Bits 31..1:  raw arena index (up to 2^31 - 1 nodes)
  Bit 0:       complement flag
```

The two constant nodes are:

| Name | NodeId | Meaning |
|---|---|---|
| `NodeId::ONE`  | `0b00...00` | The Boolean constant true (arena index 0, not complemented) |
| `NodeId::ZERO` | `0b00...01` | The Boolean constant false (arena index 0, complemented) |

Since ZERO is simply ONE with the complement bit flipped, negation is a single XOR operation:

```rust
use lumindd::NodeId;

let one = NodeId::ONE;
let zero = one.not();  // flips bit 0
assert!(zero.is_zero());
assert_eq!(zero, NodeId::ZERO);
```

This means `bdd_not` never allocates a node and runs in O(1) time. It also means that a function and its negation always share the exact same DAG structure, roughly halving memory usage for many practical applications.

### Canonical form convention

To keep complemented edges canonical, lumindd enforces the rule that the **then-child of a stored node is never complemented**. If an operation would produce a node with a complemented then-child, both children are complemented and the result edge is complemented instead. This ensures a unique representation.

## Unique Tables and Canonicity

Each variable level has a **unique table** -- a hash table mapping `(then_child, else_child)` pairs to existing nodes. Before allocating a new node, the manager looks up this table:

1. If a node with the same children already exists, its index is returned (sharing).
2. If the then-child equals the else-child, the node is redundant and the child is returned directly (reduction).
3. Otherwise, a new node is allocated and inserted into the table.

This guarantees that every distinct Boolean function has exactly one canonical representation in the manager. Two `NodeId` values represent the same function if and only if they are equal (accounting for the complement bit).

## Computed Table (Operation Caching)

Recursive operations like AND, OR, ITE, and quantification can visit the same subproblem many times during a single top-level call. The **computed table** (also called the operation cache or memo table) stores results of previous computations.

The table is **direct-mapped** (each entry is indexed by a hash of the operation key) and **lossy** (a new entry overwrites whatever was in that slot). This design:

- Requires no explicit invalidation or reference counting of cache entries.
- Bounds memory usage to a fixed number of slots (configurable via `with_capacity`).
- Achieves very fast lookup with a single hash computation and one comparison.
- Trades a small number of redundant recomputations for simplicity and low overhead.

The cache is automatically cleared when variable reordering occurs, since node relationships change.

You can query cache statistics:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
// ... perform operations ...
let (hits, misses) = mgr.cache_stats();
println!("Cache hit rate: {:.1}%", 100.0 * hits as f64 / (hits + misses) as f64);
```

## Reference Counting

Nodes in the arena are garbage-collected based on reference counts. Every `NodeId` that is "live" (held by your application) must have its node's reference count incremented, and decremented when no longer needed.

In the low-level API (`Manager` directly), you manage references explicitly:

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();

let f = mgr.bdd_and(x, y);
mgr.ref_node(f);   // protect f from garbage collection

// ... use f ...

mgr.deref_node(f);  // allow f to be collected
```

Variables returned by `bdd_new_var` and `bdd_ith_var` are automatically referenced. Intermediate results from operations like `bdd_and` are **not** automatically referenced -- you must call `ref_node` if you need to keep them alive across other operations that might trigger garbage collection.

For a safer interface, use the RAII wrapper types (`Bdd`, `Add`, `Zdd`) from the `wrapper` module, which handle reference counting automatically through `Clone` and `Drop`.

## Variable Ordering and Levels

The efficiency of a BDD is highly sensitive to the order in which variables appear from root to terminals. A function that requires exponential nodes under one ordering may be polynomial under another.

lumindd distinguishes between two concepts:

- **Variable index** -- a permanent identifier assigned when the variable is created (0, 1, 2, ...). This never changes.
- **Level** -- the variable's current position in the ordering (0 = top/root, increasing toward terminals). This can change when reordering occurs.

The mapping between them is stored in two arrays:

```text
perm[var_index]  = level      (variable -> position)
inv_perm[level]  = var_index  (position -> variable)
```

Initially, `perm[i] = i` (variable i is at level i). After reordering, the two arrays diverge. You can query the current mapping:

```rust
use lumindd::Manager;

let mut mgr = Manager::with_capacity(4, 0, 18);

// Initially, variable 2 is at level 2
assert_eq!(mgr.read_perm(2), 2);
assert_eq!(mgr.read_inv_perm(2), 2);
```

### Dynamic reordering

lumindd supports automatic and manual variable reordering. Automatic reordering triggers during operations when the node count exceeds a threshold:

```rust
use lumindd::{Manager, ReorderingMethod};

let mut mgr = Manager::new();
mgr.enable_auto_reorder(ReorderingMethod::Sift);
```

Manual reordering can be triggered at any time:

```rust
use lumindd::{Manager, ReorderingMethod};

let mut mgr = Manager::new();
// ... build some BDDs ...
mgr.reduce_heap(ReorderingMethod::Sift);
```

For the full set of 17 reordering algorithms, use `ExtReorderMethod` with `reduce_heap_ext`.

### Variable groups

You can constrain reordering by grouping variables together using MTR (Multi-way Tree) groups. Variables within a group stay contiguous during reordering. See the [Variable Grouping](../advanced/mtr.md) chapter for details.
