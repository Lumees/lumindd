// lumindd — Unique table for canonical node storage
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::HashMap;

use crate::node::{NodeId, RawIndex};

/// Per-variable unique table ensuring canonical representation.
///
/// Maps `(then_child, else_child)` pairs to their canonical node index.
/// This guarantees that structurally identical sub-graphs share nodes.
pub(crate) struct UniqueSubtable {
    /// Map from (then, else) child pair to raw node index.
    map: HashMap<(NodeId, NodeId), RawIndex>,
    /// Number of live entries.
    pub(crate) keys: usize,
}

impl UniqueSubtable {
    pub(crate) fn new() -> Self {
        UniqueSubtable {
            map: HashMap::new(),
            keys: 0,
        }
    }

    /// Look up the canonical node for a given (then, else) pair.
    #[inline]
    pub(crate) fn lookup(&self, then_child: NodeId, else_child: NodeId) -> Option<RawIndex> {
        self.map.get(&(then_child, else_child)).copied()
    }

    /// Insert a new canonical node.
    pub(crate) fn insert(&mut self, then_child: NodeId, else_child: NodeId, index: RawIndex) {
        self.map.insert((then_child, else_child), index);
        self.keys += 1;
    }

    /// Number of entries.
    pub(crate) fn len(&self) -> usize {
        self.map.len()
    }
}

/// Unique table for ADD constants, keyed by f64 bits.
pub(crate) struct ConstantTable {
    map: HashMap<u64, RawIndex>,
}

impl ConstantTable {
    pub(crate) fn new() -> Self {
        ConstantTable {
            map: HashMap::new(),
        }
    }

    pub(crate) fn lookup(&self, value: f64) -> Option<RawIndex> {
        self.map.get(&value.to_bits()).copied()
    }

    pub(crate) fn insert(&mut self, value: f64, index: RawIndex) {
        self.map.insert(value.to_bits(), index);
    }
}
