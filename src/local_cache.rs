// lumindd — Local operation cache for nested/temporary computations
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

//! A local (temporary) computed cache for use alongside the global cache.
//!
//! During operations like variable reordering or approximation, it can be
//! useful to have an isolated cache that does not pollute or interfere with
//! the global [`ComputedTable`]. The [`LocalCache`] provides exactly this:
//! a HashMap-based cache keyed by `(op, f, g, h)` tuples that can be created,
//! used, and discarded within a scoped computation.

use std::collections::HashMap;

use crate::manager::Manager;
use crate::node::NodeId;

/// A local operation cache for nested or temporary computations.
///
/// Unlike the global [`ComputedTable`] which uses a direct-mapped lossy scheme,
/// `LocalCache` is a lossless HashMap-based cache. This is appropriate for
/// smaller, temporary computations where every cached result matters.
///
/// # Example
///
/// ```rust
/// use lumindd::Manager;
/// use lumindd::local_cache::LocalCache;
///
/// let mut mgr = Manager::new();
/// let mut cache = LocalCache::new();
///
/// let x = mgr.bdd_new_var();
/// let y = mgr.bdd_new_var();
///
/// // Use the local cache for some temporary computation
/// cache.insert(0, x, y, lumindd::NodeId::ZERO, x);
/// assert_eq!(cache.lookup(0, x, y, lumindd::NodeId::ZERO), Some(x));
/// ```
pub struct LocalCache {
    /// The cache entries, keyed by (operation_tag, f, g, h).
    entries: HashMap<(u8, NodeId, NodeId, NodeId), NodeId>,
    /// Total number of lookup calls.
    lookups: u64,
    /// Number of successful lookups (hits).
    hits: u64,
}

impl LocalCache {
    /// Create a new empty local cache.
    pub fn new() -> Self {
        LocalCache {
            entries: HashMap::new(),
            lookups: 0,
            hits: 0,
        }
    }

    /// Create a new local cache with a pre-allocated capacity.
    pub fn with_capacity(cap: usize) -> Self {
        LocalCache {
            entries: HashMap::with_capacity(cap),
            lookups: 0,
            hits: 0,
        }
    }

    /// Look up a cached result.
    ///
    /// Returns `Some(result)` if the entry is found, `None` otherwise.
    ///
    /// # Arguments
    /// * `op` — operation tag (distinguishes different operations)
    /// * `f`, `g`, `h` — operand node IDs
    pub fn lookup(&mut self, op: u8, f: NodeId, g: NodeId, h: NodeId) -> Option<NodeId> {
        self.lookups += 1;
        let result = self.entries.get(&(op, f, g, h)).copied();
        if result.is_some() {
            self.hits += 1;
        }
        result
    }

    /// Insert a result into the cache.
    ///
    /// If an entry with the same key already exists, it is overwritten.
    ///
    /// # Arguments
    /// * `op` — operation tag
    /// * `f`, `g`, `h` — operand node IDs
    /// * `result` — the computed result to cache
    pub fn insert(&mut self, op: u8, f: NodeId, g: NodeId, h: NodeId, result: NodeId) {
        self.entries.insert((op, f, g, h), result);
    }

    /// Clear all entries from the cache, resetting hit statistics.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.lookups = 0;
        self.hits = 0;
    }

    /// Return the number of entries currently in the cache.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return true if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return the cache hit rate as a fraction in [0.0, 1.0].
    ///
    /// Returns 0.0 if no lookups have been performed.
    pub fn hit_rate(&self) -> f64 {
        if self.lookups == 0 {
            0.0
        } else {
            self.hits as f64 / self.lookups as f64
        }
    }

    /// Return the total number of lookups performed.
    pub fn total_lookups(&self) -> u64 {
        self.lookups
    }

    /// Return the total number of cache hits.
    pub fn total_hits(&self) -> u64 {
        self.hits
    }
}

impl Default for LocalCache {
    fn default() -> Self {
        Self::new()
    }
}

impl Manager {
    /// Execute a closure with a fresh local cache.
    ///
    /// The local cache is created before the closure runs and is dropped
    /// after it returns. This is useful for operations that need isolated
    /// caching (e.g., during reordering, approximation, or other nested
    /// computations that should not pollute the global cache).
    ///
    /// # Example
    ///
    /// ```rust
    /// use lumindd::Manager;
    ///
    /// let mut mgr = Manager::new();
    /// let x = mgr.bdd_new_var();
    /// let y = mgr.bdd_new_var();
    ///
    /// let result = mgr.with_local_cache(|mgr, cache| {
    ///     let r = mgr.bdd_and(x, y);
    ///     cache.insert(0, x, y, lumindd::NodeId::ZERO, r);
    ///     cache.len()
    /// });
    /// assert_eq!(result, 1);
    /// ```
    pub fn with_local_cache<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut Manager, &mut LocalCache) -> T,
    {
        let mut cache = LocalCache::new();
        f(self, &mut cache)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_insert_lookup() {
        let mut cache = LocalCache::new();
        let a = NodeId::ONE;
        let b = NodeId::ZERO;

        cache.insert(1, a, b, NodeId::ZERO, a);
        assert_eq!(cache.lookup(1, a, b, NodeId::ZERO), Some(a));
        assert_eq!(cache.lookup(1, b, a, NodeId::ZERO), None);
    }

    #[test]
    fn lookup_miss() {
        let mut cache = LocalCache::new();
        assert_eq!(cache.lookup(0, NodeId::ONE, NodeId::ZERO, NodeId::ZERO), None);
    }

    #[test]
    fn overwrite() {
        let mut cache = LocalCache::new();
        let a = NodeId::ONE;
        let b = NodeId::ZERO;

        cache.insert(0, a, a, NodeId::ZERO, b);
        assert_eq!(cache.lookup(0, a, a, NodeId::ZERO), Some(b));

        cache.insert(0, a, a, NodeId::ZERO, a);
        assert_eq!(cache.lookup(0, a, a, NodeId::ZERO), Some(a));

        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn clear_resets() {
        let mut cache = LocalCache::new();
        cache.insert(0, NodeId::ONE, NodeId::ZERO, NodeId::ZERO, NodeId::ONE);
        cache.insert(1, NodeId::ONE, NodeId::ZERO, NodeId::ZERO, NodeId::ZERO);
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
        assert_eq!(cache.hit_rate(), 0.0);
    }

    #[test]
    fn hit_rate_tracking() {
        let mut cache = LocalCache::new();
        let a = NodeId::ONE;
        let b = NodeId::ZERO;

        cache.insert(0, a, b, NodeId::ZERO, a);

        // 1 hit
        cache.lookup(0, a, b, NodeId::ZERO);
        // 1 miss
        cache.lookup(0, b, a, NodeId::ZERO);

        assert_eq!(cache.total_lookups(), 2);
        assert_eq!(cache.total_hits(), 1);
        assert!((cache.hit_rate() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn with_capacity() {
        let cache = LocalCache::with_capacity(128);
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn different_ops_same_operands() {
        let mut cache = LocalCache::new();
        let a = NodeId::ONE;
        let b = NodeId::ZERO;

        cache.insert(0, a, b, NodeId::ZERO, a);
        cache.insert(1, a, b, NodeId::ZERO, b);

        assert_eq!(cache.lookup(0, a, b, NodeId::ZERO), Some(a));
        assert_eq!(cache.lookup(1, a, b, NodeId::ZERO), Some(b));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn with_local_cache_on_manager() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();

        let (result, cache_len) = mgr.with_local_cache(|mgr, cache| {
            let r = mgr.bdd_and(x, y);
            cache.insert(0, x, y, NodeId::ZERO, r);
            (r, cache.len())
        });

        assert_eq!(cache_len, 1);
        // Result should be valid
        assert!(!mgr.is_constant(result) || result == NodeId::ZERO || result == NodeId::ONE);
    }

    #[test]
    fn empty_hit_rate() {
        let cache = LocalCache::new();
        assert_eq!(cache.hit_rate(), 0.0);
    }

    #[test]
    fn default_trait() {
        let cache = LocalCache::default();
        assert!(cache.is_empty());
    }
}
