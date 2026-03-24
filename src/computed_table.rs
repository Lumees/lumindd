// lumindd — Computed table (operation cache)
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use crate::node::NodeId;

/// Operation tags for the computed table.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[allow(dead_code)]
pub(crate) enum OpTag {
    BddIte,
    BddAnd,
    BddXor,
    BddExist,
    BddAndAbstract,
    BddCompose,
    BddRestrict,
    BddConstrain,
    AddIte,
    AddApply(u8), // sub-tag for +, *, -, /, min, max, etc.
    AddMonadic(u8),
    ZddUnion,
    ZddIntersect,
    ZddDiff,
    ZddProduct,
    ZddWeakDiv,
    ZddChange,
    ZddIte,
    BddVectorCompose,
    BddPermute,
    BddSwapVars,
    AddVectorCompose,
    AddPermute,
    BddUnderApprox,
    BddOverApprox,
    BddSubsetHeavy,
    BddSupersetHeavy,
    BddSubsetShort,
    BddSupersetShort,
    BddRemapUnderApprox,
    BddSqueeze,
    BddHammingDist,
    AddHamming,
    BddConjDecomp,
    BddDisjDecomp,
    BddSolveEqn,
    BddCompatProj,
}

impl OpTag {
    fn discriminant(self) -> u64 {
        // Produce a unique u64 for hashing purposes.
        match self {
            OpTag::BddIte => 0,
            OpTag::BddAnd => 1,
            OpTag::BddXor => 2,
            OpTag::BddExist => 3,
            OpTag::BddAndAbstract => 5,
            OpTag::BddCompose => 6,
            OpTag::BddRestrict => 7,
            OpTag::BddConstrain => 8,
            OpTag::AddIte => 9,
            OpTag::AddApply(t) => 10 + t as u64,
            OpTag::AddMonadic(t) => 30 + t as u64,
            OpTag::ZddUnion => 50,
            OpTag::ZddIntersect => 51,
            OpTag::ZddDiff => 52,
            OpTag::ZddProduct => 53,
            OpTag::ZddWeakDiv => 54,
            OpTag::ZddChange => 55,
            OpTag::ZddIte => 56,
            OpTag::BddVectorCompose => 60,
            OpTag::BddPermute => 61,
            OpTag::BddSwapVars => 62,
            OpTag::AddVectorCompose => 63,
            OpTag::AddPermute => 64,
            OpTag::BddUnderApprox => 70,
            OpTag::BddOverApprox => 71,
            OpTag::BddSubsetHeavy => 72,
            OpTag::BddSupersetHeavy => 73,
            OpTag::BddSubsetShort => 74,
            OpTag::BddSupersetShort => 75,
            OpTag::BddRemapUnderApprox => 76,
            OpTag::BddSqueeze => 77,
            OpTag::BddHammingDist => 78,
            OpTag::AddHamming => 79,
            OpTag::BddConjDecomp => 80,
            OpTag::BddDisjDecomp => 81,
            OpTag::BddSolveEqn => 82,
            OpTag::BddCompatProj => 83,
        }
    }
}

/// A single cache entry.
#[derive(Clone)]
struct CacheEntry {
    op: OpTag,
    f: NodeId,
    g: NodeId,
    h: NodeId,
    result: NodeId,
}

/// Direct-mapped computed table (cache).
///
/// Uses a power-of-two sized array with hash-based indexing.
/// Collisions silently overwrite the previous entry (no chaining).
/// This matches CUDD's design — a lossy cache is fine because
/// results can always be recomputed.
pub(crate) struct ComputedTable {
    entries: Vec<Option<CacheEntry>>,
    mask: usize,
    pub(crate) hits: u64,
    pub(crate) misses: u64,
}

impl ComputedTable {
    pub(crate) fn new(size_log2: u32) -> Self {
        let size = 1usize << size_log2;
        ComputedTable {
            entries: vec![None; size],
            mask: size - 1,
            hits: 0,
            misses: 0,
        }
    }

    #[inline]
    fn hash(&self, op: OpTag, f: NodeId, g: NodeId, h: NodeId) -> usize {
        // FNV-1a inspired hash mixing
        let mut hash = 0xcbf29ce484222325u64;
        hash ^= op.discriminant();
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= f.regular().raw_index() as u64 | ((f.is_complemented() as u64) << 32);
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= g.regular().raw_index() as u64 | ((g.is_complemented() as u64) << 33);
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= h.regular().raw_index() as u64 | ((h.is_complemented() as u64) << 34);
        hash = hash.wrapping_mul(0x100000001b3);
        hash as usize & self.mask
    }

    /// Look up a cached result.
    #[inline]
    pub(crate) fn lookup(
        &mut self,
        op: OpTag,
        f: NodeId,
        g: NodeId,
        h: NodeId,
    ) -> Option<NodeId> {
        let idx = self.hash(op, f, g, h);
        if let Some(entry) = &self.entries[idx] {
            if entry.op == op && entry.f == f && entry.g == g && entry.h == h {
                self.hits += 1;
                return Some(entry.result);
            }
        }
        self.misses += 1;
        None
    }

    /// Insert a result into the cache (overwrites on collision).
    #[inline]
    pub(crate) fn insert(
        &mut self,
        op: OpTag,
        f: NodeId,
        g: NodeId,
        h: NodeId,
        result: NodeId,
    ) {
        let idx = self.hash(op, f, g, h);
        self.entries[idx] = Some(CacheEntry {
            op,
            f,
            g,
            h,
            result,
        });
    }

    /// Clear the entire cache (called during GC or reordering).
    pub(crate) fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            *entry = None;
        }
    }

}
