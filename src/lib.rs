// lumindd — Decision diagram library for BDD, ADD, and ZDD manipulation
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

#![allow(clippy::needless_range_loop)]
#![allow(clippy::manual_memcpy)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::only_used_in_recursion)]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::doc_lazy_continuation)]
#![allow(clippy::ptr_arg)]

//! # lumindd
//!
//! A pure Rust decision diagram library by [Lumees Lab](https://lumeeslab.com) ([GitHub](https://github.com/Lumees)),
//! providing manipulation of:
//!
//! - **BDD** (Binary Decision Diagrams) — represent Boolean functions
//! - **ADD** (Algebraic Decision Diagrams) — represent functions from Boolean domains to real values
//! - **ZDD** (Zero-suppressed Decision Diagrams) — represent families of sets
//!
//! Inspired by the CUDD library architecture, reimplemented from scratch in safe Rust
//! with complemented edges, dynamic variable reordering, and operation caching.
//!
//! ## Quick Start
//!
//! ```rust
//! use lumindd::Manager;
//!
//! let mut mgr = Manager::new();
//! let x = mgr.bdd_new_var();   // variable x0
//! let y = mgr.bdd_new_var();   // variable x1
//!
//! let f = mgr.bdd_and(x, y);   // f = x0 AND x1
//! let g = mgr.bdd_or(x, y);    // g = x0 OR x1
//! let h = mgr.bdd_not(f);      // h = NOT(x0 AND x1) = NAND
//!
//! let taut = mgr.bdd_or(f, h);
//! assert!(mgr.bdd_is_tautology(taut)); // f OR NOT(f) = 1
//! ```
//!
//! ## Architecture
//!
//! The library uses an arena-based design where all decision diagram nodes live
//! inside a [`Manager`]. Nodes are referenced by [`NodeId`] handles that encode
//! a complemented-edge bit in the LSB for O(1) negation.

mod node;
mod manager;
mod unique_table;
mod computed_table;
mod bdd;
mod add;
mod zdd;
mod zdd_advanced;
mod zdd_reorder;
mod reorder;
pub mod mtr;
mod reorder_symmetric;
mod reorder_group;
mod reorder_linear;
mod reorder_annealing;
mod reorder_genetic;
mod reorder_exact;
mod interact;
mod util;
mod export;
mod dddmp;
mod compose_adv;
mod bdd_approx;
mod bdd_priority;
mod bdd_decomp;
mod bdd_correl;
mod bdd_clip;
mod bdd_clause;
mod bdd_sign;
pub mod epd;
pub mod apa;
mod count_ext;
mod reorder_window4;
mod bdd_misc;
mod add_walsh;
mod harwell;
pub mod local_cache;
mod level_queue;
mod add_abstract;
mod add_matrix;
mod zdd_extra;
mod zdd_extra2;
mod add_extra;
mod bdd_approx_extra;
mod bdd_extra;
mod bdd_transfer;
mod manager_accessors;
mod reorder_dispatch;
mod debug;

pub use manager::Manager;
pub use debug::ManagerStats;
pub use node::NodeId;
pub use reorder::ReorderingMethod;
pub use reorder_dispatch::ExtReorderMethod;
pub use mtr::{GroupFlags, MtrTree};
pub use add::{AddOp, AddMonadicOp};
pub use epd::EpDouble;
pub use apa::ApInt;
pub use bdd_clip::ClipDirection;
pub use bdd_clause::Literal;
pub use harwell::HarwellMatrix;
pub use level_queue::LevelQueue;
pub use local_cache::LocalCache;
