# Variable Grouping (MTR)

The Multi-way Tree (MTR) subsystem lets you declare groups of variables that must stay together during dynamic variable reordering. Without grouping, reordering is free to place any variable at any level. With groups, variables within a group can be reordered among themselves, but the group as a whole stays contiguous.

## Motivation

Variable grouping is essential when:

- **Present/next-state variables** in sequential circuits must remain interleaved or grouped to preserve BDD structure.
- **Bit vectors** (e.g., a 32-bit integer encoded across 32 BDD variables) should stay contiguous so word-level operations remain efficient.
- **Interface boundaries** separate input variables from internal state, and mixing them would hurt performance.

## Core Types

### GroupFlags

A bitflag type controlling group behavior:

| Flag | Value | Meaning |
|---|---|---|
| `DEFAULT` | `0x00` | Group can be freely reordered within its parent. |
| `FIXED` | `0x01` | The relative order of variables inside this group is fixed -- reordering will not change it. |
| `TERMINAL` | `0x02` | Leaf group -- no sub-groups may be inserted inside it. |

Flags can be combined: `GroupFlags::FIXED | GroupFlags::TERMINAL`.

### MtrNode

A single node in the group tree, representing a contiguous range of variable levels `[low .. low + size)`.

```rust
pub struct MtrNode {
    pub low: u16,       // lowest variable level
    pub size: u16,      // number of levels in this group
    pub flags: GroupFlags,
    pub children: Vec<MtrNode>,
}
```

Key methods:

| Method | Description |
|---|---|
| `high()` | Returns `low + size` (exclusive upper bound). |
| `contains(level)` | True if `level` falls within this group. |
| `insert_child(low, size, flags)` | Add a sub-group. Absorbed children become grandchildren. |
| `find_group(level)` | Find the innermost group containing `level`. |
| `validate()` | Assert tree invariants (sorted, non-overlapping, contained). |

### MtrTree

The complete group tree with a root node spanning `[0 .. num_vars)`.

```rust
let mut tree = MtrTree::new(8);       // root covers levels 0..8
tree.make_group(0, 4, GroupFlags::DEFAULT);  // group levels 0..4
tree.make_group(4, 4, GroupFlags::FIXED);    // group levels 4..8, fixed order
tree.validate();                              // verify invariants
```

Groups are automatically placed at the correct depth in the tree hierarchy. If a new group fully contains existing children, those children become its grandchildren.

## Creating Groups

### Through the Manager

The most common way to create groups is through the `Manager`:

```rust
use lumindd::Manager;
use lumindd::mtr::GroupFlags;

let mut mgr = Manager::new();
for _ in 0..8 { mgr.bdd_new_var(); }

// Group levels 0-3 (present-state bits) -- can be reordered
mgr.make_bdd_group(0, 4, GroupFlags::DEFAULT);

// Group levels 4-7 (next-state bits) -- fixed order
mgr.make_bdd_group(4, 4, GroupFlags::FIXED);
```

For ZDD variables:

```rust
mgr.make_zdd_group(0, 4, GroupFlags::DEFAULT);
```

### Nested Groups

Groups can be nested to create a hierarchy:

```rust
// Outer group: all 8 variables
mgr.make_bdd_group(0, 8, GroupFlags::DEFAULT);

// Inner groups: present-state and next-state
mgr.make_bdd_group(0, 4, GroupFlags::DEFAULT);
mgr.make_bdd_group(4, 4, GroupFlags::DEFAULT);

// Even deeper: individual byte halves
mgr.make_bdd_group(0, 2, GroupFlags::TERMINAL);
mgr.make_bdd_group(2, 2, GroupFlags::TERMINAL);
```

The tree is automatically organized so that children are contained within their parents.

## Constrained Reordering

Once groups are defined, use `reduce_heap_with_groups()` to run sifting that respects group boundaries:

```rust
mgr.reduce_heap_with_groups();
```

This method:

1. Decomposes the group tree into leaf blocks.
2. For each non-fixed leaf block, sifts variables within the block boundaries.
3. In a second pass, sifts entire groups as atomic units within their parent group.

Variables in `FIXED` groups are never reordered. Bound variables (see below) are also excluded.

## Variable Binding

Individual variables can be excluded from reordering using `bind_var`:

```rust
mgr.bind_var(0);   // variable 0 will not be moved
mgr.bind_var(3);   // variable 3 will not be moved

mgr.reduce_heap_with_groups(); // variables 0 and 3 stay in place

mgr.unbind_var(0);  // variable 0 can be moved again

assert!(mgr.is_var_bound(3));
assert!(!mgr.is_var_bound(0));
```

Binding is orthogonal to grouping. A bound variable within a non-fixed group will simply be skipped during sifting; the other variables in the group will still be reordered around it.

## Use Cases

### Present/Next-State Variable Interleaving

In model checking, state variables come in pairs: present-state (`ps`) and next-state (`ns`). Keeping each pair adjacent improves BDD performance for transition relation operations.

```rust
let num_state_bits = 16;
for i in 0..num_state_bits {
    // Group each present/next pair
    mgr.make_bdd_group((i * 2) as u16, 2, GroupFlags::DEFAULT);
}
```

### Bit-Vector Grouping

When representing a 16-bit integer, keep all 16 bits together:

```rust
// Variables 0-15 represent a 16-bit value
mgr.make_bdd_group(0, 16, GroupFlags::DEFAULT);

// Variables 16-31 represent another 16-bit value
mgr.make_bdd_group(16, 16, GroupFlags::DEFAULT);
```

### Fixed Input/Output Separation

Ensure input variables always appear above output variables:

```rust
mgr.make_bdd_group(0, num_inputs as u16, GroupFlags::DEFAULT);
mgr.make_bdd_group(num_inputs as u16, num_outputs as u16, GroupFlags::DEFAULT);
```

## Inspecting the Group Tree

```rust
if let Some(tree) = mgr.group_tree() {
    tree.validate(); // check invariants

    let root = tree.root();
    println!("Root: [{}, {}), {} children",
             root.low, root.high(), root.children.len());

    // Find which group contains level 5
    let group = tree.find_group(5);
    println!("Level 5 is in group [{}, {})", group.low, group.high());

    // Get the leaf-level sifting blocks
    let blocks = tree.leaf_blocks();
    for (low, size, fixed) in &blocks {
        println!("  Block [{}, {}), fixed={}", low, low + size, fixed);
    }
}
```

## API Reference

| Method | Description |
|---|---|
| `make_bdd_group(low, size, flags)` | Create a BDD variable group |
| `make_zdd_group(low, size, flags)` | Create a ZDD variable group |
| `make_tree_node(low, size, flags)` | Create a group node (returns reference) |
| `set_group_tree(tree)` | Replace the entire group tree |
| `group_tree()` | Get a reference to the current group tree |
| `bind_var(var)` | Prevent a variable from being reordered |
| `unbind_var(var)` | Allow a variable to be reordered again |
| `is_var_bound(var)` | Check if a variable is bound |
| `reduce_heap_with_groups()` | Run group-constrained sifting |
