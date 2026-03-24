# Variable Reordering

Variable ordering is the single most important factor affecting BDD size. The same Boolean function can require millions of nodes under one ordering and just a handful under another. The difference is often exponential: an n-bit adder represented with interleaved input orderings needs O(n) nodes, but with a poor ordering it can require O(2^n).

lumindd provides a complete suite of reordering algorithms -- from fast heuristics to exact methods -- giving you control over this critical performance lever.

## Why Variable Ordering Matters

A BDD encodes a Boolean function as a directed acyclic graph where each internal node tests one variable. The order in which variables are tested determines how much sharing is possible between subgraphs. A good ordering maximizes sharing; a bad ordering creates an explosion of distinct subgraphs.

**Rule of thumb:** variables that interact heavily in the function (appear together in clauses, gates, or constraints) should be placed near each other in the ordering.

## Automatic Reordering

The simplest way to manage variable ordering is to let lumindd handle it automatically. When enabled, the manager triggers reordering whenever the BDD grows beyond an internal threshold.

```rust
use lumindd::Manager;
use lumindd::reorder::ReorderingMethod;

let mut mgr = Manager::new();

// Enable automatic reordering with Sift (the best general-purpose method)
mgr.enable_auto_reorder(ReorderingMethod::Sift);

// ... build BDDs as usual -- reordering happens transparently ...

// Disable auto-reorder when you want stable node IDs
mgr.disable_auto_reorder();
```

### Checking Status

```rust
let enabled = mgr.is_auto_reorder_enabled();
let method = mgr.read_reordering_method();
```

## Manual Reordering

You can trigger reordering explicitly at any point.

### reduce_heap

Applies a reordering algorithm to minimize total BDD size:

```rust
use lumindd::reorder::ReorderingMethod;

mgr.reduce_heap(ReorderingMethod::Sift);
```

### reduce_heap_ext

Accesses the full set of 19 algorithms via the `ExtReorderMethod` enum:

```rust
use lumindd::reorder_dispatch::ExtReorderMethod;

mgr.reduce_heap_ext(ExtReorderMethod::SymmSift);
mgr.reduce_heap_ext(ExtReorderMethod::LinearConverge);
```

### shuffle_heap

Applies a specific variable permutation directly:

```rust
// Reverse the ordering of 4 variables
mgr.shuffle_heap(&[3, 2, 1, 0]);
```

`permutation[i]` specifies the new level for variable index `i`. The slice must be a valid permutation (no duplicates, all values in range).

## Reordering Algorithms

### ReorderingMethod (basic enum)

| Variant | Description |
|---|---|
| `None` | No-op. |
| `Sift` | Rudell's sifting. Best general-purpose method. |
| `SiftConverge` | Repeats sifting until no further improvement. |
| `Window2` | Tries all orderings within sliding windows of 2 variables. |
| `Window3` | Tries all orderings within sliding windows of 3 variables. |
| `Random` | Random permutation. Useful for benchmarking baselines. |

### ExtReorderMethod (full enum)

The extended enum provides access to all 19 algorithms:

| Variant | Description | When to Use |
|---|---|---|
| `None` | No reordering. | Explicit no-op. |
| `Sift` | Rudell's sifting -- moves each variable to its best position. | **Default choice.** Fast, effective, works well on most problems. |
| `SiftConverge` | Sifting repeated until no improvement. | When you want a better result and can afford 2-5x more time. |
| `SymmSift` | Exploits variable symmetry during sifting. | Problems with symmetric variable pairs (e.g., adders). |
| `SymmSiftConverge` | Symmetric sifting repeated until convergence. | Best for symmetric problems when time allows. |
| `GroupSift` | Sifts groups of variables as units. | Use with variable groups (see [Variable Grouping](../advanced/mtr.md)). |
| `GroupSiftConverge` | Group sifting repeated until convergence. | Best group-aware reordering. |
| `Window2` | Optimal permutation in windows of 2 adjacent variables. | Fast, low overhead, good for fine-tuning. |
| `Window3` | Optimal permutation in windows of 3. | Better than Window2, slightly slower. |
| `Window4` | Optimal permutation in windows of 4. | Best window method; tries 24 permutations per window. |
| `Window2Converge` | Window2 repeated until convergence. | Good cheap refinement pass. |
| `Window3Converge` | Window3 repeated until convergence. | Moderate refinement. |
| `Window4Converge` | Window4 repeated until convergence. | Thorough window-based refinement. |
| `Linear` | Sifting combined with XOR linear transforms. | Can find orderings that plain sifting misses. |
| `LinearConverge` | Linear sifting repeated until convergence. | Most thorough linear sifting. |
| `Annealing` | Simulated annealing over the permutation space. | Escapes local minima; useful when sifting gets stuck. |
| `Genetic` | Genetic algorithm search over permutations. | Large search spaces; population-based exploration. |
| `Exact` | Exhaustive search for the optimal ordering. | **Only feasible for small BDDs** (up to ~15-20 variables). |
| `Random` | Random permutation. | Benchmarking and testing. |

### Choosing an Algorithm

For most applications, start with **Sift** or **SiftConverge**. These provide the best balance of speed and quality:

```rust
// Good default
mgr.reduce_heap_ext(ExtReorderMethod::Sift);

// When you need better results and have time
mgr.reduce_heap_ext(ExtReorderMethod::SiftConverge);
```

Use **SymmSift** when your problem has natural variable symmetries (e.g., both inputs to an XOR gate are interchangeable).

Use **Exact** only for debugging or small problems -- it has factorial complexity.

Use **Window** methods as a quick polishing pass after sifting, or when you need low-overhead incremental improvement.

Use **Annealing** or **Genetic** when sifting repeatedly fails to find good orderings and you can afford a longer search.

## Variable Interaction Matrix

For problems where variable dependencies are known in advance, building a variable interaction matrix can significantly speed up sifting. The matrix tells the reorder algorithm which variable pairs actually interact, so it can skip swaps that would have no effect.

lumindd's sifting implementation sorts variables by subtable size (largest first), ensuring the most impactful variables are sifted first.

## Performance Tips

1. **Enable auto-reorder early.** Turn it on before building large BDDs so the manager can keep sizes under control as you build.

2. **Use a good initial ordering.** Even with dynamic reordering, starting from a reasonable ordering (e.g., grouping related variables) saves significant reordering time.

3. **Disable reordering during timing-critical phases.** Reordering invalidates the computed-table cache, so disable it during sequences of operations that should not be interrupted.

4. **Combine methods.** Run `Sift` first, then polish with `Window4Converge`:

   ```rust
   mgr.reduce_heap_ext(ExtReorderMethod::Sift);
   mgr.reduce_heap_ext(ExtReorderMethod::Window4Converge);
   ```

5. **Use variable groups** to prevent reordering from separating variables that must stay together (see [Variable Grouping](../advanced/mtr.md)).

6. **Bind critical variables** with `bind_var()` to prevent them from being moved.

7. **Monitor reordering** with hooks to track when and how much reordering helps:

   ```rust
   use lumindd::hooks::{HookType, HookInfo};

   mgr.add_hook(HookType::PreReorder, Box::new(|info: &HookInfo| {
       println!("Reordering: {} nodes, {} vars", info.num_nodes, info.num_vars);
   }));
   ```
