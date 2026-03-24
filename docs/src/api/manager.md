# Manager API Reference

The `Manager` struct is the central type in lumindd. All BDD, ADD, and ZDD operations are methods on the manager. This page organizes every public method by category.

## Construction

| Method | Signature | Description |
|---|---|---|
| `new` | `fn new() -> Self` | Create a manager with default settings |
| `with_capacity` | `fn with_capacity(num_bdd_vars: u16, num_zdd_vars: u16, cache_log2: u32) -> Self` | Create with pre-allocated variables and cache |

## Variable Creation

| Method | Signature | Description |
|---|---|---|
| `bdd_new_var` | `fn bdd_new_var(&mut self) -> NodeId` | Create a new BDD/ADD variable |
| `bdd_ith_var` | `fn bdd_ith_var(&mut self, i: u16) -> NodeId` | Get or create the i-th BDD variable |
| `zdd_new_var` | `fn zdd_new_var(&mut self) -> NodeId` | Create a new ZDD variable |
| `bdd_new_var_at_level` | `fn bdd_new_var_at_level(&mut self, level: u32) -> NodeId` | Create a variable at a specific level |

## Constants

| Method | Signature | Description |
|---|---|---|
| `one` | `fn one(&self) -> NodeId` | The constant ONE node |
| `zero` | `fn zero(&self) -> NodeId` | The constant ZERO node |
| `is_constant` | `fn is_constant(&self, id: NodeId) -> bool` | Test if a node is a terminal |

## BDD Operations

| Method | Signature | Description |
|---|---|---|
| `bdd_and` | `fn bdd_and(&mut self, f: NodeId, g: NodeId) -> NodeId` | Conjunction |
| `bdd_or` | `fn bdd_or(&mut self, f: NodeId, g: NodeId) -> NodeId` | Disjunction |
| `bdd_xor` | `fn bdd_xor(&mut self, f: NodeId, g: NodeId) -> NodeId` | Exclusive or |
| `bdd_nand` | `fn bdd_nand(&mut self, f: NodeId, g: NodeId) -> NodeId` | Not-and |
| `bdd_nor` | `fn bdd_nor(&mut self, f: NodeId, g: NodeId) -> NodeId` | Not-or |
| `bdd_xnor` | `fn bdd_xnor(&mut self, f: NodeId, g: NodeId) -> NodeId` | Equivalence (XNOR) |
| `bdd_not` | `fn bdd_not(&self, f: NodeId) -> NodeId` | Complement |
| `bdd_ite` | `fn bdd_ite(&mut self, f: NodeId, g: NodeId, h: NodeId) -> NodeId` | If-then-else |
| `bdd_restrict` | `fn bdd_restrict(&mut self, f: NodeId, c: NodeId) -> NodeId` | Restrict (Coudert & Madre) |
| `bdd_constrain` | `fn bdd_constrain(&mut self, f: NodeId, c: NodeId) -> NodeId` | Constrain (generalized cofactor) |
| `bdd_compose` | `fn bdd_compose(&mut self, f: NodeId, g: NodeId, var: u16) -> NodeId` | Substitute g for var in f |
| `bdd_intersect` | `fn bdd_intersect(&mut self, f: NodeId, g: NodeId) -> NodeId` | Intersection (smallest cube containing f AND g) |

## BDD Quantification

| Method | Signature | Description |
|---|---|---|
| `bdd_exist_abstract` | `fn bdd_exist_abstract(&mut self, f: NodeId, cube: NodeId) -> NodeId` | Existential quantification |
| `bdd_univ_abstract` | `fn bdd_univ_abstract(&mut self, f: NodeId, cube: NodeId) -> NodeId` | Universal quantification |
| `bdd_and_abstract` | `fn bdd_and_abstract(&mut self, f: NodeId, g: NodeId, cube: NodeId) -> NodeId` | AND followed by exist |
| `bdd_xor_exist_abstract` | `fn bdd_xor_exist_abstract(&mut self, f: NodeId, g: NodeId, cube: NodeId) -> NodeId` | XOR followed by exist |
| `bdd_boolean_diff` | `fn bdd_boolean_diff(&mut self, f: NodeId, var: u16) -> NodeId` | Boolean difference w.r.t. var |

## BDD Queries

| Method | Signature | Description |
|---|---|---|
| `bdd_count_minterm` | `fn bdd_count_minterm(&self, f: NodeId, num_vars: u32) -> f64` | Count satisfying assignments |
| `bdd_count_minterm_epd` | `fn bdd_count_minterm_epd(&self, f: NodeId, num_vars: u32) -> EpDouble` | Extended precision count |
| `bdd_count_minterm_apa` | `fn bdd_count_minterm_apa(&self, f: NodeId, num_vars: u32) -> ApInt` | Arbitrary precision count |
| `bdd_support` | `fn bdd_support(&self, f: NodeId) -> Vec<u16>` | Support variables |
| `bdd_eval` | `fn bdd_eval(&self, f: NodeId, assignment: &[bool]) -> bool` | Evaluate under an assignment |
| `bdd_essential_vars` | `fn bdd_essential_vars(&self, f: NodeId) -> Vec<u16>` | Essential variables |
| `bdd_cube` | `fn bdd_cube(&mut self, vars: &[u16]) -> NodeId` | Build a cube from variables |

## BDD Extra Operations

| Method | Signature | Description |
|---|---|---|
| `bdd_make_prime` | `fn bdd_make_prime(&mut self, cube: NodeId, f: NodeId) -> NodeId` | Expand cube to a prime implicant |
| `bdd_pick_one_minterm` | `fn bdd_pick_one_minterm(&mut self, f: NodeId, vars: &[u16]) -> NodeId` | Pick one satisfying assignment |
| `bdd_split_set` | `fn bdd_split_set(&mut self, f: NodeId, n: usize) -> Vec<NodeId>` | Partition minterms into n parts |
| `bdd_xeqy` | `fn bdd_xeqy(&mut self, x_vars: &[u16], y_vars: &[u16]) -> NodeId` | BDD for x == y |
| `bdd_xgty` | `fn bdd_xgty(&mut self, x_vars: &[u16], y_vars: &[u16]) -> NodeId` | BDD for x > y |
| `bdd_adj_permute_x` | `fn bdd_adj_permute_x(&mut self, f: NodeId, var: u16) -> NodeId` | Swap var with its adjacent variable in f |
| `bdd_li_compaction` | `fn bdd_li_compaction(&mut self, f: NodeId, c: NodeId) -> NodeId` | Li's compaction method |

## BDD Priority / Comparison

| Method | Signature | Description |
|---|---|---|
| `bdd_inequality` | `fn bdd_inequality(&mut self, n: u32, x: &[u16], y: &[u16]) -> NodeId` | BDD for x > y |
| `bdd_interval` | `fn bdd_interval(&mut self, x: &[u16], lo: u64, hi: u64) -> NodeId` | BDD for lo <= x <= hi |
| `bdd_disequality` | `fn bdd_disequality(&mut self, n: u32, x: &[u16], y: &[u16]) -> NodeId` | BDD for x != y |
| `bdd_hamming_distance` | `fn bdd_hamming_distance(&mut self, f: NodeId, x: &[u16], dist: u32) -> NodeId` | Expand f by Hamming ball |
| `add_hamming` | `fn add_hamming(&mut self, x: &[u16], y: &[u16]) -> NodeId` | ADD of Hamming distance |
| `bdd_dxygtdxz` | `fn bdd_dxygtdxz(&mut self, x: &[u16], y: &[u16], z: &[u16]) -> NodeId` | BDD for d(x,y) > d(x,z) |
| `bdd_dxygtdyz` | `fn bdd_dxygtdyz(&mut self, x: &[u16], y: &[u16], z: &[u16]) -> NodeId` | BDD for d(x,y) > d(y,z) |

## BDD Approximation

| Method | Signature | Description |
|---|---|---|
| `bdd_under_approx` | `fn bdd_under_approx(&mut self, f: NodeId, n: u32, th: u32) -> NodeId` | Underapproximation |
| `bdd_over_approx` | `fn bdd_over_approx(&mut self, f: NodeId, n: u32, th: u32) -> NodeId` | Overapproximation |
| `bdd_subset_heavy_branch` | `fn bdd_subset_heavy_branch(&mut self, f: NodeId, n: u32, th: u32) -> NodeId` | Heavy-branch subset |
| `bdd_superset_heavy_branch` | `fn bdd_superset_heavy_branch(&mut self, f: NodeId, n: u32, th: u32) -> NodeId` | Heavy-branch superset |
| `bdd_subset_short_paths` | `fn bdd_subset_short_paths(&mut self, f: NodeId, n: u32, th: u32) -> NodeId` | Short-path subset |
| `bdd_superset_short_paths` | `fn bdd_superset_short_paths(&mut self, f: NodeId, n: u32, th: u32) -> NodeId` | Short-path superset |
| `bdd_remap_under_approx` | `fn bdd_remap_under_approx(&mut self, f: NodeId, n: u32, th: u32) -> NodeId` | Remap underapproximation |
| `bdd_remap_over_approx` | `fn bdd_remap_over_approx(&mut self, f: NodeId, n: u32, th: u32) -> NodeId` | Remap overapproximation |
| `bdd_biased_under_approx` | `fn bdd_biased_under_approx(&mut self, f: NodeId, n: u32, th: u32, bias: f64) -> NodeId` | Biased underapproximation |
| `bdd_biased_over_approx` | `fn bdd_biased_over_approx(&mut self, f: NodeId, n: u32, th: u32, bias: f64) -> NodeId` | Biased overapproximation |
| `bdd_subset_compress` | `fn bdd_subset_compress(&mut self, f: NodeId, n: u32, th: u32) -> NodeId` | Subset compression |
| `bdd_superset_compress` | `fn bdd_superset_compress(&mut self, f: NodeId, n: u32, th: u32) -> NodeId` | Superset compression |
| `bdd_squeeze` | `fn bdd_squeeze(&mut self, lb: NodeId, ub: NodeId) -> NodeId` | Squeeze between bounds |

## BDD Decomposition

| Method | Signature | Description |
|---|---|---|
| `bdd_conjunctive_decomp` | `fn bdd_conjunctive_decomp(&mut self, f: NodeId) -> (NodeId, NodeId)` | f = g AND h |
| `bdd_disjunctive_decomp` | `fn bdd_disjunctive_decomp(&mut self, f: NodeId) -> (NodeId, NodeId)` | f = g OR h |
| `bdd_iterative_conjunctive_decomp` | `fn bdd_iterative_conjunctive_decomp(&mut self, f: NodeId, max: usize) -> Vec<NodeId>` | Multi-part conjunctive decomposition |
| `bdd_solve_eqn` | `fn bdd_solve_eqn(&mut self, f: NodeId, var: u16) -> (NodeId, NodeId)` | Solve f=0 for var |
| `bdd_verify_sol` | `fn bdd_verify_sol(&mut self, f: NodeId, vars: &[u16], sols: &[NodeId]) -> bool` | Verify equation solutions |
| `bdd_compatible_projection` | `fn bdd_compatible_projection(&mut self, f: NodeId, cube: NodeId) -> NodeId` | Project onto cube variables |

## ADD Operations

| Method | Signature | Description |
|---|---|---|
| `add_const` | `fn add_const(&mut self, val: f64) -> NodeId` | Create a constant ADD |
| `add_apply` | `fn add_apply(&mut self, op: AddOp, f: NodeId, g: NodeId) -> NodeId` | Binary apply |
| `add_monadic_apply` | `fn add_monadic_apply(&mut self, op: AddMonadicOp, f: NodeId) -> NodeId` | Unary apply |
| `add_plus` | `fn add_plus(&mut self, f: NodeId, g: NodeId) -> NodeId` | Addition |
| `add_times` | `fn add_times(&mut self, f: NodeId, g: NodeId) -> NodeId` | Multiplication |
| `add_minus` | `fn add_minus(&mut self, f: NodeId, g: NodeId) -> NodeId` | Subtraction |
| `add_divide` | `fn add_divide(&mut self, f: NodeId, g: NodeId) -> NodeId` | Division |
| `add_min` | `fn add_min(&mut self, f: NodeId, g: NodeId) -> NodeId` | Minimum |
| `add_max` | `fn add_max(&mut self, f: NodeId, g: NodeId) -> NodeId` | Maximum |
| `add_value` | `fn add_value(&self, f: NodeId) -> Option<f64>` | Get terminal value |
| `add_exist_abstract` | `fn add_exist_abstract(&mut self, f: NodeId, cube: NodeId) -> NodeId` | Sum over cube variables |
| `bdd_to_add` | `fn bdd_to_add(&mut self, f: NodeId) -> NodeId` | Convert BDD to 0/1 ADD |
| `add_bdd_strict_threshold` | `fn add_bdd_strict_threshold(&mut self, f: NodeId, th: f64) -> NodeId` | Threshold ADD to BDD |

## ADD Matrix Operations

| Method | Signature | Description |
|---|---|---|
| `add_matrix_multiply` | `fn add_matrix_multiply(&mut self, a: NodeId, b: NodeId, z: &[u16]) -> NodeId` | Matrix multiply |
| `add_times_plus` | `fn add_times_plus(&mut self, a: NodeId, b: NodeId, z: &[u16]) -> NodeId` | Alias for matrix_multiply |
| `add_triangle` | `fn add_triangle(&mut self, a: NodeId, b: NodeId, z: &[u16]) -> NodeId` | Min-plus product (shortest path) |
| `add_outer_sum` | `fn add_outer_sum(&mut self, a: NodeId, b: NodeId) -> NodeId` | Outer sum of vectors |

## ZDD Operations

| Method | Signature | Description |
|---|---|---|
| `zdd_union` | `fn zdd_union(&mut self, f: NodeId, g: NodeId) -> NodeId` | Set-family union |
| `zdd_intersect` | `fn zdd_intersect(&mut self, f: NodeId, g: NodeId) -> NodeId` | Set-family intersection |
| `zdd_diff` | `fn zdd_diff(&mut self, f: NodeId, g: NodeId) -> NodeId` | Set-family difference |
| `zdd_product` | `fn zdd_product(&mut self, f: NodeId, g: NodeId) -> NodeId` | Cross-product |
| `zdd_count` | `fn zdd_count(&self, f: NodeId) -> u64` | Count sets in family |

## Variable Reordering

| Method | Signature | Description |
|---|---|---|
| `reduce_heap` | `fn reduce_heap(&mut self, method: ReorderingMethod)` | Manual reorder (basic) |
| `reduce_heap_ext` | `fn reduce_heap_ext(&mut self, method: ExtReorderMethod)` | Manual reorder (extended) |
| `shuffle_heap` | `fn shuffle_heap(&mut self, perm: &[u32])` | Apply specific permutation |
| `enable_auto_reorder` | `fn enable_auto_reorder(&mut self, method: ReorderingMethod)` | Enable auto-reorder |
| `disable_auto_reorder` | `fn disable_auto_reorder(&mut self)` | Disable auto-reorder |
| `reduce_heap_with_groups` | `fn reduce_heap_with_groups(&mut self)` | Group-constrained sifting |

## Variable Grouping

| Method | Signature | Description |
|---|---|---|
| `make_bdd_group` | `fn make_bdd_group(&mut self, low: u16, size: u16, flags: GroupFlags)` | Create BDD variable group |
| `make_zdd_group` | `fn make_zdd_group(&mut self, low: u16, size: u16, flags: GroupFlags)` | Create ZDD variable group |
| `make_tree_node` | `fn make_tree_node(&mut self, low: u16, size: u16, flags: GroupFlags) -> &MtrNode` | Create group node |
| `set_group_tree` | `fn set_group_tree(&mut self, tree: MtrTree)` | Replace group tree |
| `group_tree` | `fn group_tree(&self) -> Option<&MtrTree>` | Get current group tree |
| `bind_var` | `fn bind_var(&mut self, var: u16)` | Exclude variable from reordering |
| `unbind_var` | `fn unbind_var(&mut self, var: u16)` | Re-enable variable reordering |
| `is_var_bound` | `fn is_var_bound(&self, var: u16) -> bool` | Check if variable is bound |

## Serialization

| Method | Signature | Description |
|---|---|---|
| `dddmp_save_text` | `fn dddmp_save_text<W>(&self, f: NodeId, names: Option<&[&str]>, out: &mut W) -> io::Result<()>` | Save text format |
| `dddmp_load_text` | `fn dddmp_load_text<R>(&mut self, input: &mut R) -> io::Result<NodeId>` | Load text format |
| `dddmp_save_binary` | `fn dddmp_save_binary<W>(&self, f: NodeId, out: &mut W) -> io::Result<()>` | Save binary format |
| `dddmp_load_binary` | `fn dddmp_load_binary<R>(&mut self, input: &mut R) -> io::Result<NodeId>` | Load binary format |
| `dddmp_save_cnf` | `fn dddmp_save_cnf<W>(&self, f: NodeId, out: &mut W) -> io::Result<()>` | Export as DIMACS CNF |

## Export

| Method | Signature | Description |
|---|---|---|
| `dump_dot_color` | `fn dump_dot_color<W>(&self, f: NodeId, highlight: &[NodeId], out: &mut W) -> io::Result<()>` | DOT with highlighting |
| `dump_blif` | `fn dump_blif<W>(&self, f: NodeId, names: Option<&[&str]>, out_name: &str, out: &mut W) -> io::Result<()>` | BLIF export |
| `dump_davinci` | `fn dump_davinci<W>(&self, f: NodeId, out: &mut W) -> io::Result<()>` | DaVinci format |
| `dump_factored_form` | `fn dump_factored_form(&self, f: NodeId) -> String` | Boolean expression string |
| `dump_truth_table` | `fn dump_truth_table<W>(&self, f: NodeId, out: &mut W) -> io::Result<()>` | Full truth table |

## Reference Counting

| Method | Signature | Description |
|---|---|---|
| `ref_node` | `fn ref_node(&mut self, id: NodeId)` | Increment reference count |
| `deref_node` | `fn deref_node(&mut self, id: NodeId)` | Decrement reference count |

## Hooks

| Method | Signature | Description |
|---|---|---|
| `add_hook` | `fn add_hook(&mut self, hook_type: HookType, callback: HookFn)` | Register a hook |
| `remove_hooks` | `fn remove_hooks(&mut self, hook_type: HookType)` | Remove hooks for a type |

## Debug

| Method | Signature | Description |
|---|---|---|
| `debug_check` | `fn debug_check(&self) -> Result<(), String>` | Full invariant check |
| `debug_check_keys` | `fn debug_check_keys(&self) -> Result<(), String>` | Unique table key check |
| `debug_verify_dd` | `fn debug_verify_dd(&self, f: NodeId) -> Result<(), String>` | Verify a single DD |
| `debug_stats` | `fn debug_stats(&self) -> ManagerStats` | Detailed statistics |

## Accessor Functions

| Method | Signature | Description |
|---|---|---|
| `read_size` | `fn read_size(&self) -> u16` | BDD/ADD variable count |
| `read_zdd_size` | `fn read_zdd_size(&self) -> u16` | ZDD variable count |
| `read_node_count` | `fn read_node_count(&self) -> usize` | Total node count |
| `read_peak_node_count` | `fn read_peak_node_count(&self) -> usize` | Peak node count |
| `read_dead` | `fn read_dead(&self) -> usize` | Dead node count |
| `read_live` | `fn read_live(&self) -> usize` | Live node count |
| `read_perm` | `fn read_perm(&self, var: u16) -> u32` | Variable-to-level mapping |
| `read_inv_perm` | `fn read_inv_perm(&self, level: u32) -> u16` | Level-to-variable mapping |
| `read_perm_zdd` | `fn read_perm_zdd(&self, var: u16) -> u32` | ZDD variable-to-level |
| `read_inv_perm_zdd` | `fn read_inv_perm_zdd(&self, level: u32) -> u16` | ZDD level-to-variable |
| `read_var_index` | `fn read_var_index(&self, f: NodeId) -> u16` | Variable index of a node |
| `read_then` | `fn read_then(&self, f: NodeId) -> NodeId` | Then-child |
| `read_else` | `fn read_else(&self, f: NodeId) -> NodeId` | Else-child |
| `read_cache_hits` | `fn read_cache_hits(&self) -> u64` | Cache hit count |
| `read_cache_misses` | `fn read_cache_misses(&self) -> u64` | Cache miss count |
| `read_cache_hit_rate` | `fn read_cache_hit_rate(&self) -> f64` | Cache hit rate |
| `read_reorderings` | `fn read_reorderings(&self) -> u64` | Reorder/GC count |
| `read_reordering_method` | `fn read_reordering_method(&self) -> ReorderingMethod` | Current reorder method |
| `read_memory_in_use` | `fn read_memory_in_use(&self) -> usize` | Memory estimate (bytes) |
| `read_gc_threshold` | `fn read_gc_threshold(&self) -> usize` | GC trigger threshold |
| `read_max_cache_hard` | `fn read_max_cache_hard(&self) -> usize` | Cache size limit |
| `read_max_growth` | `fn read_max_growth(&self) -> f64` | Max growth factor |
| `num_vars` | `fn num_vars(&self) -> u16` | BDD/ADD variable count |
| `num_zdd_vars` | `fn num_zdd_vars(&self) -> u16` | ZDD variable count |
| `num_nodes` | `fn num_nodes(&self) -> usize` | Total node count |
| `cache_stats` | `fn cache_stats(&self) -> (u64, u64)` | (hits, misses) |
| `garbage_collect` | `fn garbage_collect(&mut self)` | Trigger garbage collection |
