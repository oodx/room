//! Layout module orchestrator following the RSB module specification.
//!
//! Downstream crates and examples import layout types from here while the
//! implementation details live in the private `core` module.

use std::collections::HashMap;

use crate::{Rect, Result, Size};

mod core;
pub mod grid;

pub use core::{Constraint, Direction, LayoutNode, LayoutTree, NodeId};
pub use grid::{GridArea, GridError, GridLayout, GridSize, ZoneId};

/// Unified layout interface for calculating zone rectangles from terminal size.
pub trait Layout {
    /// Calculate zone rectangles for the given terminal size.
    fn solve(&self, size: Size) -> Result<HashMap<String, Rect>>;
}

impl Layout for GridLayout {
    fn solve(&self, size: Size) -> Result<HashMap<String, Rect>> {
        GridLayout::solve(self, size).map_err(Into::into)
    }
}

impl Layout for LayoutTree {
    fn solve(&self, size: Size) -> Result<HashMap<String, Rect>> {
        LayoutTree::solve(self, size)
    }
}
