# ADD Matrix Operations

Algebraic Decision Diagrams (ADDs) can compactly represent matrices by encoding row and column indices as Boolean variables. lumindd provides matrix operations over this representation, enabling symbolic linear algebra on potentially huge matrices.

## Representing Matrices as ADDs

A matrix `M[i][j]` is encoded as an ADD over two sets of Boolean variables: row variables and column variables. For an `n x n` matrix, you need `log2(n)` row variables and `log2(n)` column variables.

For a 2x2 matrix with row variable `r` and column variable `c`:

| r | c | M[r][c] |
|---|---|---|
| 0 | 0 | M[0][0] |
| 0 | 1 | M[0][1] |
| 1 | 0 | M[1][0] |
| 1 | 1 | M[1][1] |

The ADD branches on `r` and `c`, with terminal nodes holding the matrix values.

## Matrix Multiplication

`add_matrix_multiply` computes the standard matrix product:

**C(x,y) = sum over z of A(x,z) * B(z,y)**

where `z_vars` specifies the variables encoding the shared (inner) dimension.

```rust
let mut mgr = Manager::new();
let _row = mgr.bdd_new_var();    // variable 0: row index
let _shared = mgr.bdd_new_var(); // variable 1: shared index
let _col = mgr.bdd_new_var();    // variable 2: column index

// Build 2x2 matrices as ADDs
// A = [[1, 2], [3, 4]]  over (row=var 0, shared=var 1)
// B = [[5, 6], [7, 8]]  over (shared=var 1, col=var 2)

// ... (construct ADD nodes for A and B) ...

// C = A * B, summing over the shared variable (var 1)
let c = mgr.add_matrix_multiply(a, b, &[1]);

// Result: C = [[19, 22], [43, 50]]
```

### Implementation

The multiplication is performed in two steps:
1. **Element-wise product**: `add_times(A, B)` computes A(x,z) * B(z,y)
2. **Existential abstraction**: `add_exist_abstract(product, z_cube)` sums over the z variables

The alias `add_times_plus` is also available and calls the same implementation.

## Shortest Path (Triangle Operation)

`add_triangle` computes the min-plus (tropical semiring) matrix product:

**C(x,y) = min over z of (A(x,z) + B(z,y))**

This is the fundamental operation for all-pairs shortest path algorithms (e.g., Floyd-Warshall).

```rust
// Distance matrices
// A = [[0, 3], [7, 1]]
// B = [[0, 2], [5, 0]]

let c = mgr.add_triangle(a, b, &[1]);

// Result: C[0][0] = min(0+0, 3+5) = 0
//         C[0][1] = min(0+2, 3+0) = 2
//         C[1][0] = min(7+0, 1+5) = 6
//         C[1][1] = min(7+2, 1+0) = 1
```

### Implementation

1. **Element-wise sum**: `add_plus(A, B)` computes A(x,z) + B(z,y)
2. **Min-abstraction**: Takes the minimum over z variables (rather than the sum used in standard multiplication)

### Floyd-Warshall with ADDs

To compute all-pairs shortest paths on a graph:

```rust
let mut dist = adjacency_matrix_add; // initial distance ADD
for iteration in 0..log2_n {
    dist = mgr.add_triangle(dist, dist, &z_vars);
}
```

## Outer Sum

`add_outer_sum` computes the outer sum of two vectors:

**result(i,j) = a(i) + b(j)**

where `a` depends only on row variables and `b` depends only on column variables.

```rust
// a depends on var 0: a(0)=1, a(1)=3
// b depends on var 1: b(0)=10, b(1)=20

let result = mgr.add_outer_sum(a, b);

// result = [[11, 21], [13, 23]]
```

Since `a` and `b` depend on disjoint variable sets, the outer sum is simply the ADD pointwise sum.

## Building Matrices

To construct a matrix ADD manually, build it bottom-up: create terminal constants, then compose them into column-level nodes, then row-level nodes.

```rust
fn build_2x2(mgr: &mut Manager, rv: u16, cv: u16,
             m00: f64, m01: f64, m10: f64, m11: f64) -> NodeId {
    let c00 = mgr.add_const(m00);
    let c01 = mgr.add_const(m01);
    let c10 = mgr.add_const(m10);
    let c11 = mgr.add_const(m11);

    // Column nodes (lower variable)
    let row0 = mgr.add_unique_inter(cv, c01, c00); // cv=1 -> m01, cv=0 -> m00
    let row1 = mgr.add_unique_inter(cv, c11, c10);

    // Row node (upper variable)
    mgr.add_unique_inter(rv, row1, row0) // rv=1 -> row1, rv=0 -> row0
}
```

## Extracting Values

To read specific matrix entries, use cofactoring:

```rust
// Get M[1][0] from a 2x2 matrix over (row_var, col_var)
let (row1, row0) = mgr.add_cofactors(matrix, row_var);
let (_, entry_10) = mgr.add_cofactors(row1, col_var);
let value = mgr.add_value(entry_10).unwrap();
```

## Performance Considerations

ADD matrix operations are particularly effective when:

- The matrix is **sparse** or has **regular structure** (many repeated values)
- The matrix dimension is a power of 2
- Operations are applied to the symbolic representation rather than expanded

For dense matrices with no repeated structure, ADD representations may not provide benefits over explicit arrays.

## API Reference

| Method | Description |
|---|---|
| `add_matrix_multiply(a, b, z_vars)` | Matrix multiply: sum_z A(x,z) * B(z,y) |
| `add_times_plus(a, b, z_vars)` | Alias for `add_matrix_multiply` |
| `add_triangle(a, b, z_vars)` | Shortest path: min_z (A(x,z) + B(z,y)) |
| `add_outer_sum(a, b)` | Outer sum: result(i,j) = a(i) + b(j) |
