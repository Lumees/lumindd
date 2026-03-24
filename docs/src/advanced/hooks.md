# Hooks and Callbacks

lumindd provides a hook system for observing manager-level events such as variable reordering and garbage collection. Hooks are useful for progress reporting, statistics collection, logging, and debugging.

## Hook Types

Four hook types are available:

| Hook Type | When It Fires |
|---|---|
| `HookType::PreReorder` | Immediately before variable reordering begins |
| `HookType::PostReorder` | Immediately after variable reordering completes |
| `HookType::PreGarbageCollect` | Immediately before garbage collection begins |
| `HookType::PostGarbageCollect` | Immediately after garbage collection completes |

## HookInfo

When a hook fires, the callback receives a `HookInfo` struct containing a snapshot of the manager state:

```rust
pub struct HookInfo {
    pub hook_type: HookType,  // which event triggered this callback
    pub num_nodes: usize,     // total nodes in the arena
    pub num_vars: u16,        // number of BDD/ADD variables
}
```

## Registering Hooks

Register callbacks using `add_hook` on the `Manager`:

```rust
use lumindd::Manager;
use lumindd::hooks::{HookType, HookInfo};

let mut mgr = Manager::new();

mgr.add_hook(HookType::PreReorder, Box::new(|info: &HookInfo| {
    println!("Reordering starting: {} nodes, {} variables",
             info.num_nodes, info.num_vars);
}));

mgr.add_hook(HookType::PostReorder, Box::new(|info: &HookInfo| {
    println!("Reordering complete: {} nodes", info.num_nodes);
}));
```

### Multiple Callbacks

You can register multiple callbacks for the same hook type. They fire in registration order:

```rust
mgr.add_hook(HookType::PostGarbageCollect, Box::new(|_| {
    println!("GC callback 1");
}));
mgr.add_hook(HookType::PostGarbageCollect, Box::new(|_| {
    println!("GC callback 2");
}));
// When GC occurs: prints "GC callback 1" then "GC callback 2"
```

## Removing Hooks

Remove all callbacks for a specific hook type:

```rust
mgr.remove_hooks(HookType::PreReorder);
```

This removes all callbacks registered for `PreReorder`. Callbacks for other hook types are unaffected.

## Using the HookRegistry Directly

For more control, you can use the `HookRegistry` type directly:

```rust
use lumindd::hooks::{HookRegistry, HookType, HookInfo, HookFn};

let mut registry = HookRegistry::new();

registry.register(HookType::PreReorder, Box::new(|info: &HookInfo| {
    println!("About to reorder: {} nodes", info.num_nodes);
}));

// Check registration status
assert!(registry.has_hooks(HookType::PreReorder));
assert_eq!(registry.hook_count(HookType::PreReorder), 1);

// Fire manually (normally done internally by the Manager)
let info = HookInfo {
    hook_type: HookType::PreReorder,
    num_nodes: 1024,
    num_vars: 8,
};
registry.fire(&info);

// Remove specific hook type
registry.unregister_all(HookType::PreReorder);

// Remove all hooks
registry.clear();
```

## Use Cases

### Progress Reporting

Track how often reordering occurs and its effect on size:

```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

let pre_size = Arc::new(AtomicUsize::new(0));
let pre_clone = pre_size.clone();

mgr.add_hook(HookType::PreReorder, Box::new(move |info: &HookInfo| {
    pre_clone.store(info.num_nodes, Ordering::Relaxed);
}));

let pre_ref = pre_size.clone();
mgr.add_hook(HookType::PostReorder, Box::new(move |info: &HookInfo| {
    let before = pre_ref.load(Ordering::Relaxed);
    let after = info.num_nodes;
    let ratio = after as f64 / before as f64;
    println!("Reordering: {} -> {} nodes ({:.1}% of original)",
             before, after, ratio * 100.0);
}));
```

### Statistics Collection

Count GC events over the lifetime of a computation:

```rust
let gc_count = Arc::new(AtomicUsize::new(0));
let gc_ref = gc_count.clone();

mgr.add_hook(HookType::PostGarbageCollect, Box::new(move |_| {
    gc_ref.fetch_add(1, Ordering::Relaxed);
}));

// ... perform BDD operations ...

println!("Total GC events: {}", gc_count.load(Ordering::Relaxed));
```

### Debugging

Log detailed state when unexpected events occur:

```rust
mgr.add_hook(HookType::PostReorder, Box::new(|info: &HookInfo| {
    if info.num_nodes > 1_000_000 {
        eprintln!("WARNING: BDD still large after reordering: {} nodes",
                  info.num_nodes);
    }
}));
```

## Callback Requirements

Hook callbacks must be `Send` (so that the `Manager` can be moved between threads, even though callbacks are always invoked on the owning thread). Callbacks are invoked synchronously -- the manager operation (reordering, GC) is paused while callbacks execute. Keep callbacks fast to avoid impacting performance.

## API Reference

### Manager Methods

| Method | Description |
|---|---|
| `add_hook(hook_type, callback)` | Register a hook callback |
| `remove_hooks(hook_type)` | Remove all callbacks for a hook type |

### HookRegistry Methods

| Method | Description |
|---|---|
| `new()` | Create an empty registry |
| `register(hook_type, callback)` | Register a callback |
| `unregister_all(hook_type)` | Remove callbacks for a type |
| `fire(info)` | Fire all callbacks for the info's type |
| `has_hooks(hook_type)` | Check if any callbacks are registered |
| `hook_count(hook_type)` | Count registered callbacks |
| `clear()` | Remove all callbacks |
