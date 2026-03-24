# Alphabetical Function Index

Complete index of all public functions and methods in lumindd, organized alphabetically.

## A

| Function | Module | Description |
|---|---|---|
| `add_apply(op, f, g)` | `Manager` | Binary ADD operation with given operator |
| `add_bdd_strict_threshold(f, th)` | `Manager` | Convert ADD to BDD by strict threshold |
| `add_const(val)` | `Manager` | Create a constant ADD terminal |
| `add_const(val)` | `CuddManager` | Create a wrapped ADD constant |
| `add_divide(f, g)` | `Manager` | ADD element-wise division |
| `add_exist_abstract(f, cube)` | `Manager` | Sum ADD over cube variables |
| `add_hamming(x_vars, y_vars)` | `Manager` | ADD representing Hamming distance |
| `add_matrix_multiply(a, b, z)` | `Manager` | ADD matrix multiply (times-plus semiring) |
| `add_max(f, g)` | `Manager` | ADD element-wise maximum |
| `add_min(f, g)` | `Manager` | ADD element-wise minimum |
| `add_minus(f, g)` | `Manager` | ADD element-wise subtraction |
| `add_monadic_apply(op, f)` | `Manager` | Unary ADD operation |
| `add_outer_sum(a, b)` | `Manager` | Outer sum of two ADD vectors |
| `add_plus(f, g)` | `Manager` | ADD element-wise addition |
| `add_times(f, g)` | `Manager` | ADD element-wise multiplication |
| `add_times_plus(a, b, z)` | `Manager` | Alias for `add_matrix_multiply` |
| `add_triangle(a, b, z)` | `Manager` | Min-plus matrix product (shortest path) |
| `add_value(f)` | `Manager` | Get terminal value of an ADD node |
| `add_hook(type, callback)` | `Manager` | Register a hook callback |
| `and(other)` | `Bdd` | Logical AND |
| `apply(op, other)` | `Add` | General binary ADD apply |

## B

| Function | Module | Description |
|---|---|---|
| `bdd_adj_permute_x(f, var)` | `Manager` | Swap var with adjacent variable in f |
| `bdd_and(f, g)` | `Manager` | BDD conjunction |
| `bdd_and_abstract(f, g, cube)` | `Manager` | AND followed by existential quantification |
| `bdd_biased_over_approx(f, n, th, bias)` | `Manager` | Biased overapproximation |
| `bdd_biased_under_approx(f, n, th, bias)` | `Manager` | Biased underapproximation |
| `bdd_boolean_diff(f, var)` | `Manager` | Boolean difference w.r.t. variable |
| `bdd_compatible_projection(f, cube)` | `Manager` | Project f onto cube variables |
| `bdd_compose(f, g, var)` | `Manager` | Substitute g for var in f |
| `bdd_conjunctive_decomp(f)` | `Manager` | Conjunctive decomposition: f = g AND h |
| `bdd_constrain(f, c)` | `Manager` | Generalized cofactor (constrain) |
| `bdd_count_minterm(f, n)` | `Manager` | Count satisfying assignments (f64) |
| `bdd_count_minterm_apa(f, n)` | `Manager` | Count satisfying assignments (arbitrary precision) |
| `bdd_count_minterm_epd(f, n)` | `Manager` | Count satisfying assignments (extended precision) |
| `bdd_cube(vars)` | `Manager` | Build a cube from variable indices |
| `bdd_disequality(n, x, y)` | `Manager` | BDD for x != y |
| `bdd_disjunctive_decomp(f)` | `Manager` | Disjunctive decomposition: f = g OR h |
| `bdd_dxygtdxz(x, y, z)` | `Manager` | BDD for d(x,y) > d(x,z) |
| `bdd_dxygtdyz(x, y, z)` | `Manager` | BDD for d(x,y) > d(y,z) |
| `bdd_essential_vars(f)` | `Manager` | Find essential variables |
| `bdd_eval(f, assignment)` | `Manager` | Evaluate BDD under assignment |
| `bdd_exist_abstract(f, cube)` | `Manager` | Existential quantification |
| `bdd_hamming_distance(f, x, d)` | `Manager` | Expand f by Hamming ball of radius d |
| `bdd_inequality(n, x, y)` | `Manager` | BDD for x > y |
| `bdd_intersect(f, g)` | `Manager` | BDD intersection |
| `bdd_interval(x, lo, hi)` | `Manager` | BDD for lo <= x <= hi |
| `bdd_ite(f, g, h)` | `Manager` | If-then-else |
| `bdd_ith_var(i)` | `Manager` | Get or create i-th BDD variable |
| `bdd_iterative_conjunctive_decomp(f, max)` | `Manager` | Multi-part conjunctive decomposition |
| `bdd_li_compaction(f, c)` | `Manager` | Li's compaction method |
| `bdd_make_prime(cube, f)` | `Manager` | Expand cube to prime implicant of f |
| `bdd_nand(f, g)` | `Manager` | BDD not-and |
| `bdd_new_var()` | `Manager` | Create a new BDD variable |
| `bdd_new_var_at_level(level)` | `Manager` | Create variable at specific level |
| `bdd_nor(f, g)` | `Manager` | BDD not-or |
| `bdd_not(f)` | `Manager` | BDD complement |
| `bdd_one()` | `CuddManager` | Wrapped BDD constant ONE |
| `bdd_or(f, g)` | `Manager` | BDD disjunction |
| `bdd_over_approx(f, n, th)` | `Manager` | General overapproximation |
| `bdd_pick_one_minterm(f, vars)` | `Manager` | Pick one satisfying assignment as a cube |
| `bdd_remap_over_approx(f, n, th)` | `Manager` | Remap-based overapproximation |
| `bdd_remap_under_approx(f, n, th)` | `Manager` | Remap-based underapproximation |
| `bdd_restrict(f, c)` | `Manager` | Restrict (Coudert & Madre) |
| `bdd_solve_eqn(f, var)` | `Manager` | Solve f=0 for variable |
| `bdd_split_set(f, n)` | `Manager` | Split minterms into n parts |
| `bdd_squeeze(lb, ub)` | `Manager` | Find minimal BDD between bounds |
| `bdd_subset_compress(f, n, th)` | `Manager` | Subset compression |
| `bdd_subset_heavy_branch(f, n, th)` | `Manager` | Heavy-branch subset |
| `bdd_subset_short_paths(f, n, th)` | `Manager` | Short-path subset |
| `bdd_superset_compress(f, n, th)` | `Manager` | Superset compression |
| `bdd_superset_heavy_branch(f, n, th)` | `Manager` | Heavy-branch superset |
| `bdd_superset_short_paths(f, n, th)` | `Manager` | Short-path superset |
| `bdd_support(f)` | `Manager` | Get support variable indices |
| `bdd_to_add(f)` | `Manager` | Convert BDD to 0/1 ADD |
| `bdd_under_approx(f, n, th)` | `Manager` | General underapproximation |
| `bdd_univ_abstract(f, cube)` | `Manager` | Universal quantification |
| `bdd_var(i)` | `CuddManager` | Get wrapped i-th BDD variable |
| `bdd_verify_sol(f, vars, sols)` | `Manager` | Verify equation solutions |
| `bdd_xeqy(x, y)` | `Manager` | BDD for x == y |
| `bdd_xgty(x, y)` | `Manager` | BDD for x > y |
| `bdd_xnor(f, g)` | `Manager` | BDD equivalence (XNOR) |
| `bdd_xor(f, g)` | `Manager` | BDD exclusive or |
| `bdd_xor_exist_abstract(f, g, cube)` | `Manager` | XOR followed by existential quantification |
| `bdd_zero()` | `CuddManager` | Wrapped BDD constant ZERO |
| `bind_var(var)` | `Manager` | Exclude variable from reordering |

## C-D

| Function | Module | Description |
|---|---|---|
| `cache_stats()` | `Manager` | Get (hits, misses) tuple |
| `clear()` | `HookRegistry` | Remove all hooks |
| `compose(g, var)` | `Bdd` | Variable substitution |
| `count()` | `Zdd` | Count sets in ZDD family |
| `count_minterm(n)` | `Bdd` | Count satisfying assignments |
| `dddmp_load_binary(input)` | `Manager` | Load BDD from binary format |
| `dddmp_load_text(input)` | `Manager` | Load BDD from text format |
| `dddmp_save_binary(f, out)` | `Manager` | Save BDD in binary format |
| `dddmp_save_cnf(f, out)` | `Manager` | Export BDD as DIMACS CNF |
| `dddmp_save_text(f, names, out)` | `Manager` | Save BDD in text format |
| `debug_check()` | `Manager` | Full invariant verification |
| `debug_check_keys()` | `Manager` | Unique table key check |
| `debug_stats()` | `Manager` | Detailed manager statistics |
| `debug_verify_dd(f)` | `Manager` | Verify a single DD |
| `deref_node(id)` | `Manager` | Decrement reference count |
| `diff(other)` | `Zdd` | Set-family difference |
| `disable_auto_reorder()` | `Manager` | Disable auto variable reordering |
| `divide(other)` | `Add` | ADD division |
| `dump_blif(f, names, out_name, out)` | `Manager` | Export to BLIF format |
| `dump_davinci(f, out)` | `Manager` | Export to DaVinci format |
| `dump_dot_color(f, highlight, out)` | `Manager` | Export to DOT with highlighting |
| `dump_factored_form(f)` | `Manager` | Convert to Boolean expression string |
| `dump_truth_table(f, out)` | `Manager` | Print full truth table |

## E-I

| Function | Module | Description |
|---|---|---|
| `enable_auto_reorder(method)` | `Manager` | Enable auto variable reordering |
| `exist_abstract(cube)` | `Bdd` | Existential quantification |
| `fire(info)` | `HookRegistry` | Fire callbacks for the given hook type |
| `from_manager(mgr)` | `CuddManager` | Wrap an existing Manager |
| `garbage_collect()` | `Manager` | Trigger garbage collection |
| `group_tree()` | `Manager` | Get the current group tree |
| `has_hooks(type)` | `HookRegistry` | Check if hooks exist for a type |
| `hook_count(type)` | `HookRegistry` | Count hooks for a type |
| `intersect(other)` | `Zdd` | Set-family intersection |
| `is_auto_reorder_enabled()` | `Manager` | Check auto-reorder status |
| `is_constant(id)` | `Manager` | Test if node is terminal |
| `is_var_bound(var)` | `Manager` | Check if variable is bound |
| `ite(then, else)` | `Bdd` | If-then-else |

## M-O

| Function | Module | Description |
|---|---|---|
| `make_bdd_group(low, size, flags)` | `Manager` | Create BDD variable group |
| `make_tree_node(low, size, flags)` | `Manager` | Create group tree node |
| `make_zdd_group(low, size, flags)` | `Manager` | Create ZDD variable group |
| `maximum(other)` | `Add` | Element-wise maximum |
| `minimum(other)` | `Add` | Element-wise minimum |
| `minus(other)` | `Add` | ADD subtraction |
| `monadic_apply(op)` | `Add` | Unary ADD apply |
| `negate()` | `Add` | ADD negation |
| `new()` | `Manager` | Create empty manager |
| `new()` | `CuddManager` | Create managed context |
| `new()` | `HookRegistry` | Create empty hook registry |
| `node_id()` | `Bdd`/`Add`/`Zdd` | Get underlying NodeId |
| `not()` | `Bdd` | Logical NOT |
| `num_nodes()` | `Manager` | Total node count |
| `num_vars()` | `Manager`/`CuddManager` | BDD/ADD variable count |
| `num_zdd_vars()` | `Manager` | ZDD variable count |
| `one()` | `Manager` | Constant ONE NodeId |
| `or(other)` | `Bdd` | Logical OR |

## P-R

| Function | Module | Description |
|---|---|---|
| `plus(other)` | `Add` | ADD addition |
| `product(other)` | `Zdd` | ZDD cross-product |
| `read_cache_hit_rate()` | `Manager` | Cache hit rate |
| `read_cache_hits()` | `Manager` | Cache hit count |
| `read_cache_misses()` | `Manager` | Cache miss count |
| `read_cache_used_slots()` | `Manager` | Approximate cache slot usage |
| `read_dead()` | `Manager` | Dead node count |
| `read_else(f)` | `Manager` | Else-child of node |
| `read_gc_threshold()` | `Manager` | GC trigger threshold |
| `read_inv_perm(level)` | `Manager` | BDD level-to-variable |
| `read_inv_perm_zdd(level)` | `Manager` | ZDD level-to-variable |
| `read_live()` | `Manager` | Live node count |
| `read_max_cache_hard()` | `Manager` | Cache size limit |
| `read_max_growth()` | `Manager` | Max reorder growth factor |
| `read_memory_in_use()` | `Manager` | Memory estimate (bytes) |
| `read_node_count()` | `Manager` | Total node count |
| `read_peak_node_count()` | `Manager` | Peak node count |
| `read_perm(var)` | `Manager` | BDD variable-to-level |
| `read_perm_zdd(var)` | `Manager` | ZDD variable-to-level |
| `read_reordering_method()` | `Manager` | Current reorder method |
| `read_reorderings()` | `Manager` | Reorder/GC count |
| `read_size()` | `Manager` | BDD/ADD variable count |
| `read_then(f)` | `Manager` | Then-child of node |
| `read_var_index(f)` | `Manager` | Variable index of node |
| `read_zdd_size()` | `Manager` | ZDD variable count |
| `reduce_heap(method)` | `Manager` | Manual reorder (basic methods) |
| `reduce_heap_ext(method)` | `Manager` | Manual reorder (extended methods) |
| `reduce_heap_with_groups()` | `Manager` | Group-constrained sifting |
| `ref_node(id)` | `Manager` | Increment reference count |
| `register(type, callback)` | `HookRegistry` | Register a hook callback |
| `remove_hooks(type)` | `Manager` | Remove hooks for a type |

## S-Z

| Function | Module | Description |
|---|---|---|
| `set_gc_threshold(n)` | `Manager` | Set GC trigger threshold |
| `set_group_tree(tree)` | `Manager` | Replace group tree |
| `set_max_cache_hard(n)` | `Manager` | Set cache size limit |
| `set_max_growth(factor)` | `Manager` | Set max reorder growth |
| `shuffle_heap(perm)` | `Manager` | Apply specific variable permutation |
| `support()` | `Bdd` | Support variable set |
| `times(other)` | `Add` | ADD multiplication |
| `unbind_var(var)` | `Manager` | Re-enable variable reordering |
| `union(other)` | `Zdd` | Set-family union |
| `unregister_all(type)` | `HookRegistry` | Remove all hooks for a type |
| `value()` | `Add` | Get terminal value |
| `with_capacity(bdd, zdd, cache)` | `Manager` | Create manager with pre-allocation |
| `xor(other)` | `Bdd` | Logical XOR |
| `zdd_count(f)` | `Manager` | Count sets in ZDD family |
| `zdd_diff(f, g)` | `Manager` | ZDD set-family difference |
| `zdd_empty()` | `CuddManager` | Wrapped empty ZDD family |
| `zdd_base()` | `CuddManager` | Wrapped ZDD base family |
| `zdd_intersect(f, g)` | `Manager` | ZDD set-family intersection |
| `zdd_new_var()` | `Manager` | Create a new ZDD variable |
| `zdd_product(f, g)` | `Manager` | ZDD cross-product |
| `zdd_union(f, g)` | `Manager` | ZDD set-family union |
| `zdd_var(i)` | `CuddManager` | Get wrapped i-th ZDD variable |
| `zero()` | `Manager` | Constant ZERO NodeId |
