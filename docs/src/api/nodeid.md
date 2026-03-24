# NodeId Reference

`NodeId` is the fundamental handle type used to reference nodes in lumindd's decision diagram arena. Every BDD, ADD, and ZDD node is identified by a `NodeId`.

## What NodeId Represents

A `NodeId` is a 32-bit value that encodes two pieces of information:

- **Raw index** (bits 1-31): The position of the node in the manager's node arena.
- **Complement flag** (bit 0): Whether this edge is complemented (negated).

This encoding is the same as CUDD's complemented-edge representation. The complement bit allows representing the negation of any BDD without allocating a new node -- `NOT(f)` is simply `f` with the complement bit flipped.

## Constants

| Constant | Value | Description |
|---|---|---|
| `NodeId::ONE` | Raw index 0, not complemented | The Boolean constant TRUE / ADD constant 1.0 |
| `NodeId::ZERO` | Raw index 0, complemented | The Boolean constant FALSE (complement of ONE) |

Note that ZERO is not a separate node -- it is the complement of ONE. This means there is only one terminal node in the arena.

## Methods

### Constant Tests

```rust
let one = NodeId::ONE;
let zero = NodeId::ZERO;

assert!(one.is_one());       // true
assert!(zero.is_zero());     // true
assert!(one.is_constant());  // true
assert!(zero.is_constant()); // true
```

| Method | Signature | Description |
|---|---|---|
| `is_one(self)` | `fn is_one(self) -> bool` | True if this is the ONE constant |
| `is_zero(self)` | `fn is_zero(self) -> bool` | True if this is the ZERO constant (complemented ONE) |
| `is_constant(self)` | `fn is_constant(self) -> bool` | True if this is any terminal node |

### Complement Operations

```rust
let x: NodeId = /* some BDD variable */;

let nx = x.not();            // flip complement bit
assert!(nx.is_complemented());

let rx = nx.regular();       // strip complement bit
assert!(!rx.is_complemented());
assert_eq!(rx, x.regular()); // both point to same node

let cx = x.not_cond(true);   // conditional complement
assert_eq!(cx, nx);
let cx2 = x.not_cond(false); // no-op
assert_eq!(cx2, x);
```

| Method | Signature | Description |
|---|---|---|
| `is_complemented(self)` | `fn is_complemented(self) -> bool` | True if the complement bit is set |
| `regular(self)` | `fn regular(self) -> Self` | Strip the complement bit (return the non-complemented version) |
| `not(self)` | `fn not(self) -> Self` | Flip the complement bit |
| `not_cond(self, cond: bool)` | `fn not_cond(self, cond: bool) -> Self` | Flip the complement bit if `cond` is true |

## Complemented Edge Encoding

The complemented-edge representation is a key optimization in BDD packages:

1. **Negation is O(1):** `bdd_not(f)` simply flips a bit -- no new nodes are created.
2. **Memory savings:** `f` and `NOT(f)` share the same graph structure.
3. **Canonical form:** lumindd enforces that the then-child of every stored node is non-complemented. If needed, both children are complemented and the result edge is complemented instead.

### Implications for Users

- Two `NodeId` values are equal (`==`) only if they refer to the same node with the same complement status.
- `f.regular() == g.regular()` tests whether `f` and `g` refer to the same underlying node (possibly with different complementation).
- When traversing a BDD manually, use `regular()` to get the arena index, and check `is_complemented()` to determine the polarity.

## Creating NodeIds

NodeIds are typically obtained from manager methods:

```rust
let mut mgr = Manager::new();
let x = mgr.bdd_new_var();      // NodeId for variable x
let y = mgr.bdd_new_var();      // NodeId for variable y
let f = mgr.bdd_and(x, y);      // NodeId for x AND y
let nf = f.not();                // NodeId for NOT(x AND y)
```

You can also create a NodeId from a raw index (for deserialization or debugging):

```rust
let id = NodeId::from_raw(42, false);  // index 42, not complemented
let comp = NodeId::from_raw(42, true); // index 42, complemented
```

## Usage with Manager

When passing a `NodeId` to manager methods, the manager uses the raw index to look up the node and the complement bit to determine the polarity:

```rust
// These are equivalent:
let result1 = mgr.bdd_and(f, g);
let result2 = mgr.bdd_and(f.not().not(), g); // double negation cancels

assert_eq!(result1, result2);
```

The manager's `then_child` and `else_child` methods automatically handle complement propagation, so if you pass a complemented NodeId, the returned children will be appropriately complemented.
