# BDD Approximation

When a BDD grows too large for practical use, approximation methods produce a smaller BDD that is "close" to the original. lumindd provides a comprehensive set of approximation methods, each offering different trade-offs between size reduction, accuracy, and computational cost.

## Fundamental Invariants

All approximation methods in lumindd maintain one of two guarantees:

- **Underapproximation** (subset): `approx` implies `f`. Every satisfying assignment of the approximation is also a satisfying assignment of the original. Formally: `approx AND NOT(f) = ZERO`.

- **Overapproximation** (superset): `f` implies `approx`. Every satisfying assignment of the original is also a satisfying assignment of the approximation. Formally: `f AND NOT(approx) = ZERO`.

These invariants are critical for applications like model checking where soundness must be preserved.

## Methods at a Glance

| Method | Direction | Strategy | Best For |
|---|---|---|---|
| `bdd_under_approx` | Under | General (delegates to heavy-branch) | Quick underapproximation |
| `bdd_over_approx` | Over | General (delegates to heavy-branch) | Quick overapproximation |
| `bdd_subset_heavy_branch` | Under | Keeps high-minterm-count branches | Preserving the most common behaviors |
| `bdd_superset_heavy_branch` | Over | Dual of heavy-branch | Preserving all common behaviors |
| `bdd_subset_short_paths` | Under | Keeps short decision paths | Preserving simple, easy-to-reach behaviors |
| `bdd_superset_short_paths` | Over | Dual of short-paths | Covering all simple behaviors |
| `bdd_remap_under_approx` | Under | Iterative restrict + refine | Better quality than heavy-branch |
| `bdd_remap_over_approx` | Over | Dual of remap-under | Better quality overapproximation |
| `bdd_biased_under_approx` | Under | Weighted branch selection | Control over which branches to keep |
| `bdd_biased_over_approx` | Over | Dual of biased-under | Controlled overapproximation |
| `bdd_subset_compress` | Under | Iterative restrict + subset | Maximum compression |
| `bdd_superset_compress` | Over | Dual of subset-compress | Maximum compression (superset) |
| `bdd_squeeze` | Between | Minimize between bounds | When you have both lower and upper bounds |

## Heavy-Branch Subsetting

The heavy-branch method keeps BDD branches that cover the most satisfying assignments (minterms) and replaces lighter branches with ZERO (for underapproximation) or ONE (for overapproximation).

```rust
let mut mgr = Manager::new();
// ... build BDD f ...

// Underapproximation: result implies f, at most 100 nodes
let under = mgr.bdd_subset_heavy_branch(f, num_vars, 100);

// Overapproximation: f implies result, at most 100 nodes
let over = mgr.bdd_superset_heavy_branch(f, num_vars, 100);
```

**How it works:** At each internal node, the minterm counts of the then-branch and else-branch are compared. The node budget is allocated proportionally to minterm count. If the result is still too large, the lighter branch is pruned entirely.

**When to use:** This is the best general-purpose approximation method. It preserves the most "important" (highest-probability) behaviors of the function.

## Short-Path Subsetting

The short-path method keeps BDD paths that involve the fewest variable decisions (shortest paths from root to terminal ONE) and prunes longer paths.

```rust
let under = mgr.bdd_subset_short_paths(f, num_vars, 100);
let over = mgr.bdd_superset_short_paths(f, num_vars, 100);
```

**When to use:** When the BDD represents a set of configurations and you want to preserve the simplest configurations (those requiring the fewest variable decisions).

## Remap Approximation

The remap method (based on Ravi & Somenzi, ICCAD 1998) iteratively refines an initial heavy-branch approximation using the `restrict` operation with care-set refinement.

```rust
let under = mgr.bdd_remap_under_approx(f, num_vars, 100);
let over = mgr.bdd_remap_over_approx(f, num_vars, 100);
```

**How it works:**
1. Start with a heavy-branch subset as the initial approximation.
2. Compute a care set from the relationship between the approximation and the original.
3. Use `restrict` to simplify the approximation on the care set.
4. Verify the result still satisfies the approximation invariant.
5. Repeat for up to 3 iterations.

**When to use:** When you need higher-quality approximations than heavy-branch provides and can tolerate the extra computation.

## Biased Approximation

The biased method is a generalization of heavy-branch subsetting that uses an explicit bias parameter to control which branches are favored.

```rust
// bias = 0.0 to 1.0
// Higher bias favors the then-branch; lower bias favors the else-branch
let under = mgr.bdd_biased_under_approx(f, num_vars, 100, 0.7);
let over = mgr.bdd_biased_over_approx(f, num_vars, 100, 0.3);
```

**When to use:** When you have domain knowledge about which branches are more important. For example, if positive cofactors represent "normal" system behavior and you want to preserve those, use a high bias.

## Subset/Superset Compression

Compression methods combine restrict and subsetting iteratively for maximum size reduction.

```rust
let under = mgr.bdd_subset_compress(f, num_vars, 50);
let over = mgr.bdd_superset_compress(f, num_vars, 50);
```

**How it works:**
1. Compute a heavy-branch subset.
2. Use the subset as a care set and restrict the original function.
3. Intersect the restricted result with the original to ensure the underapproximation invariant.
4. Repeat until the BDD is within the threshold.

**When to use:** When you need aggressive size reduction and can accept potentially more minterm loss than other methods.

## Squeeze

The squeeze operation finds a BDD *between* a lower bound and an upper bound that uses the minimum number of nodes. This is the most general approximation method, but it requires you to provide both bounds.

```rust
// Precondition: lb implies ub (lb AND NOT(ub) = ZERO)
let result = mgr.bdd_squeeze(lb, ub);

// Postcondition: lb implies result, and result implies ub
```

**How it works:** At each node, the algorithm exploits the gap between the lower and upper bound cofactors. When a cofactor of the upper bound is ONE, the result can freely choose ONE for that branch. When a cofactor of the lower bound is ZERO, the result can freely choose ZERO. This flexibility allows constructing a smaller BDD than either bound alone.

**When to use:** In iterative abstraction-refinement loops where you have both an underapproximation and an overapproximation and want to find a compact representation between them.

## General Usage Pattern

```rust
use lumindd::Manager;

let mut mgr = Manager::new();
// ... build a large BDD f ...

let num_vars = mgr.read_size() as u32;
let threshold = 200; // maximum number of nodes

// Underapproximation: sound for reachability (no false positives)
let under = mgr.bdd_under_approx(f, num_vars, threshold);

// Overapproximation: sound for safety (no false negatives)
let over = mgr.bdd_over_approx(f, num_vars, threshold);

// Verify the invariants
let check_under = mgr.bdd_and(under, f.not());
assert!(check_under.is_zero()); // under implies f

let check_over = mgr.bdd_and(f, over.not());
assert!(check_over.is_zero()); // f implies over
```

## Choosing the Right Method

1. **Start with heavy-branch** (`bdd_subset_heavy_branch` / `bdd_superset_heavy_branch`). It is fast and effective for most use cases.

2. **Use remap** if heavy-branch does not produce compact enough results.

3. **Use squeeze** if you have both bounds and want maximum compression.

4. **Use biased** if you have domain knowledge about which branches matter more.

5. **Use compression** for the most aggressive size reduction at the cost of accuracy.

6. **Use short-path** when path length (complexity of assignments) matters more than minterm count.
