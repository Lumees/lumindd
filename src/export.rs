// lumindd — Export formats: BLIF, DaVinci, factored form
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::HashSet;
use std::io::{self, Write};

use crate::manager::Manager;
use crate::node::NodeId;

impl Manager {
    // ==================================================================
    // BLIF export
    // ==================================================================

    /// Export a BDD to Berkeley Logic Interchange Format (BLIF).
    ///
    /// BLIF represents a logic network as a set of `.names` tables, each
    /// defining a single-output combinational logic function. For a BDD,
    /// each internal node becomes a `.names` entry: the node's output is 1
    /// when the variable is true and the then-child is 1, or when the variable
    /// is false and the else-child is 1.
    ///
    /// `var_names` optionally provides human-readable names for input variables.
    /// `output_name` is the name for the primary output signal.
    pub fn dump_blif<W: Write>(
        &self,
        f: NodeId,
        var_names: Option<&[&str]>,
        output_name: &str,
        out: &mut W,
    ) -> io::Result<()> {
        writeln!(out, ".model bdd")?;

        // Collect all variable indices used
        let support = self.bdd_support(f);

        // Write inputs
        write!(out, ".inputs")?;
        for &var in &support {
            let name = var_names
                .and_then(|names| names.get(var as usize).copied())
                .unwrap_or("");
            if name.is_empty() {
                write!(out, " x{}", var)?;
            } else {
                write!(out, " {}", name)?;
            }
        }
        writeln!(out)?;

        // Write output
        writeln!(out, ".outputs {}", output_name)?;

        // Handle constant cases
        if f.is_one() {
            writeln!(out, ".names {}", output_name)?;
            writeln!(out, "1")?;
            writeln!(out, ".end")?;
            return Ok(());
        }
        if f.is_zero() {
            writeln!(out, ".names {}", output_name)?;
            // No rows means output is always 0
            writeln!(out, ".end")?;
            return Ok(());
        }

        // Traverse the BDD and emit .names for each internal node
        let mut visited = HashSet::new();
        self.dump_blif_rec(f, var_names, out, &mut visited)?;

        // Connect the root to the output
        let root_name = self.blif_node_name(f);
        if root_name != output_name {
            writeln!(out, ".names {} {}", root_name, output_name)?;
            writeln!(out, "1 1")?;
        }

        writeln!(out, ".end")?;
        Ok(())
    }

    fn dump_blif_rec<W: Write>(
        &self,
        f: NodeId,
        var_names: Option<&[&str]>,
        out: &mut W,
        visited: &mut HashSet<u32>,
    ) -> io::Result<()> {
        if f.is_constant() {
            return Ok(());
        }

        let reg = f.regular();
        let raw = reg.raw_index();
        if !visited.insert(raw) {
            return Ok(());
        }

        let var = self.var_index(reg);
        let t = self.then_child(f.regular());
        let e = self.else_child(f.regular());

        // Recurse into children first
        self.dump_blif_rec(t, var_names, out, visited)?;
        self.dump_blif_rec(e, var_names, out, visited)?;

        let var_name = var_names
            .and_then(|names| names.get(var as usize).copied())
            .unwrap_or("");
        let var_label = if var_name.is_empty() {
            format!("x{}", var)
        } else {
            var_name.to_string()
        };

        let t_name = self.blif_node_name(t);
        let e_name = self.blif_node_name(e);
        let node_name = format!("n{}", raw);

        // If the edge to this node is complemented, we handle it at the output.
        // For the .names table: the node output is 1 when
        //   (var=1 AND then_child=1) OR (var=0 AND else_child=1)

        // Handle then-child
        if t.is_one() && e.is_zero() {
            // Simple buffer: output = var
            writeln!(out, ".names {} {}", var_label, node_name)?;
            writeln!(out, "1 1")?;
        } else if t.is_zero() && e.is_one() {
            // Inverter: output = !var
            writeln!(out, ".names {} {}", var_label, node_name)?;
            writeln!(out, "0 1")?;
        } else if t.is_one() {
            // output = var OR else_child
            writeln!(out, ".names {} {} {}", var_label, e_name, node_name)?;
            writeln!(out, "1- 1")?;
            writeln!(out, "-1 1")?;
        } else if t.is_zero() {
            // output = !var AND else_child
            writeln!(out, ".names {} {} {}", var_label, e_name, node_name)?;
            writeln!(out, "01 1")?;
        } else if e.is_one() {
            // output = !var OR then_child
            writeln!(out, ".names {} {} {}", var_label, t_name, node_name)?;
            writeln!(out, "0- 1")?;
            writeln!(out, "-1 1")?;
        } else if e.is_zero() {
            // output = var AND then_child
            writeln!(out, ".names {} {} {}", var_label, t_name, node_name)?;
            writeln!(out, "11 1")?;
        } else {
            // General case: ITE(var, then, else)
            writeln!(
                out,
                ".names {} {} {} {}",
                var_label, t_name, e_name, node_name
            )?;
            writeln!(out, "11- 1")?;
            writeln!(out, "0-1 1")?;
        }

        Ok(())
    }

    /// Get the BLIF signal name for a node.
    fn blif_node_name(&self, f: NodeId) -> String {
        if f.is_one() {
            // Need a constant-one signal
            "\\$true".to_string()
        } else if f.is_zero() {
            "\\$false".to_string()
        } else {
            let reg = f.regular();
            let base = format!("n{}", reg.raw_index());
            if f.is_complemented() {
                format!("n{}_inv", reg.raw_index())
            } else {
                base
            }
        }
    }

    // ==================================================================
    // DaVinci graph format export
    // ==================================================================

    /// Export a BDD to DaVinci graph visualization format.
    ///
    /// DaVinci uses a term-based representation where each node is described
    /// as `l("id", n("label", [attrs], [children]))`. Edges are represented
    /// as `l("edge_id", e("label", [attrs], r("target_id")))`.
    pub fn dump_davinci<W: Write>(&self, f: NodeId, out: &mut W) -> io::Result<()> {
        writeln!(out, "[")?;

        if f.is_constant() {
            let label = if f.is_one() { "1" } else { "0" };
            writeln!(
                out,
                "  l(\"root\", n(\"{}\",[a(\"OBJECT\",\"{}\"),a(\"COLOR\",\"#AAAAAA\")],[]))",
                label, label
            )?;
            writeln!(out, "]")?;
            return Ok(());
        }

        let mut visited = HashSet::new();
        let mut nodes_output = Vec::new();
        self.davinci_collect_nodes(f, &mut visited, &mut nodes_output);

        // Add terminal nodes
        nodes_output.push("  l(\"ONE\", n(\"1\",[a(\"OBJECT\",\"1\"),a(\"COLOR\",\"#00CC00\"),a(\"_GO\",\"box\")],[]))".to_string());
        nodes_output.push("  l(\"ZERO\", n(\"0\",[a(\"OBJECT\",\"0\"),a(\"COLOR\",\"#CC0000\"),a(\"_GO\",\"box\")],[]))".to_string());

        let total = nodes_output.len();
        for (i, node_str) in nodes_output.iter().enumerate() {
            if i + 1 < total {
                writeln!(out, "{},", node_str)?;
            } else {
                writeln!(out, "{}", node_str)?;
            }
        }

        writeln!(out, "]")?;
        Ok(())
    }

    fn davinci_collect_nodes(
        &self,
        f: NodeId,
        visited: &mut HashSet<u32>,
        output: &mut Vec<String>,
    ) {
        if f.is_constant() {
            return;
        }

        let reg = f.regular();
        let raw = reg.raw_index();
        if !visited.insert(raw) {
            return;
        }

        let var = self.var_index(reg);
        let t = self.raw_then(f);
        let e = self.raw_else(f);

        // Recurse first
        self.davinci_collect_nodes(t, visited, output);
        self.davinci_collect_nodes(e, visited, output);

        // Build edges
        let t_target = self.davinci_node_id(t);
        let e_target = self.davinci_node_id(e);

        let t_color = if t.is_complemented() {
            "#FF0000"
        } else {
            "#000000"
        };
        let e_color = if e.is_complemented() {
            "#FF0000"
        } else {
            "#0000FF"
        };

        let node_str = format!(
            "  l(\"n{}\", n(\"x{}\",[a(\"OBJECT\",\"x{}\"),a(\"COLOR\",\"#4444FF\")],[\
             l(\"n{}_t\", e(\"then\",[a(\"EDGECOLOR\",\"{}\")],r(\"{}\"))),\
             l(\"n{}_e\", e(\"else\",[a(\"EDGECOLOR\",\"{}\"),a(\"EDGEPATTERN\",\"dashed\")],r(\"{}\")))]\
             ))",
            raw, var, var, raw, t_color, t_target, raw, e_color, e_target
        );

        output.push(node_str);
    }

    fn davinci_node_id(&self, f: NodeId) -> String {
        if f.is_one() || (f.is_constant() && !f.is_complemented()) {
            "ONE".to_string()
        } else if f.is_zero() || (f.is_constant() && f.is_complemented()) {
            "ZERO".to_string()
        } else {
            format!("n{}", f.regular().raw_index())
        }
    }

    // ==================================================================
    // Factored form export
    // ==================================================================

    /// Convert a BDD to a human-readable factored form expression.
    ///
    /// Produces expressions like `(x0 & (x1 | !x2))`. The result is a
    /// minimal factored form derived directly from the BDD structure:
    /// each internal node becomes `(var & then_expr | !var & else_expr)`.
    pub fn dump_factored_form(&self, f: NodeId) -> String {
        if f.is_one() {
            return "1".to_string();
        }
        if f.is_zero() {
            return "0".to_string();
        }

        self.factored_form_rec(f)
    }

    fn factored_form_rec(&self, f: NodeId) -> String {
        if f.is_one() {
            return "1".to_string();
        }
        if f.is_zero() {
            return "0".to_string();
        }

        let var = self.var_index(f.regular());
        let t = self.then_child(f);
        let e = self.else_child(f);

        let var_str = format!("x{}", var);

        let t_str = self.factored_form_rec(t);
        let e_str = self.factored_form_rec(e);

        // Simplify common patterns
        if t.is_one() && e.is_zero() {
            // f = var
            return var_str;
        }
        if t.is_zero() && e.is_one() {
            // f = !var
            return format!("!{}", var_str);
        }
        if e.is_zero() {
            // f = var & then
            if t.is_one() {
                return var_str;
            }
            return format!("({} & {})", var_str, t_str);
        }
        if t.is_zero() {
            // f = !var & else
            if e.is_one() {
                return format!("!{}", var_str);
            }
            return format!("(!{} & {})", var_str, e_str);
        }
        if t.is_one() {
            // f = var | else
            return format!("({} | {})", var_str, e_str);
        }
        if e.is_one() {
            // f = !var | then = var -> then, else -> 1
            return format!("(!{} | {})", var_str, t_str);
        }

        // General case: ITE(var, then, else) = (var & then) | (!var & else)
        format!("({} & {} | !{} & {})", var_str, t_str, var_str, e_str)
    }

    // ==================================================================
    // Enhanced DOT export with highlighting
    // ==================================================================

    /// Export a BDD to DOT format with highlighted nodes.
    ///
    /// Nodes whose `NodeId` (regular form) appears in `highlight` are drawn
    /// with a distinct fill color, making it easy to visualize subsets,
    /// critical paths, or debugging targets.
    pub fn dump_dot_color<W: Write>(
        &self,
        f: NodeId,
        highlight: &[NodeId],
        out: &mut W,
    ) -> io::Result<()> {
        let highlight_set: HashSet<u32> = highlight.iter().map(|n| n.raw_index()).collect();

        writeln!(out, "digraph BDD {{")?;
        writeln!(out, "  rankdir=TB;")?;
        writeln!(out, "  node [style=filled];")?;
        writeln!(out, "  // Terminal nodes")?;
        writeln!(out, "  {{ rank=sink;")?;
        writeln!(
            out,
            "    ONE [shape=box, label=\"1\", fillcolor=\"#90EE90\"];"
        )?;
        writeln!(
            out,
            "    ZERO [shape=box, label=\"0\", fillcolor=\"#FFB6C1\"];"
        )?;
        writeln!(out, "  }}")?;

        let mut visited = HashSet::new();
        self.dump_dot_color_rec(f, &highlight_set, out, &mut visited)?;

        writeln!(out, "}}")?;
        Ok(())
    }

    fn dump_dot_color_rec<W: Write>(
        &self,
        f: NodeId,
        highlight_set: &HashSet<u32>,
        out: &mut W,
        visited: &mut HashSet<u32>,
    ) -> io::Result<()> {
        if f.is_constant() {
            return Ok(());
        }

        let raw = f.raw_index();
        if !visited.insert(raw) {
            return Ok(());
        }

        let var = self.var_index(f.regular());
        let t = self.raw_then(f);
        let e = self.raw_else(f);

        let fill_color = if highlight_set.contains(&raw) {
            "#FFD700" // gold for highlighted
        } else {
            "#ADD8E6" // light blue for normal
        };

        writeln!(
            out,
            "  n{} [label=\"x{}\", shape=ellipse, fillcolor=\"{}\"];",
            raw, var, fill_color
        )?;

        // Then edge (solid)
        let t_label = self.dot_node_label(t);
        let t_style = if t.is_complemented() {
            "style=solid, color=red"
        } else {
            "style=solid, color=black"
        };
        writeln!(out, "  n{} -> {} [{}];", raw, t_label, t_style)?;

        // Else edge (dashed)
        let e_label = self.dot_node_label(e);
        let e_style = if e.is_complemented() {
            "style=dashed, color=red"
        } else {
            "style=dashed, color=blue"
        };
        writeln!(out, "  n{} -> {} [{}];", raw, e_label, e_style)?;

        self.dump_dot_color_rec(t.regular(), highlight_set, out, visited)?;
        self.dump_dot_color_rec(e.regular(), highlight_set, out, visited)?;

        Ok(())
    }

    fn dot_node_label(&self, f: NodeId) -> String {
        if f.is_one() {
            "ONE".to_string()
        } else if f.is_zero() {
            "ZERO".to_string()
        } else {
            format!("n{}", f.raw_index())
        }
    }

    // ==================================================================
    // Truth table export
    // ==================================================================

    /// Print the full truth table of a BDD.
    ///
    /// Lists all 2^n variable assignments and whether the function evaluates
    /// to 0 or 1 for each. The output format is:
    ///
    /// ```text
    /// x0 x1 x2 | f
    ///  0  0  0 | 0
    ///  0  0  1 | 1
    ///  ...
    /// ```
    ///
    /// Warning: exponential in the number of variables. Intended for small
    /// functions (up to ~20 variables).
    pub fn dump_truth_table<W: Write>(&self, f: NodeId, out: &mut W) -> io::Result<()> {
        let n = self.num_vars as usize;

        if n > 24 {
            writeln!(
                out,
                "// Truth table too large: {} variables ({} rows)",
                n,
                1u64 << n
            )?;
            return Ok(());
        }

        // Header
        for i in 0..n {
            if i > 0 {
                write!(out, " ")?;
            }
            write!(out, "x{}", i)?;
        }
        writeln!(out, " | f")?;

        // Separator
        for i in 0..n {
            if i > 0 {
                write!(out, " ")?;
            }
            // Width matches "xN" where N can be multi-digit
            let width = format!("x{}", i).len();
            for _ in 0..width {
                write!(out, "-")?;
            }
        }
        writeln!(out, "-+--")?;

        // Rows
        let num_rows = 1u64 << n;
        let mut assignment = vec![false; n];

        for row in 0..num_rows {
            // Decode row into assignment
            for j in 0..n {
                assignment[j] = (row >> (n - 1 - j)) & 1 == 1;
            }

            // Print assignment
            for j in 0..n {
                if j > 0 {
                    write!(out, " ")?;
                }
                let width = format!("x{}", j).len();
                let val = if assignment[j] { "1" } else { "0" };
                // Right-align the value in the column
                for _ in 0..width - 1 {
                    write!(out, " ")?;
                }
                write!(out, "{}", val)?;
            }

            // Evaluate the BDD
            let result = self.bdd_eval(f, &assignment);
            writeln!(out, " | {}", if result { "1" } else { "0" })?;
        }

        Ok(())
    }
}
