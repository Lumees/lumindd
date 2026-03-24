// lumindd — Hook and callback system for manager events
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

//! Hook/callback system for observing manager-level events.
//!
//! Users can register callbacks for events such as variable reordering and
//! garbage collection. Hooks are fired synchronously when the corresponding
//! event occurs inside the [`Manager`](crate::Manager).
//!
//! # Example
//!
//! ```rust
//! use lumindd::hooks::{HookType, HookInfo, HookFn, HookRegistry};
//!
//! let mut registry = HookRegistry::new();
//! registry.register(HookType::PreReorder, Box::new(|info: &HookInfo| {
//!     println!("about to reorder — {} nodes, {} vars", info.num_nodes, info.num_vars);
//! }));
//!
//! // Simulate firing the hook.
//! let info = HookInfo {
//!     hook_type: HookType::PreReorder,
//!     num_nodes: 1024,
//!     num_vars: 8,
//! };
//! registry.fire(&info);
//! ```

use std::collections::HashMap;

/// Types of hooks that can be registered on a [`Manager`](crate::Manager).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HookType {
    /// Fired immediately before variable reordering begins.
    PreReorder,
    /// Fired immediately after variable reordering completes.
    PostReorder,
    /// Fired immediately before garbage collection begins.
    PreGarbageCollect,
    /// Fired immediately after garbage collection completes.
    PostGarbageCollect,
}

/// Information passed to hook callbacks when a hook fires.
#[derive(Clone, Debug)]
pub struct HookInfo {
    /// Which hook event triggered the callback.
    pub hook_type: HookType,
    /// The number of nodes in the arena at the time of the event.
    pub num_nodes: usize,
    /// The number of BDD/ADD variables at the time of the event.
    pub num_vars: u16,
}

/// A hook callback function.
///
/// The function receives a shared reference to [`HookInfo`] describing the
/// event. Callbacks must be `Send` so that managers can be moved between
/// threads (the callbacks themselves are always invoked on the thread that
/// owns the manager).
pub type HookFn = Box<dyn Fn(&HookInfo) + Send>;

/// A registry that stores hook callbacks keyed by [`HookType`].
pub struct HookRegistry {
    hooks: HashMap<HookType, Vec<HookFn>>,
}

impl HookRegistry {
    /// Create an empty hook registry.
    pub fn new() -> Self {
        HookRegistry {
            hooks: HashMap::new(),
        }
    }

    /// Register a callback for the given hook type.
    ///
    /// Multiple callbacks can be registered for the same type; they will be
    /// invoked in registration order when [`fire`](Self::fire) is called.
    pub fn register(&mut self, hook_type: HookType, callback: HookFn) {
        self.hooks.entry(hook_type).or_default().push(callback);
    }

    /// Remove all callbacks registered for the given hook type.
    pub fn unregister_all(&mut self, hook_type: HookType) {
        self.hooks.remove(&hook_type);
    }

    /// Fire all callbacks registered for `info.hook_type`.
    ///
    /// Callbacks are invoked synchronously in the order they were registered.
    /// If no callbacks are registered for the given type, this is a no-op.
    pub fn fire(&self, info: &HookInfo) {
        if let Some(callbacks) = self.hooks.get(&info.hook_type) {
            for cb in callbacks {
                cb(info);
            }
        }
    }

    /// Returns true if there are any callbacks registered for the given type.
    pub fn has_hooks(&self, hook_type: HookType) -> bool {
        self.hooks
            .get(&hook_type)
            .map_or(false, |v| !v.is_empty())
    }

    /// Returns the number of callbacks registered for the given type.
    pub fn hook_count(&self, hook_type: HookType) -> usize {
        self.hooks.get(&hook_type).map_or(0, |v| v.len())
    }

    /// Remove all callbacks for every hook type.
    pub fn clear(&mut self) {
        self.hooks.clear();
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for HookRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut map = f.debug_map();
        for (ht, cbs) in &self.hooks {
            map.entry(ht, &format!("{} callback(s)", cbs.len()));
        }
        map.finish()
    }
}

// ---------------------------------------------------------------------------
// Manager integration
// ---------------------------------------------------------------------------

use crate::manager::Manager;

impl Manager {
    /// Register a hook callback for the given event type.
    ///
    /// The callback will be invoked whenever the corresponding event occurs
    /// (e.g., before/after reordering or garbage collection).
    ///
    /// # Example
    ///
    /// ```rust
    /// use lumindd::Manager;
    /// use lumindd::hooks::{HookType, HookInfo};
    ///
    /// let mut mgr = Manager::new();
    /// mgr.add_hook(HookType::PostGarbageCollect, Box::new(|info: &HookInfo| {
    ///     println!("GC done — {} nodes remain", info.num_nodes);
    /// }));
    /// ```
    pub fn add_hook(&mut self, hook_type: HookType, callback: HookFn) {
        self.hooks.register(hook_type, callback);
    }

    /// Remove all hook callbacks for the given event type.
    pub fn remove_hooks(&mut self, hook_type: HookType) {
        self.hooks.unregister_all(hook_type);
    }

    /// Fire all hooks registered for the given type, supplying a snapshot of
    /// the current manager state as [`HookInfo`].
    pub(crate) fn fire_hooks(&self, hook_type: HookType) {
        if self.hooks.has_hooks(hook_type) {
            let info = HookInfo {
                hook_type,
                num_nodes: self.nodes.len(),
                num_vars: self.num_vars,
            };
            self.hooks.fire(&info);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn register_and_fire() {
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();

        let mut reg = HookRegistry::new();
        reg.register(
            HookType::PreReorder,
            Box::new(move |_info| {
                c.fetch_add(1, Ordering::Relaxed);
            }),
        );

        let info = HookInfo {
            hook_type: HookType::PreReorder,
            num_nodes: 100,
            num_vars: 4,
        };
        reg.fire(&info);
        reg.fire(&info);

        assert_eq!(counter.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn fire_wrong_type_is_noop() {
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();

        let mut reg = HookRegistry::new();
        reg.register(
            HookType::PreReorder,
            Box::new(move |_| {
                c.fetch_add(1, Ordering::Relaxed);
            }),
        );

        let info = HookInfo {
            hook_type: HookType::PostReorder,
            num_nodes: 0,
            num_vars: 0,
        };
        reg.fire(&info);

        assert_eq!(counter.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn unregister_all() {
        let mut reg = HookRegistry::new();
        reg.register(HookType::PreGarbageCollect, Box::new(|_| {}));
        reg.register(HookType::PreGarbageCollect, Box::new(|_| {}));
        assert_eq!(reg.hook_count(HookType::PreGarbageCollect), 2);

        reg.unregister_all(HookType::PreGarbageCollect);
        assert_eq!(reg.hook_count(HookType::PreGarbageCollect), 0);
        assert!(!reg.has_hooks(HookType::PreGarbageCollect));
    }

    #[test]
    fn multiple_hooks_fire_in_order() {
        let log = Arc::new(std::sync::Mutex::new(Vec::new()));

        let mut reg = HookRegistry::new();
        let l1 = log.clone();
        reg.register(
            HookType::PostGarbageCollect,
            Box::new(move |_| l1.lock().unwrap().push(1)),
        );
        let l2 = log.clone();
        reg.register(
            HookType::PostGarbageCollect,
            Box::new(move |_| l2.lock().unwrap().push(2)),
        );
        let l3 = log.clone();
        reg.register(
            HookType::PostGarbageCollect,
            Box::new(move |_| l3.lock().unwrap().push(3)),
        );

        let info = HookInfo {
            hook_type: HookType::PostGarbageCollect,
            num_nodes: 50,
            num_vars: 2,
        };
        reg.fire(&info);

        assert_eq!(*log.lock().unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn clear_removes_all() {
        let mut reg = HookRegistry::new();
        reg.register(HookType::PreReorder, Box::new(|_| {}));
        reg.register(HookType::PostReorder, Box::new(|_| {}));
        reg.register(HookType::PreGarbageCollect, Box::new(|_| {}));
        reg.clear();
        assert!(!reg.has_hooks(HookType::PreReorder));
        assert!(!reg.has_hooks(HookType::PostReorder));
        assert!(!reg.has_hooks(HookType::PreGarbageCollect));
    }

    #[test]
    fn manager_add_and_remove_hooks() {
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();

        let mut mgr = Manager::new();
        mgr.add_hook(
            HookType::PreReorder,
            Box::new(move |_| {
                c.fetch_add(1, Ordering::Relaxed);
            }),
        );

        mgr.fire_hooks(HookType::PreReorder);
        assert_eq!(counter.load(Ordering::Relaxed), 1);

        mgr.remove_hooks(HookType::PreReorder);
        mgr.fire_hooks(HookType::PreReorder);
        assert_eq!(counter.load(Ordering::Relaxed), 1); // unchanged
    }
}
