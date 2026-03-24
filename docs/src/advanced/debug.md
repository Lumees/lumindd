# Debug and Invariant Checking

lumindd provides a suite of debug utilities for verifying the internal consistency of the manager and individual decision diagrams. These tools are invaluable during development, testing, and diagnosing unexpected behavior.

## Full Consistency Check: debug_check()

`debug_check()` verifies all global invariants of the manager:

```rust
let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();
let _f = mgr.bdd_and(x, y);

match mgr.debug_check() {
    Ok(()) => println!("All invariants OK"),
    Err(msg) => eprintln!("Invariant violation: {}", msg),
}
```

### What It Checks

- **Permutation consistency:** `perm` and `inv_perm` are valid inverses of each other, and all values are in range. Checked for both BDD/ADD and ZDD variable orderings.
- **Unique table integrity:** No unique table has more entries than the total node count.
- **Canonical form:** Every stored internal node has a non-complemented then-child (the fundamental invariant of complemented-edge BDDs).

## Unique Table Key Verification: debug_check_keys()

`debug_check_keys()` verifies that the key counts reported by unique tables are consistent with actual table sizes:

```rust
match mgr.debug_check_keys() {
    Ok(()) => println!("Key counts OK"),
    Err(msg) => eprintln!("Key count mismatch: {}", msg),
}
```

This catches cases where the key counter has gotten out of sync with the actual number of entries (which would indicate a bug in the unique table or garbage collection logic).

## Single DD Verification: debug_verify_dd()

`debug_verify_dd()` performs a deep structural check on a specific decision diagram:

```rust
let f = mgr.bdd_and(x, y);

match mgr.debug_verify_dd(f) {
    Ok(()) => println!("DD {} is valid", f),
    Err(msg) => eprintln!("DD verification failed: {}", msg),
}
```

### What It Checks

- **Valid indices:** All reachable nodes reference valid arena positions.
- **Variable ordering:** For every node, its variable level is strictly less than the levels of its children. This is the core BDD ordering invariant.
- **Acyclicity:** No cycles exist in the DAG (detected via path tracking during DFS).

This method is especially useful after operations that modify the BDD structure, such as reordering or composition.

## Manager Statistics: debug_stats()

`debug_stats()` returns a comprehensive snapshot of the manager state:

```rust
let stats = mgr.debug_stats();

println!("Total nodes:          {}", stats.total_nodes);
println!("Live nodes:           {}", stats.live_nodes);
println!("Dead nodes:           {}", stats.dead_nodes);
println!("Unique table entries: {}", stats.unique_table_entries);
println!("Cache hit rate:       {:.1}%", stats.cache_hit_rate * 100.0);
println!("BDD variables:        {}", stats.num_bdd_vars);
println!("ZDD variables:        {}", stats.num_zdd_vars);
println!("Peak nodes:           {}", stats.peak_nodes);
```

### ManagerStats Fields

| Field | Type | Description |
|---|---|---|
| `total_nodes` | `usize` | Total nodes in the arena (live + dead) |
| `live_nodes` | `usize` | Nodes with positive reference count |
| `dead_nodes` | `usize` | Nodes with zero reference count |
| `unique_table_entries` | `usize` | Sum of entries across all unique tables |
| `cache_entries` | `usize` | Total cache operations (approximation) |
| `cache_hit_rate` | `f64` | Fraction of cache lookups that hit (0.0 to 1.0) |
| `num_bdd_vars` | `u16` | Number of BDD/ADD variables |
| `num_zdd_vars` | `u16` | Number of ZDD variables |
| `peak_nodes` | `usize` | Peak node count (equals total_nodes in non-compacting arena) |

## Manager Accessor Functions

Beyond the debug utilities, the manager provides a rich set of read-only accessor methods for querying its state.

### Size and Count Queries

| Method | Description |
|---|---|
| `read_size()` | Number of BDD/ADD variables |
| `read_zdd_size()` | Number of ZDD variables |
| `read_node_count()` | Total allocated nodes |
| `read_peak_node_count()` | Peak node count |
| `read_dead()` | Dead (unreferenced) node count |
| `read_live()` | Live node count |
| `read_memory_in_use()` | Estimated memory usage in bytes |

### Cache Queries

| Method | Description |
|---|---|
| `read_cache_hits()` | Total cache hits |
| `read_cache_misses()` | Total cache misses |
| `read_cache_hit_rate()` | Hit rate (0.0 to 1.0) |
| `read_cache_used_slots()` | Approximate occupied cache slots |
| `read_max_cache_hard()` | Cache size limit |

### Reordering Queries

| Method | Description |
|---|---|
| `read_reorderings()` | Number of GC/reorder cycles |
| `read_reordering_method()` | Currently configured reordering method |
| `is_auto_reorder_enabled()` | Whether auto-reorder is on |

### Permutation Queries

| Method | Description |
|---|---|
| `read_perm(var)` | Current level of BDD variable `var` |
| `read_inv_perm(level)` | BDD variable at `level` |
| `read_perm_zdd(var)` | Current level of ZDD variable `var` |
| `read_inv_perm_zdd(level)` | ZDD variable at `level` |

### Node Queries

| Method | Description |
|---|---|
| `read_var_index(f)` | Variable index of node `f` |
| `read_then(f)` | Then-child of node `f` |
| `read_else(f)` | Else-child of node `f` |

### Configuration

| Method | Description |
|---|---|
| `set_gc_threshold(n)` | Set dead-node threshold for GC |
| `read_gc_threshold()` | Current GC threshold |
| `set_max_cache_hard(n)` | Set cache size limit |
| `set_max_growth(factor)` | Set max growth factor for reordering |
| `read_max_growth()` | Current max growth factor (default: 1.2) |

## Typical Debug Workflow

```rust
// 1. Build some BDDs
let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
let y = mgr.bdd_new_var();
let f = mgr.bdd_and(x, y);

// 2. Verify global consistency
assert!(mgr.debug_check().is_ok());

// 3. Verify the specific DD
assert!(mgr.debug_verify_dd(f).is_ok());

// 4. Check unique table integrity
assert!(mgr.debug_check_keys().is_ok());

// 5. Inspect statistics
let stats = mgr.debug_stats();
println!("Cache hit rate: {:.1}%", stats.cache_hit_rate * 100.0);
println!("Live/Dead: {}/{}", stats.live_nodes, stats.dead_nodes);
```

## Performance Notes

Debug checks traverse the entire node arena or BDD graph, so they have non-trivial cost. Use them during development and testing, but consider removing them from hot paths in production code. The accessor methods (`read_size`, `read_perm`, etc.) are all O(1) and safe to use anywhere.
