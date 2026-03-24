// lumindd — Node representation with complemented edges
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

/// Raw index into the node arena (without complement bit).
pub(crate) type RawIndex = u32;

/// Maximum reference count — saturates to prevent overflow (never freed).
pub(crate) const MAX_REF: u32 = u32::MAX;

/// Constant sentinel index for terminal nodes.
pub(crate) const CONST_INDEX: u16 = u16::MAX;

/// A node handle that encodes a complemented-edge bit in the LSB.
///
/// The upper 31 bits store the raw arena index, the lowest bit is the
/// complement flag. This makes NOT an O(1) pointer flip.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u32);

impl NodeId {
    /// The canonical "one" constant (index 0, not complemented).
    pub const ONE: NodeId = NodeId(0);

    /// The canonical "zero" constant (index 0, complemented).
    pub const ZERO: NodeId = NodeId(1);

    #[inline(always)]
    pub(crate) fn from_raw(index: RawIndex, complemented: bool) -> Self {
        NodeId((index << 1) | (complemented as u32))
    }

    #[inline(always)]
    pub(crate) fn raw_index(self) -> RawIndex {
        self.0 >> 1
    }

    /// Returns true if this edge is complemented.
    #[inline(always)]
    pub fn is_complemented(self) -> bool {
        self.0 & 1 != 0
    }

    /// Returns the regular (non-complemented) version of this node.
    #[inline(always)]
    pub fn regular(self) -> Self {
        NodeId(self.0 & !1)
    }

    /// Flips the complement bit.
    #[inline(always)]
    pub fn not(self) -> Self {
        NodeId(self.0 ^ 1)
    }

    /// Conditionally complements.
    #[inline(always)]
    pub fn not_cond(self, cond: bool) -> Self {
        NodeId(self.0 ^ (cond as u32))
    }

    /// Returns true if this is the constant ONE node.
    #[inline(always)]
    pub fn is_one(self) -> bool {
        self.0 == 0
    }

    /// Returns true if this is the constant ZERO node.
    #[inline(always)]
    pub fn is_zero(self) -> bool {
        self.0 == 1
    }

    /// Returns true if this is a constant (ONE or ZERO for BDD).
    #[inline(always)]
    pub fn is_constant(self) -> bool {
        self.raw_index() == 0
    }
}

impl std::fmt::Debug for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_complemented() {
            write!(f, "!N{}", self.raw_index())
        } else {
            write!(f, "N{}", self.raw_index())
        }
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

/// Internal node stored in the arena.
#[derive(Clone)]
pub(crate) enum DdNode {
    /// Terminal / constant node. For BDD the only constant is ONE (index 0).
    /// For ADD, terminals hold an f64 value.
    Constant {
        value: f64,
        ref_count: u32,
    },
    /// Internal decision node.
    Internal {
        /// Variable index (not level — level depends on current ordering).
        var_index: u16,
        /// Then-child (high branch).
        then_child: NodeId,
        /// Else-child (low branch).
        else_child: NodeId,
        /// Reference count (saturating).
        ref_count: u32,
    },
}

impl DdNode {
    #[inline]
    pub(crate) fn ref_count(&self) -> u32 {
        match self {
            DdNode::Constant { ref_count, .. } => *ref_count,
            DdNode::Internal { ref_count, .. } => *ref_count,
        }
    }

    #[inline]
    pub(crate) fn ref_count_mut(&mut self) -> &mut u32 {
        match self {
            DdNode::Constant { ref_count, .. } => ref_count,
            DdNode::Internal { ref_count, .. } => ref_count,
        }
    }

    #[inline]
    pub(crate) fn incr_ref(&mut self) {
        let rc = self.ref_count_mut();
        if *rc < MAX_REF {
            *rc += 1;
        }
    }

    #[inline]
    pub(crate) fn decr_ref(&mut self) {
        let rc = self.ref_count_mut();
        if *rc < MAX_REF && *rc > 0 {
            *rc -= 1;
        }
    }

    #[inline]
    pub(crate) fn var_index(&self) -> u16 {
        match self {
            DdNode::Constant { .. } => CONST_INDEX,
            DdNode::Internal { var_index, .. } => *var_index,
        }
    }

    /// Returns then-child. Returns `None` for constant nodes.
    #[inline]
    pub(crate) fn then_child(&self) -> NodeId {
        match self {
            DdNode::Internal { then_child, .. } => *then_child,
            DdNode::Constant { .. } => {
                debug_assert!(false, "then_child called on constant node");
                NodeId::ZERO
            }
        }
    }

    /// Returns else-child. Returns `None` for constant nodes.
    #[inline]
    pub(crate) fn else_child(&self) -> NodeId {
        match self {
            DdNode::Internal { else_child, .. } => *else_child,
            DdNode::Constant { .. } => {
                debug_assert!(false, "else_child called on constant node");
                NodeId::ZERO
            }
        }
    }
}
