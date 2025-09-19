//! Layout module orchestrator following the RSB module specification.
//!
//! Downstream crates and examples import layout types from here while the
//! implementation details live in the private `core` module.

mod core;

pub use core::{Constraint, Direction, LayoutNode, LayoutTree, NodeId};
