//! Grid layout system - declarative 2D layouts with rows, columns, and cell placement
//!
//! This module provides a CSS Grid-inspired layout system that replaces the nested
//! flexbox-style LayoutTree. Key features:
//! - Define all rows and columns upfront (no nesting required)
//! - Zone placement via GridArea (cell coordinates and spanning)
//! - Automatic space distribution with Fixed, Flex, and Percent sizing
//! - Fail-fast validation (bounds checking, overlap detection)
//!
//! # Example
//! ```no_run
//! use room_mvp::layout::grid::{GridLayout, GridSize, GridArea};
//!
//! let mut grid = GridLayout::new();
//!
//! // Define grid structure
//! grid.add_col(GridSize::Fixed(20))       // Sidebar: 20 chars
//!     .add_col(GridSize::flex(2))         // Main: 2x flex
//!     .add_col(GridSize::flex(1));        // Side: 1x flex
//!
//! grid.add_row(GridSize::Fixed(3))        // Header: 3 rows
//!     .add_row(GridSize::flex(1))         // Body: fills space
//!     .add_row(GridSize::Fixed(2));       // Footer: 2 rows
//!
//! // Place zones
//! grid.place("header", GridArea::span_cols(0, 0..3))?;  // Spans all columns
//! grid.place("sidebar", GridArea::cell(1, 0))?;
//! grid.place("main", GridArea::cell(1, 1))?;
//! # Ok::<(), room_mvp::layout::grid::GridError>(())
//! ```

use crate::{Rect, Size};
use std::collections::HashMap;
use std::num::{NonZeroU16, NonZeroU8};
use std::ops::Range;

/// Zone identifier
pub type ZoneId = String;

/// Defines how a column or row should be sized
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridSize {
    /// Fixed size in terminal cells
    Fixed(u16),

    /// Flexible size (proportional units, like CSS 'fr')
    /// Flex(2) takes 2x the space of Flex(1)
    /// Must be non-zero - zero would create a track that absorbs no space
    Flex(NonZeroU16),

    /// Percentage of available space (1-100)
    /// If total percentages exceed 100%, they will be normalized proportionally
    /// If total is less than 100%, remaining space goes to Flex tracks
    Percent(NonZeroU8),
}

impl GridSize {
    /// Create Flex variant
    ///
    /// # Panics
    /// Panics if n is zero
    pub fn flex(n: u16) -> Self {
        Self::Flex(NonZeroU16::new(n).expect("Flex size must be non-zero"))
    }

    /// Create Percent variant
    ///
    /// # Panics
    /// Panics if n is zero or greater than 100
    pub fn percent(n: u8) -> Self {
        assert!(n > 0 && n <= 100, "Percent must be 1-100");
        Self::Percent(NonZeroU8::new(n).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_size_fixed() {
        let size = GridSize::Fixed(20);
        assert_eq!(size, GridSize::Fixed(20));
    }

    #[test]
    fn test_grid_size_flex() {
        let size = GridSize::flex(2);
        match size {
            GridSize::Flex(n) => assert_eq!(n.get(), 2),
            _ => panic!("Expected Flex variant"),
        }
    }

    #[test]
    #[should_panic(expected = "Flex size must be non-zero")]
    fn test_grid_size_flex_zero_panics() {
        GridSize::flex(0);
    }

    #[test]
    fn test_grid_size_percent() {
        let size = GridSize::percent(30);
        match size {
            GridSize::Percent(n) => assert_eq!(n.get(), 30),
            _ => panic!("Expected Percent variant"),
        }
    }

    #[test]
    #[should_panic(expected = "Percent must be 1-100")]
    fn test_grid_size_percent_zero_panics() {
        GridSize::percent(0);
    }

    #[test]
    #[should_panic(expected = "Percent must be 1-100")]
    fn test_grid_size_percent_over_100_panics() {
        GridSize::percent(101);
    }

    #[test]
    fn test_grid_size_percent_edge_cases() {
        let min = GridSize::percent(1);
        match min {
            GridSize::Percent(n) => assert_eq!(n.get(), 1),
            _ => panic!("Expected Percent variant"),
        }

        let max = GridSize::percent(100);
        match max {
            GridSize::Percent(n) => assert_eq!(n.get(), 100),
            _ => panic!("Expected Percent variant"),
        }
    }
}