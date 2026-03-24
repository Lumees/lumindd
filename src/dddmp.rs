// lumindd — DDDMP-compatible serialization for decision diagrams
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

//! DDDMP-compatible serialization and deserialization for BDDs, ADDs, and ZDDs.
//!
//! Supports three output formats:
//! - **Text** — human-readable, one line per node
//! - **Binary** — compact encoding with variable-length node IDs
//! - **CNF** — DIMACS-format clause export for SAT solver interop

use crate::manager::Manager;
use crate::node::{DdNode, NodeId, CONST_INDEX};

use std::collections::HashMap;
use std::io::{self, BufRead, Read, Write};

// ======================================================================
// Constants
// ======================================================================

const DDDMP_VERSION: &str = "DDDMP-2.0";
const DDDMP_TEXT_MODE: &str = "A";
const DDDMP_BINARY_MODE: &str = "B";

// Node type tags in the binary encoding
const BIN_TAG_TERMINAL_ONE: u8 = 0x00;
const BIN_TAG_TERMINAL_ZERO: u8 = 0x01;
const BIN_TAG_INTERNAL: u8 = 0x02;
const BIN_TAG_INTERNAL_COMP: u8 = 0x03;

// ======================================================================
// Internal serialisation helpers
// ======================================================================

/// A node in serialization order with a sequential ID.
struct SerNode {
    /// Sequential ID assigned during topological sort (1-based; 0 = unused).
    seq_id: u32,
    /// Variable index (CONST_INDEX for terminals).
    var_index: u16,
    /// Sequential ID of the then-child (0 for terminals).
    then_seq: u32,
    /// Sequential ID of the else-child (0 for terminals).
    else_seq: u32,
    /// True if the then-edge is complemented.
    then_comp: bool,
    /// True if the else-edge is complemented.
    else_comp: bool,
    /// True if this node is the constant ONE terminal.
    is_one: bool,
}

impl Manager {
    // ------------------------------------------------------------------
    // Topological sort of BDD DAG
    // ------------------------------------------------------------------

    /// Collect all nodes reachable from `root` in bottom-up topological order.
    ///
    /// Returns `(sorted_nodes, root_complemented)` where each entry is
    /// `(raw_index, var_index, raw_then, then_complemented, raw_else, else_complemented)`.
    /// The constant-ONE node is always first in the returned vector.
    fn topo_sort(&self, root: NodeId) -> (Vec<SerNode>, bool) {
        let root_comp = root.is_complemented();
        let root_reg = root.regular();

        // DFS to collect all reachable raw indices
        let mut visited: HashMap<u32, u32> = HashMap::new(); // raw_index -> seq_id
        let mut order: Vec<SerNode> = Vec::new();

        // Reserve seq_id 1 for the constant-ONE terminal
        visited.insert(0, 1);
        order.push(SerNode {
            seq_id: 1,
            var_index: CONST_INDEX,
            then_seq: 0,
            else_seq: 0,
            then_comp: false,
            else_comp: false,
            is_one: true,
        });

        self.topo_visit(root_reg, &mut visited, &mut order);

        (order, root_comp)
    }

    /// Recursive DFS visitor for topological sort.
    fn topo_visit(
        &self,
        id: NodeId,
        visited: &mut HashMap<u32, u32>,
        order: &mut Vec<SerNode>,
    ) {
        let raw = id.raw_index();
        if visited.contains_key(&raw) {
            return;
        }

        match &self.nodes[raw as usize] {
            DdNode::Constant { .. } => {
                // Non-ONE constant (ADD terminal) — assign a new seq_id
                let seq = order.len() as u32 + 1;
                visited.insert(raw, seq);
                order.push(SerNode {
                    seq_id: seq,
                    var_index: CONST_INDEX,
                    then_seq: 0,
                    else_seq: 0,
                    then_comp: false,
                    else_comp: false,
                    is_one: false,
                });
            }
            DdNode::Internal {
                var_index,
                then_child,
                else_child,
                ..
            } => {
                let vi = *var_index;
                let t = *then_child;
                let e = *else_child;

                // Visit children first (bottom-up)
                self.topo_visit(t.regular(), visited, order);
                self.topo_visit(e.regular(), visited, order);

                let seq = order.len() as u32 + 1;
                visited.insert(raw, seq);

                let t_seq = visited[&t.raw_index()];
                let e_seq = visited[&e.raw_index()];

                order.push(SerNode {
                    seq_id: seq,
                    var_index: vi,
                    then_seq: t_seq,
                    else_seq: e_seq,
                    then_comp: t.is_complemented(),
                    else_comp: e.is_complemented(),
                    is_one: false,
                });
            }
        }
    }

    // ==================================================================
    // Text format — save
    // ==================================================================

    /// Save the BDD rooted at `f` in DDDMP text format.
    ///
    /// If `var_names` is provided, the header will include symbolic names
    /// for each variable; otherwise default names `x0, x1, …` are used.
    pub fn dddmp_save_text<W: Write>(
        &self,
        f: NodeId,
        var_names: Option<&[&str]>,
        out: &mut W,
    ) -> io::Result<()> {
        let (nodes, root_comp) = self.topo_sort(f);
        let num_nodes = nodes.len();
        let root_seq = nodes.last().map_or(1, |n| n.seq_id);

        // --- header ---
        writeln!(out, ".ver {DDDMP_VERSION}")?;
        writeln!(out, ".mode {DDDMP_TEXT_MODE}")?;
        writeln!(out, ".varinfo 0")?;
        writeln!(out, ".nnodes {num_nodes}")?;
        writeln!(out, ".nvars {}", self.num_vars)?;
        writeln!(out, ".nsuppvars {}", self.num_vars)?;

        // Variable ordering
        write!(out, ".orderedvarnames")?;
        for level in 0..self.num_vars as usize {
            let vi = self.inv_perm[level] as usize;
            let name = var_names
                .and_then(|ns| ns.get(vi).copied())
                .unwrap_or("");
            if name.is_empty() {
                write!(out, " x{vi}")?;
            } else {
                write!(out, " {name}")?;
            }
        }
        writeln!(out)?;

        // Support variable IDs
        write!(out, ".suppvarnames")?;
        for vi in 0..self.num_vars as usize {
            let name = var_names
                .and_then(|ns| ns.get(vi).copied())
                .unwrap_or("");
            if name.is_empty() {
                write!(out, " x{vi}")?;
            } else {
                write!(out, " {name}")?;
            }
        }
        writeln!(out)?;

        // IDs (variable indices)
        write!(out, ".ids")?;
        for vi in 0..self.num_vars {
            write!(out, " {vi}")?;
        }
        writeln!(out)?;

        // Permutation
        write!(out, ".permids")?;
        for vi in 0..self.num_vars as usize {
            write!(out, " {}", self.perm[vi])?;
        }
        writeln!(out)?;

        // Root node(s)
        let root_id_signed: i64 = if root_comp {
            -(root_seq as i64)
        } else {
            root_seq as i64
        };
        writeln!(out, ".nroots 1")?;
        writeln!(out, ".rootids {root_id_signed}")?;

        // --- nodes ---
        writeln!(out, ".nodes")?;
        for sn in &nodes {
            if sn.var_index == CONST_INDEX {
                // Terminal node
                writeln!(out, "{} T 1 0 0", sn.seq_id)?;
            } else {
                let t_signed: i64 = if sn.then_comp {
                    -(sn.then_seq as i64)
                } else {
                    sn.then_seq as i64
                };
                let e_signed: i64 = if sn.else_comp {
                    -(sn.else_seq as i64)
                } else {
                    sn.else_seq as i64
                };
                writeln!(
                    out,
                    "{} {} {} {} {}",
                    sn.seq_id, sn.var_index, t_signed, e_signed, 0
                )?;
            }
        }
        writeln!(out, ".end")?;

        Ok(())
    }

    // ==================================================================
    // Text format — load
    // ==================================================================

    /// Load a BDD from DDDMP text format, reconstructing it in this manager.
    ///
    /// Returns the root `NodeId`. Variables are created as needed if the
    /// manager does not already have enough.
    pub fn dddmp_load_text<R: BufRead>(&mut self, input: &mut R) -> io::Result<NodeId> {
        let mut num_nodes: usize = 0;
        let mut num_vars: u16 = 0;
        let mut root_ids: Vec<i64> = Vec::new();
        let mut in_nodes = false;
        let mut _perm_map: Vec<u32> = Vec::new(); // file var index -> file level

        // Raw node descriptions read from the file
        struct RawNode {
            var_index: Option<u16>, // None for terminal
            then_id: i64,          // signed seq id
            else_id: i64,          // signed seq id
        }

        let mut raw_nodes: Vec<RawNode> = Vec::new(); // indexed by (seq_id - 1)

        let mut line_buf = String::new();
        loop {
            line_buf.clear();
            let n = input.read_line(&mut line_buf)?;
            if n == 0 {
                break;
            }
            let line = line_buf.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if in_nodes {
                if line == ".end" {
                    break;
                }
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 4 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("malformed node line: {line}"),
                    ));
                }

                if parts[1] == "T" {
                    // Terminal
                    raw_nodes.push(RawNode {
                        var_index: None,
                        then_id: 0,
                        else_id: 0,
                    });
                } else {
                    let vi: u16 = parts[1].parse().map_err(|e| {
                        io::Error::new(io::ErrorKind::InvalidData, format!("bad var index: {e}"))
                    })?;
                    let t_id: i64 = parts[2].parse().map_err(|e| {
                        io::Error::new(io::ErrorKind::InvalidData, format!("bad then id: {e}"))
                    })?;
                    let e_id: i64 = parts[3].parse().map_err(|e| {
                        io::Error::new(io::ErrorKind::InvalidData, format!("bad else id: {e}"))
                    })?;

                    raw_nodes.push(RawNode {
                        var_index: Some(vi),
                        then_id: t_id,
                        else_id: e_id,
                    });
                }
            } else if let Some(rest) = line.strip_prefix(".nnodes ") {
                num_nodes = rest.trim().parse().map_err(|e| {
                    io::Error::new(io::ErrorKind::InvalidData, format!("bad nnodes: {e}"))
                })?;
                let _ = num_nodes; // used for pre-allocation below
            } else if let Some(rest) = line.strip_prefix(".nvars ") {
                num_vars = rest.trim().parse().map_err(|e| {
                    io::Error::new(io::ErrorKind::InvalidData, format!("bad nvars: {e}"))
                })?;
            } else if let Some(rest) = line.strip_prefix(".rootids ") {
                root_ids = rest
                    .split_whitespace()
                    .map(|s| {
                        s.parse::<i64>()
                            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("bad root id: {e}")))
                    })
                    .collect::<io::Result<Vec<_>>>()?;
            } else if let Some(rest) = line.strip_prefix(".permids ") {
                _perm_map = rest
                    .split_whitespace()
                    .map(|s| {
                        s.parse::<u32>()
                            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("bad permid: {e}")))
                    })
                    .collect::<io::Result<Vec<_>>>()?;
            } else if line == ".nodes" {
                in_nodes = true;
                raw_nodes.reserve(num_nodes);
            }
            // other header lines are accepted and ignored
        }

        // Ensure the manager has enough variables
        while self.num_vars < num_vars {
            self.bdd_new_var();
        }

        // Build a mapping from sequential IDs (1-based) to NodeIds
        let mut seq_to_node: Vec<NodeId> = Vec::with_capacity(raw_nodes.len() + 1);
        seq_to_node.push(NodeId::ZERO); // placeholder for unused index 0

        let resolve = |seq_signed: i64, map: &[NodeId]| -> io::Result<NodeId> {
            let comp = seq_signed < 0;
            let idx = seq_signed.unsigned_abs() as usize;
            if idx == 0 || idx >= map.len() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid node reference: {seq_signed}"),
                ));
            }
            Ok(map[idx].not_cond(comp))
        };

        // Reconstruct bottom-up — nodes are already stored in topological order
        for rn in &raw_nodes {
            let node_id = match rn.var_index {
                None => {
                    // Terminal ONE
                    NodeId::ONE
                }
                Some(vi) => {
                    let t = resolve(rn.then_id, &seq_to_node)?;
                    let e = resolve(rn.else_id, &seq_to_node)?;
                    self.unique_inter(vi, t, e)
                }
            };
            seq_to_node.push(node_id);
        }

        // Resolve root
        if root_ids.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "no root IDs found",
            ));
        }

        let root = resolve(root_ids[0], &seq_to_node)?;
        self.ref_node(root);
        Ok(root)
    }

    // ==================================================================
    // Binary format — save
    // ==================================================================

    /// Save the BDD rooted at `f` in DDDMP binary (compact) format.
    pub fn dddmp_save_binary<W: Write>(&self, f: NodeId, out: &mut W) -> io::Result<()> {
        let (nodes, root_comp) = self.topo_sort(f);
        let num_nodes = nodes.len();
        let root_seq = nodes.last().map_or(1, |n| n.seq_id);

        // --- text header (same structure as text mode, but mode = B) ---
        writeln!(out, ".ver {DDDMP_VERSION}")?;
        writeln!(out, ".mode {DDDMP_BINARY_MODE}")?;
        writeln!(out, ".varinfo 0")?;
        writeln!(out, ".nnodes {num_nodes}")?;
        writeln!(out, ".nvars {}", self.num_vars)?;
        writeln!(out, ".nsuppvars {}", self.num_vars)?;

        write!(out, ".orderedvarnames")?;
        for level in 0..self.num_vars as usize {
            let vi = self.inv_perm[level] as usize;
            write!(out, " x{vi}")?;
        }
        writeln!(out)?;

        write!(out, ".ids")?;
        for vi in 0..self.num_vars {
            write!(out, " {vi}")?;
        }
        writeln!(out)?;

        write!(out, ".permids")?;
        for vi in 0..self.num_vars as usize {
            write!(out, " {}", self.perm[vi])?;
        }
        writeln!(out)?;

        let root_id_signed: i64 = if root_comp {
            -(root_seq as i64)
        } else {
            root_seq as i64
        };
        writeln!(out, ".nroots 1")?;
        writeln!(out, ".rootids {root_id_signed}")?;

        writeln!(out, ".nodes")?;
        out.flush()?;

        // --- binary node data ---
        for sn in &nodes {
            if sn.var_index == CONST_INDEX {
                if sn.is_one {
                    out.write_all(&[BIN_TAG_TERMINAL_ONE])?;
                } else {
                    out.write_all(&[BIN_TAG_TERMINAL_ZERO])?;
                }
            } else {
                let tag = if sn.else_comp {
                    BIN_TAG_INTERNAL_COMP
                } else {
                    BIN_TAG_INTERNAL
                };
                out.write_all(&[tag])?;
                Self::write_varint(out, sn.var_index as u32)?;
                // then-child: seq_id with complement in LSB
                let t_enc = (sn.then_seq << 1) | (sn.then_comp as u32);
                Self::write_varint(out, t_enc)?;
                // else-child: seq_id (complement already encoded in tag)
                Self::write_varint(out, sn.else_seq)?;
            }
        }

        // End marker
        writeln!(out)?;
        writeln!(out, ".end")?;

        Ok(())
    }

    // ==================================================================
    // Binary format — load
    // ==================================================================

    /// Load a BDD from DDDMP binary format.
    pub fn dddmp_load_binary<R: Read>(&mut self, input: &mut R) -> io::Result<NodeId> {
        // Read the entire input so we can split header (text) from body (binary)
        let mut data = Vec::new();
        input.read_to_end(&mut data)?;

        let mut cursor = 0usize;
        let mut num_nodes: usize = 0;
        let mut num_vars: u16 = 0;
        let mut root_ids: Vec<i64> = Vec::new();
        let mut nodes_marker_found = false;

        // Parse text header line-by-line
        loop {
            if cursor >= data.len() {
                break;
            }
            // Find end of line
            let line_end = data[cursor..]
                .iter()
                .position(|&b| b == b'\n')
                .map(|p| cursor + p)
                .unwrap_or(data.len());

            let line_bytes = &data[cursor..line_end];
            let line = std::str::from_utf8(line_bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid utf8 in header: {e}")))?
                .trim();

            cursor = line_end + 1; // skip past newline

            if line == ".nodes" {
                nodes_marker_found = true;
                break;
            }

            if let Some(rest) = line.strip_prefix(".nnodes ") {
                num_nodes = rest.trim().parse().map_err(|e| {
                    io::Error::new(io::ErrorKind::InvalidData, format!("bad nnodes: {e}"))
                })?;
            } else if let Some(rest) = line.strip_prefix(".nvars ") {
                num_vars = rest.trim().parse().map_err(|e| {
                    io::Error::new(io::ErrorKind::InvalidData, format!("bad nvars: {e}"))
                })?;
            } else if let Some(rest) = line.strip_prefix(".rootids ") {
                root_ids = rest
                    .split_whitespace()
                    .map(|s| {
                        s.parse::<i64>()
                            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("bad root id: {e}")))
                    })
                    .collect::<io::Result<Vec<_>>>()?;
            }
        }

        if !nodes_marker_found {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "missing .nodes marker",
            ));
        }

        // Ensure enough variables
        while self.num_vars < num_vars {
            self.bdd_new_var();
        }

        // Decode binary node data
        let mut seq_to_node: Vec<NodeId> = Vec::with_capacity(num_nodes + 1);
        seq_to_node.push(NodeId::ZERO); // placeholder for index 0

        for _ in 0..num_nodes {
            if cursor >= data.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "unexpected end of binary node data",
                ));
            }
            let tag = data[cursor];
            cursor += 1;

            let node_id = match tag {
                BIN_TAG_TERMINAL_ONE => NodeId::ONE,
                BIN_TAG_TERMINAL_ZERO => NodeId::ZERO,
                BIN_TAG_INTERNAL | BIN_TAG_INTERNAL_COMP => {
                    let else_comp = tag == BIN_TAG_INTERNAL_COMP;
                    let (vi, adv) = Self::read_varint(&data[cursor..])?;
                    cursor += adv;
                    let (t_enc, adv) = Self::read_varint(&data[cursor..])?;
                    cursor += adv;
                    let (e_seq, adv) = Self::read_varint(&data[cursor..])?;
                    cursor += adv;

                    let then_comp = (t_enc & 1) != 0;
                    let then_seq = (t_enc >> 1) as usize;
                    let else_seq = e_seq as usize;

                    if then_seq == 0 || then_seq >= seq_to_node.len()
                        || else_seq == 0 || else_seq >= seq_to_node.len()
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "invalid child reference in binary data",
                        ));
                    }

                    let t = seq_to_node[then_seq].not_cond(then_comp);
                    let e = seq_to_node[else_seq].not_cond(else_comp);
                    self.unique_inter(vi as u16, t, e)
                }
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("unknown binary node tag: 0x{tag:02x}"),
                    ));
                }
            };
            seq_to_node.push(node_id);
        }

        // Resolve root
        if root_ids.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "no root IDs found",
            ));
        }

        let root_signed = root_ids[0];
        let root_comp = root_signed < 0;
        let root_seq = root_signed.unsigned_abs() as usize;
        if root_seq == 0 || root_seq >= seq_to_node.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "root ID out of range",
            ));
        }

        let root = seq_to_node[root_seq].not_cond(root_comp);
        self.ref_node(root);
        Ok(root)
    }

    // ==================================================================
    // CNF export (DIMACS format)
    // ==================================================================

    /// Export the BDD rooted at `f` as a CNF formula in DIMACS format.
    ///
    /// Each BDD internal node `v` with then-child `t` and else-child `e`
    /// produces clauses that encode:  `v <=> ITE(x_i, t, e)`.
    ///
    /// The root is asserted positive. The resulting CNF is equisatisfiable
    /// with the original BDD function.
    pub fn dddmp_save_cnf<W: Write>(&self, f: NodeId, out: &mut W) -> io::Result<()> {
        if f.is_one() {
            // Trivially true — emit an empty CNF
            writeln!(out, "p cnf 0 0")?;
            return Ok(());
        }
        if f.is_zero() {
            // Trivially false — emit a single empty clause
            writeln!(out, "p cnf 0 1")?;
            writeln!(out, "0")?;
            return Ok(());
        }

        let (nodes, root_comp) = self.topo_sort(f);

        // Assign a DIMACS variable to each non-terminal node.
        // BDD variables x0..x_{n-1} get DIMACS vars 1..n.
        // Node auxiliary variables get n+1, n+2, …
        let bdd_var_base: i32 = 1; // BDD variable i -> DIMACS var (i + 1)
        let node_var_base: i32 = self.num_vars as i32 + 1;

        // Map seq_id -> DIMACS variable for node
        let mut seq_to_dimacs: Vec<i32> = vec![0; nodes.len() + 1];
        let mut next_aux = node_var_base;

        for sn in &nodes {
            if sn.var_index == CONST_INDEX {
                // Terminal ONE gets a dedicated aux var that will be asserted true
                // Terminal ZERO gets a var that will be asserted false
                seq_to_dimacs[sn.seq_id as usize] = next_aux;
                next_aux += 1;
            } else {
                seq_to_dimacs[sn.seq_id as usize] = next_aux;
                next_aux += 1;
            }
        }

        let total_vars = (next_aux - 1) as usize;

        // Collect clauses
        let mut clauses: Vec<Vec<i32>> = Vec::new();

        for sn in &nodes {
            let nv = seq_to_dimacs[sn.seq_id as usize];

            if sn.var_index == CONST_INDEX {
                if sn.is_one {
                    // Assert: ONE-node variable is true
                    clauses.push(vec![nv]);
                } else {
                    // Assert: ZERO-node variable is false
                    clauses.push(vec![-nv]);
                }
                continue;
            }

            // BDD variable for this node's decision variable
            let xi = sn.var_index as i32 + bdd_var_base;

            // Resolve child DIMACS literals (applying complement)
            let t_dimacs = {
                let base = seq_to_dimacs[sn.then_seq as usize];
                if sn.then_comp { -base } else { base }
            };
            let e_dimacs = {
                let base = seq_to_dimacs[sn.else_seq as usize];
                if sn.else_comp { -base } else { base }
            };

            // Encode: nv <=> ITE(xi, t, e)
            //
            // Forward implication (nv => ITE):
            //   nv => (xi => t)    : (!nv | !xi | t)
            //   nv => (!xi => e)   : (!nv | xi | e)
            //
            // Backward implication (ITE => nv):
            //   (xi & t) => nv     : (!xi | !t | nv)
            //   (!xi & e) => nv    : (xi | !e | nv)
            //
            // Also: (!t & !e) => !nv  (optional but helps)
            //   and: (t & e) => nv     (optional but helps)

            clauses.push(vec![-nv, -xi, t_dimacs]);
            clauses.push(vec![-nv, xi, e_dimacs]);
            clauses.push(vec![nv, -xi, -t_dimacs]);
            clauses.push(vec![nv, xi, -e_dimacs]);
        }

        // Assert the root
        let root_var = seq_to_dimacs[nodes.last().unwrap().seq_id as usize];
        let root_lit = if root_comp { -root_var } else { root_var };
        clauses.push(vec![root_lit]);

        // Write DIMACS header and clauses
        writeln!(out, "p cnf {total_vars} {}", clauses.len())?;
        for clause in &clauses {
            for &lit in clause {
                write!(out, "{lit} ")?;
            }
            writeln!(out, "0")?;
        }

        Ok(())
    }

    // ==================================================================
    // Variable-length integer encoding (LEB128-style unsigned)
    // ==================================================================

    /// Write a u32 in variable-length encoding (little-endian base-128).
    fn write_varint<W: Write>(out: &mut W, mut value: u32) -> io::Result<()> {
        loop {
            let mut byte = (value & 0x7F) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            out.write_all(&[byte])?;
            if value == 0 {
                break;
            }
        }
        Ok(())
    }

    /// Read a variable-length encoded u32. Returns `(value, bytes_consumed)`.
    fn read_varint(data: &[u8]) -> io::Result<(u32, usize)> {
        let mut result: u32 = 0;
        let mut shift: u32 = 0;
        for (i, &byte) in data.iter().enumerate() {
            if shift >= 35 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "varint too long",
                ));
            }
            result |= ((byte & 0x7F) as u32) << shift;
            shift += 7;
            if byte & 0x80 == 0 {
                return Ok((result, i + 1));
            }
        }
        Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "unterminated varint",
        ))
    }
}

// ======================================================================
// Tests
// ======================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    #[test]
    fn round_trip_text_simple_and() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y);

        // Save
        let mut buf = Vec::new();
        mgr.dddmp_save_text(f, Some(&["a", "b"]), &mut buf).unwrap();
        let text = String::from_utf8(buf.clone()).unwrap();
        assert!(text.contains(".ver DDDMP-2.0"));
        assert!(text.contains(".mode A"));

        // Load into a fresh manager
        let mut mgr2 = Manager::new();
        let mut reader = BufReader::new(buf.as_slice());
        let f2 = mgr2.dddmp_load_text(&mut reader).unwrap();

        // Verify the loaded BDD behaves like AND
        let x2 = mgr2.bdd_ith_var(0);
        let y2 = mgr2.bdd_ith_var(1);
        let and2 = mgr2.bdd_and(x2, y2);
        assert_eq!(f2, and2);
    }

    #[test]
    fn round_trip_text_not() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let f = mgr.bdd_not(x);

        let mut buf = Vec::new();
        mgr.dddmp_save_text(f, None, &mut buf).unwrap();

        let mut mgr2 = Manager::new();
        let mut reader = BufReader::new(buf.as_slice());
        let f2 = mgr2.dddmp_load_text(&mut reader).unwrap();

        let x2 = mgr2.bdd_ith_var(0);
        let not_x2 = mgr2.bdd_not(x2);
        assert_eq!(f2, not_x2);
    }

    #[test]
    fn round_trip_binary_or() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_or(x, y);

        let mut buf = Vec::new();
        mgr.dddmp_save_binary(f, &mut buf).unwrap();

        let mut mgr2 = Manager::new();
        let f2 = mgr2.dddmp_load_binary(&mut buf.as_slice()).unwrap();

        let x2 = mgr2.bdd_ith_var(0);
        let y2 = mgr2.bdd_ith_var(1);
        let or2 = mgr2.bdd_or(x2, y2);
        assert_eq!(f2, or2);
    }

    #[test]
    fn round_trip_binary_constants() {
        let mut mgr = Manager::new();

        // Save and load ONE
        let mut buf = Vec::new();
        mgr.dddmp_save_binary(NodeId::ONE, &mut buf).unwrap();
        let mut mgr2 = Manager::new();
        let one = mgr2.dddmp_load_binary(&mut buf.as_slice()).unwrap();
        assert!(one.is_one());

        // Save and load ZERO
        buf.clear();
        mgr.dddmp_save_binary(NodeId::ZERO, &mut buf).unwrap();
        let mut mgr3 = Manager::new();
        let zero = mgr3.dddmp_load_binary(&mut buf.as_slice()).unwrap();
        assert!(zero.is_zero());
    }

    #[test]
    fn cnf_export_simple() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_and(x, y);

        let mut buf = Vec::new();
        mgr.dddmp_save_cnf(f, &mut buf).unwrap();
        let cnf = String::from_utf8(buf).unwrap();
        assert!(cnf.starts_with("p cnf"));
        // Every non-header line should end with " 0"
        for line in cnf.lines().skip(1) {
            assert!(line.ends_with(" 0") || line.ends_with("0"));
        }
    }

    #[test]
    fn cnf_export_trivial() {
        let mut mgr = Manager::new();

        let mut buf = Vec::new();
        mgr.dddmp_save_cnf(NodeId::ONE, &mut buf).unwrap();
        let cnf = String::from_utf8(buf).unwrap();
        assert!(cnf.contains("p cnf 0 0"));

        let mut buf = Vec::new();
        mgr.dddmp_save_cnf(NodeId::ZERO, &mut buf).unwrap();
        let cnf = String::from_utf8(buf).unwrap();
        assert!(cnf.contains("p cnf 0 1"));
    }

    #[test]
    fn varint_round_trip() {
        for &val in &[0u32, 1, 127, 128, 255, 16384, 0x0FFF_FFFF, u32::MAX] {
            let mut buf = Vec::new();
            Manager::write_varint(&mut buf, val).unwrap();
            let (decoded, len) = Manager::read_varint(&buf).unwrap();
            assert_eq!(decoded, val);
            assert_eq!(len, buf.len());
        }
    }

    #[test]
    fn round_trip_text_xor() {
        let mut mgr = Manager::new();
        let x = mgr.bdd_new_var();
        let y = mgr.bdd_new_var();
        let f = mgr.bdd_xor(x, y);

        let mut buf = Vec::new();
        mgr.dddmp_save_text(f, None, &mut buf).unwrap();

        let mut mgr2 = Manager::new();
        let mut reader = BufReader::new(buf.as_slice());
        let f2 = mgr2.dddmp_load_text(&mut reader).unwrap();

        let x2 = mgr2.bdd_ith_var(0);
        let y2 = mgr2.bdd_ith_var(1);
        let xor2 = mgr2.bdd_xor(x2, y2);
        assert_eq!(f2, xor2);
    }
}
