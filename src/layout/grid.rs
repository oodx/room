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

/// Grid error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GridError {
    OutOfBounds {
        zone: ZoneId,
        row_start: usize,
        row_end: usize,
        row_count: usize,
        col_start: usize,
        col_end: usize,
        col_count: usize,
    },
    DuplicateZone {
        zone: ZoneId,
    },
    OverlappingAreas {
        zone1: ZoneId,
        zone2: ZoneId,
    },
}

impl std::fmt::Display for GridError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GridError::OutOfBounds {
                zone,
                row_start,
                row_end,
                row_count,
                col_start,
                col_end,
                col_count,
            } => write!(
                f,
                "Zone '{}' out of bounds: rows {}..{} (grid has {}), cols {}..{} (grid has {})",
                zone, row_start, row_end, row_count, col_start, col_end, col_count
            ),
            GridError::DuplicateZone { zone } => {
                write!(f, "Zone '{}' already exists in grid", zone)
            }
            GridError::OverlappingAreas { zone1, zone2 } => {
                write!(f, "Zones '{}' and '{}' overlap", zone1, zone2)
            }
        }
    }
}

impl std::error::Error for GridError {}

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

/// Defines the placement of a zone within the grid
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridArea {
    pub rows: Range<usize>,
    pub cols: Range<usize>,
}

impl GridArea {
    pub fn new(rows: Range<usize>, cols: Range<usize>) -> Self {
        Self { rows, cols }
    }

    pub fn cell(row: usize, col: usize) -> Self {
        Self {
            rows: row..row + 1,
            cols: col..col + 1,
        }
    }

    pub fn span_rows(col: usize, rows: Range<usize>) -> Self {
        Self {
            rows,
            cols: col..col + 1,
        }
    }

    pub fn span_cols(row: usize, cols: Range<usize>) -> Self {
        Self {
            rows: row..row + 1,
            cols,
        }
    }

    pub(crate) fn validate(
        self,
        zone: &ZoneId,
        row_count: usize,
        col_count: usize,
    ) -> Result<Self, GridError> {
        if self.rows.end > row_count || self.cols.end > col_count {
            return Err(GridError::OutOfBounds {
                zone: zone.clone(),
                row_start: self.rows.start,
                row_end: self.rows.end,
                row_count,
                col_start: self.cols.start,
                col_end: self.cols.end,
                col_count,
            });
        }
        Ok(self)
    }

    pub(crate) fn overlaps(&self, other: &GridArea) -> bool {
        let rows_overlap = self.rows.start < other.rows.end && other.rows.start < self.rows.end;
        let cols_overlap = self.cols.start < other.cols.end && other.cols.start < self.cols.end;
        rows_overlap && cols_overlap
    }
}

/// Grid layout container with builder pattern
pub struct GridLayout {
    cols: Vec<GridSize>,
    rows: Vec<GridSize>,
    areas: HashMap<ZoneId, GridArea>,
    gap: u16,
}

impl GridLayout {
    pub fn new() -> Self {
        Self {
            cols: Vec::new(),
            rows: Vec::new(),
            areas: HashMap::new(),
            gap: 0,
        }
    }

    pub fn add_col(&mut self, size: GridSize) -> &mut Self {
        self.cols.push(size);
        self
    }

    pub fn add_row(&mut self, size: GridSize) -> &mut Self {
        self.rows.push(size);
        self
    }

    pub fn with_gap(&mut self, gap: u16) -> &mut Self {
        self.gap = gap;
        self
    }

    pub fn place(
        &mut self,
        zone_id: impl Into<ZoneId>,
        area: GridArea,
    ) -> Result<&mut Self, GridError> {
        let zone_id = zone_id.into();

        if self.areas.contains_key(&zone_id) {
            return Err(GridError::DuplicateZone { zone: zone_id });
        }

        let area = area.validate(&zone_id, self.rows.len(), self.cols.len())?;

        for (existing_id, existing_area) in &self.areas {
            if existing_area.overlaps(&area) {
                return Err(GridError::OverlappingAreas {
                    zone1: existing_id.clone(),
                    zone2: zone_id,
                });
            }
        }

        self.areas.insert(zone_id, area);
        Ok(self)
    }

    pub fn solve(&self, total_size: Size) -> Result<HashMap<ZoneId, Rect>, GridError> {
        let (col_offsets, col_sizes) = Self::solve_axis(&self.cols, total_size.width, self.gap);
        let (row_offsets, row_sizes) = Self::solve_axis(&self.rows, total_size.height, self.gap);

        let mut result = HashMap::new();

        for (zone_id, area) in &self.areas {
            let x = col_offsets[area.cols.start];
            let y = row_offsets[area.rows.start];

            let width: u16 = col_sizes[area.cols.start..area.cols.end].iter().sum::<u16>()
                + self.gap.saturating_mul((area.cols.end - area.cols.start).saturating_sub(1) as u16);
            let height: u16 = row_sizes[area.rows.start..area.rows.end].iter().sum::<u16>()
                + self.gap.saturating_mul((area.rows.end - area.rows.start).saturating_sub(1) as u16);

            let width = width.min(total_size.width.saturating_sub(x));
            let height = height.min(total_size.height.saturating_sub(y));

            result.insert(
                zone_id.clone(),
                Rect {
                    x,
                    y,
                    width,
                    height,
                },
            );
        }

        Ok(result)
    }

    fn solve_axis(tracks: &[GridSize], total: u16, gap: u16) -> (Vec<u16>, Vec<u16>) {
        if tracks.is_empty() {
            return (vec![0], vec![]);
        }

        let track_count = tracks.len();
        let total_gap = gap.saturating_mul((track_count.saturating_sub(1)) as u16);

        let available = if total_gap >= total {
            0
        } else {
            total - total_gap
        };

        let mut track_sizes = vec![0u16; track_count];
        let mut remaining = available;

        let mut fixed_sum = 0u16;
        for track in tracks {
            if let GridSize::Fixed(size) = track {
                fixed_sum = fixed_sum.saturating_add(*size);
            }
        }
        remaining = remaining.saturating_sub(fixed_sum);

        let total_percent: u32 = tracks
            .iter()
            .filter_map(|t| match t {
                GridSize::Percent(p) => Some(p.get() as u32),
                _ => None,
            })
            .sum();

        let mut percent_used = 0u16;
        if total_percent > 0 {
            for track in tracks {
                if let GridSize::Percent(p) = track {
                    let weight = if total_percent > 100 {
                        (p.get() as u32 * 100) / total_percent
                    } else {
                        p.get() as u32
                    };
                    let size = ((remaining as u32 * weight) / 100) as u16;
                    percent_used = percent_used.saturating_add(size);
                }
            }
        }
        remaining = remaining.saturating_sub(percent_used);

        let flex_sum: u32 = tracks
            .iter()
            .filter_map(|t| match t {
                GridSize::Flex(f) => Some(f.get() as u32),
                _ => None,
            })
            .sum();

        for (i, track) in tracks.iter().enumerate() {
            match track {
                GridSize::Fixed(size) => {
                    track_sizes[i] = *size;
                }
                GridSize::Percent(p) => {
                    let weight = if total_percent > 100 {
                        (p.get() as u32 * 100) / total_percent
                    } else {
                        p.get() as u32
                    };
                    let size = (((available.saturating_sub(fixed_sum)) as u32 * weight) / 100) as u16;
                    track_sizes[i] = size;
                }
                GridSize::Flex(f) => {
                    if flex_sum > 0 {
                        let size = ((remaining as u32 * f.get() as u32) / flex_sum) as u16;
                        track_sizes[i] = size;
                    }
                }
            }
        }

        let mut remaining_for_min = available;
        for i in 0..track_sizes.len() {
            if track_sizes[i] == 0 && remaining_for_min > 0 {
                track_sizes[i] = 1;
                remaining_for_min = remaining_for_min.saturating_sub(1);
            }
        }

        let assigned_sum: u16 = track_sizes.iter().sum();
        let mut leftover = available.saturating_sub(assigned_sum);

        for (i, track) in tracks.iter().enumerate() {
            if leftover == 0 {
                break;
            }
            if matches!(track, GridSize::Flex(_)) {
                track_sizes[i] = track_sizes[i].saturating_add(1);
                leftover = leftover.saturating_sub(1);
            }
        }

        if leftover > 0 {
            for (i, track) in tracks.iter().enumerate() {
                if leftover == 0 {
                    break;
                }
                if matches!(track, GridSize::Percent(_)) {
                    track_sizes[i] = track_sizes[i].saturating_add(1);
                    leftover = leftover.saturating_sub(1);
                }
            }
        }

        let mut offsets: Vec<u16> = Vec::with_capacity(track_count + 1);
        let mut cumulative: u16 = 0;
        offsets.push(cumulative);

        for (i, &size) in track_sizes.iter().enumerate() {
            cumulative = cumulative.saturating_add(size);

            if i < track_count - 1 && cumulative < total {
                let gap_to_add = gap.min(total.saturating_sub(cumulative));
                cumulative = cumulative.saturating_add(gap_to_add);
            }

            cumulative = cumulative.min(total);
            offsets.push(cumulative);
        }

        (offsets, track_sizes)
    }
}

impl Default for GridLayout {
    fn default() -> Self {
        Self::new()
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

    #[test]
    fn test_grid_area_cell() {
        let area = GridArea::cell(2, 3);
        assert_eq!(area.rows, 2..3);
        assert_eq!(area.cols, 3..4);
    }

    #[test]
    fn test_grid_area_new() {
        let area = GridArea::new(1..4, 2..5);
        assert_eq!(area.rows, 1..4);
        assert_eq!(area.cols, 2..5);
    }

    #[test]
    fn test_grid_area_span_rows() {
        let area = GridArea::span_rows(2, 0..3);
        assert_eq!(area.rows, 0..3);
        assert_eq!(area.cols, 2..3);
    }

    #[test]
    fn test_grid_area_span_cols() {
        let area = GridArea::span_cols(1, 0..4);
        assert_eq!(area.rows, 1..2);
        assert_eq!(area.cols, 0..4);
    }

    #[test]
    fn test_grid_area_validate_success() {
        let area = GridArea::new(0..2, 0..3);
        let result = area.validate(&"test".to_string(), 3, 4);
        assert!(result.is_ok());
    }

    #[test]
    fn test_grid_area_validate_out_of_bounds_rows() {
        let area = GridArea::new(0..5, 0..2);
        let result = area.validate(&"test".to_string(), 3, 4);
        assert!(result.is_err());
        match result {
            Err(GridError::OutOfBounds {
                zone,
                row_end,
                row_count,
                ..
            }) => {
                assert_eq!(zone, "test");
                assert_eq!(row_end, 5);
                assert_eq!(row_count, 3);
            }
            _ => panic!("Expected OutOfBounds error"),
        }
    }

    #[test]
    fn test_grid_area_validate_out_of_bounds_cols() {
        let area = GridArea::new(0..2, 0..5);
        let result = area.validate(&"test".to_string(), 3, 4);
        assert!(result.is_err());
        match result {
            Err(GridError::OutOfBounds {
                zone,
                col_end,
                col_count,
                ..
            }) => {
                assert_eq!(zone, "test");
                assert_eq!(col_end, 5);
                assert_eq!(col_count, 4);
            }
            _ => panic!("Expected OutOfBounds error"),
        }
    }

    #[test]
    fn test_grid_area_overlaps_true() {
        let area1 = GridArea::new(1..3, 1..3);
        let area2 = GridArea::new(2..4, 2..4);
        assert!(area1.overlaps(&area2));
        assert!(area2.overlaps(&area1));
    }

    #[test]
    fn test_grid_area_overlaps_false_rows() {
        let area1 = GridArea::new(0..2, 1..3);
        let area2 = GridArea::new(2..4, 1..3);
        assert!(!area1.overlaps(&area2));
        assert!(!area2.overlaps(&area1));
    }

    #[test]
    fn test_grid_area_overlaps_false_cols() {
        let area1 = GridArea::new(1..3, 0..2);
        let area2 = GridArea::new(1..3, 2..4);
        assert!(!area1.overlaps(&area2));
        assert!(!area2.overlaps(&area1));
    }

    #[test]
    fn test_grid_area_overlaps_adjacent() {
        let area1 = GridArea::new(0..2, 0..2);
        let area2 = GridArea::new(0..2, 2..4);
        assert!(!area1.overlaps(&area2));
    }

    #[test]
    fn test_grid_area_overlaps_contained() {
        let area1 = GridArea::new(0..4, 0..4);
        let area2 = GridArea::new(1..3, 1..3);
        assert!(area1.overlaps(&area2));
        assert!(area2.overlaps(&area1));
    }

    #[test]
    fn test_grid_layout_builder() {
        let mut grid = GridLayout::new();
        grid.add_col(GridSize::Fixed(20))
            .add_col(GridSize::flex(1))
            .add_row(GridSize::Fixed(3))
            .add_row(GridSize::flex(1))
            .with_gap(1);

        assert_eq!(grid.cols.len(), 2);
        assert_eq!(grid.rows.len(), 2);
        assert_eq!(grid.gap, 1);
    }

    #[test]
    fn test_grid_layout_place_success() {
        let mut grid = GridLayout::new();
        grid.add_col(GridSize::Fixed(20))
            .add_col(GridSize::flex(1))
            .add_row(GridSize::Fixed(3))
            .add_row(GridSize::flex(1));

        let result = grid.place("header", GridArea::span_cols(0, 0..2));
        assert!(result.is_ok());
        assert_eq!(grid.areas.len(), 1);
    }

    #[test]
    fn test_grid_layout_place_duplicate() {
        let mut grid = GridLayout::new();
        grid.add_col(GridSize::Fixed(20))
            .add_row(GridSize::Fixed(3));

        grid.place("test", GridArea::cell(0, 0)).unwrap();
        let result = grid.place("test", GridArea::cell(0, 0));

        assert!(matches!(result, Err(GridError::DuplicateZone { .. })));
    }

    #[test]
    fn test_grid_layout_place_out_of_bounds() {
        let mut grid = GridLayout::new();
        grid.add_col(GridSize::Fixed(20))
            .add_row(GridSize::Fixed(3));

        let result = grid.place("test", GridArea::cell(5, 0));
        assert!(matches!(result, Err(GridError::OutOfBounds { .. })));
    }

    #[test]
    fn test_grid_layout_place_overlapping() {
        let mut grid = GridLayout::new();
        grid.add_col(GridSize::Fixed(20))
            .add_col(GridSize::flex(1))
            .add_row(GridSize::Fixed(3))
            .add_row(GridSize::flex(1));

        grid.place("zone1", GridArea::new(0..2, 0..2)).unwrap();
        let result = grid.place("zone2", GridArea::new(1..2, 1..2));

        assert!(matches!(result, Err(GridError::OverlappingAreas { .. })));
    }

    #[test]
    fn test_grid_layout_solve_fixed_only() {
        let mut grid = GridLayout::new();
        grid.add_col(GridSize::Fixed(20))
            .add_col(GridSize::Fixed(30))
            .add_row(GridSize::Fixed(5))
            .add_row(GridSize::Fixed(10));

        grid.place("a", GridArea::cell(0, 0)).unwrap();
        grid.place("b", GridArea::cell(0, 1)).unwrap();
        grid.place("c", GridArea::cell(1, 0)).unwrap();
        grid.place("d", GridArea::cell(1, 1)).unwrap();

        let result = grid.solve(Size::new(100, 50)).unwrap();

        assert_eq!(result["a"], Rect { x: 0, y: 0, width: 20, height: 5 });
        assert_eq!(result["b"], Rect { x: 20, y: 0, width: 30, height: 5 });
        assert_eq!(result["c"], Rect { x: 0, y: 5, width: 20, height: 10 });
        assert_eq!(result["d"], Rect { x: 20, y: 5, width: 30, height: 10 });
    }

    #[test]
    fn test_grid_layout_solve_with_gap() {
        let mut grid = GridLayout::new();
        grid.add_col(GridSize::Fixed(20))
            .add_col(GridSize::Fixed(30))
            .add_row(GridSize::Fixed(5))
            .with_gap(2);

        grid.place("a", GridArea::cell(0, 0)).unwrap();
        grid.place("b", GridArea::cell(0, 1)).unwrap();

        let result = grid.solve(Size::new(100, 50)).unwrap();

        assert_eq!(result["a"], Rect { x: 0, y: 0, width: 20, height: 5 });
        assert_eq!(result["b"], Rect { x: 22, y: 0, width: 30, height: 5 });
    }

    #[test]
    fn test_grid_layout_solve_flex_distribution() {
        let mut grid = GridLayout::new();
        grid.add_col(GridSize::flex(1))
            .add_col(GridSize::flex(2))
            .add_row(GridSize::flex(1));

        grid.place("left", GridArea::cell(0, 0)).unwrap();
        grid.place("right", GridArea::cell(0, 1)).unwrap();

        let result = grid.solve(Size::new(90, 20)).unwrap();

        assert_eq!(result["left"].width, 30);
        assert_eq!(result["right"].width, 60);
    }

    #[test]
    fn test_grid_layout_solve_percent() {
        let mut grid = GridLayout::new();
        grid.add_col(GridSize::percent(30))
            .add_col(GridSize::percent(70))
            .add_row(GridSize::Fixed(10));

        grid.place("left", GridArea::cell(0, 0)).unwrap();
        grid.place("right", GridArea::cell(0, 1)).unwrap();

        let result = grid.solve(Size::new(100, 20)).unwrap();

        assert_eq!(result["left"].width, 30);
        assert_eq!(result["right"].width, 70);
    }

    #[test]
    fn test_grid_layout_solve_mixed() {
        let mut grid = GridLayout::new();
        grid.add_col(GridSize::Fixed(20))
            .add_col(GridSize::flex(2))
            .add_col(GridSize::flex(1))
            .add_row(GridSize::Fixed(5));

        grid.place("sidebar", GridArea::cell(0, 0)).unwrap();
        grid.place("main", GridArea::cell(0, 1)).unwrap();
        grid.place("aside", GridArea::cell(0, 2)).unwrap();

        let result = grid.solve(Size::new(110, 20)).unwrap();

        assert_eq!(result["sidebar"].width, 20);
        assert_eq!(result["main"].width, 60);
        assert_eq!(result["aside"].width, 30);
    }

    #[test]
    fn test_grid_layout_solve_spanning() {
        let mut grid = GridLayout::new();
        grid.add_col(GridSize::Fixed(30))
            .add_col(GridSize::Fixed(30))
            .add_col(GridSize::Fixed(30))
            .add_row(GridSize::Fixed(5))
            .add_row(GridSize::Fixed(10));

        grid.place("header", GridArea::span_cols(0, 0..3)).unwrap();
        grid.place("content", GridArea::cell(1, 1)).unwrap();

        let result = grid.solve(Size::new(100, 50)).unwrap();

        assert_eq!(result["header"], Rect { x: 0, y: 0, width: 90, height: 5 });
        assert_eq!(result["content"], Rect { x: 30, y: 5, width: 30, height: 10 });
    }

    #[test]
    fn test_grid_layout_solve_percent_only_leftover_redistribution() {
        let mut grid = GridLayout::new();
        grid.add_col(GridSize::percent(33))
            .add_col(GridSize::percent(33))
            .add_col(GridSize::percent(33))
            .add_row(GridSize::Fixed(10));

        grid.place("a", GridArea::cell(0, 0)).unwrap();
        grid.place("b", GridArea::cell(0, 1)).unwrap();
        grid.place("c", GridArea::cell(0, 2)).unwrap();

        let result = grid.solve(Size::new(100, 20)).unwrap();

        let total_width = result["a"].width + result["b"].width + result["c"].width;
        assert_eq!(total_width, 100);
    }

    #[test]
    fn test_grid_layout_solve_small_percent_no_vanishing() {
        let mut grid = GridLayout::new();
        grid.add_col(GridSize::percent(1))
            .add_col(GridSize::percent(99))
            .add_row(GridSize::Fixed(10));

        grid.place("tiny", GridArea::cell(0, 0)).unwrap();
        grid.place("large", GridArea::cell(0, 1)).unwrap();

        let result = grid.solve(Size::new(50, 20)).unwrap();

        assert!(result["tiny"].width >= 1);
        assert!(result["large"].width >= 1);

        let total_width = result["tiny"].width + result["large"].width;
        assert_eq!(total_width, 50);
    }

    #[test]
    fn test_grid_layout_solve_vanishing_track_guaranteed_minimum() {
        let mut grid = GridLayout::new();
        grid.add_col(GridSize::percent(1))
            .add_col(GridSize::percent(1))
            .add_col(GridSize::percent(98))
            .add_row(GridSize::Fixed(10));

        grid.place("a", GridArea::cell(0, 0)).unwrap();
        grid.place("b", GridArea::cell(0, 1)).unwrap();
        grid.place("c", GridArea::cell(0, 2)).unwrap();

        let result = grid.solve(Size::new(100, 20)).unwrap();

        assert_eq!(result["a"].width, 1);
        assert_eq!(result["b"].width, 1);
        assert_eq!(result["c"].width, 98);
    }

    #[test]
    fn test_grid_layout_solve_percent_normalization_with_leftover() {
        let mut grid = GridLayout::new();
        grid.add_col(GridSize::percent(50))
            .add_col(GridSize::percent(50))
            .add_col(GridSize::percent(50))
            .add_row(GridSize::Fixed(10));

        grid.place("a", GridArea::cell(0, 0)).unwrap();
        grid.place("b", GridArea::cell(0, 1)).unwrap();
        grid.place("c", GridArea::cell(0, 2)).unwrap();

        let result = grid.solve(Size::new(100, 20)).unwrap();

        let total_width = result["a"].width + result["b"].width + result["c"].width;
        assert_eq!(total_width, 100);
    }

    #[test]
    fn test_grid_layout_solve_over_constrained_budget() {
        let mut grid = GridLayout::new();
        grid.add_col(GridSize::percent(33))
            .add_col(GridSize::percent(33))
            .add_col(GridSize::percent(33))
            .add_row(GridSize::Fixed(10));

        grid.place("a", GridArea::cell(0, 0)).unwrap();
        grid.place("b", GridArea::cell(0, 1)).unwrap();
        grid.place("c", GridArea::cell(0, 2)).unwrap();

        let result = grid.solve(Size::new(1, 20)).unwrap();

        let total_width = result["a"].width + result["b"].width + result["c"].width;
        assert!(total_width <= 1, "Total width {} exceeds available width 1", total_width);

        assert!(result["a"].x + result["a"].width <= 1);
        assert!(result["b"].x + result["b"].width <= 1);
        assert!(result["c"].x + result["c"].width <= 1);
    }

    #[test]
    fn test_grid_layout_solve_more_tracks_than_pixels() {
        let mut grid = GridLayout::new();
        grid.add_col(GridSize::percent(25))
            .add_col(GridSize::percent(25))
            .add_col(GridSize::percent(25))
            .add_col(GridSize::percent(25))
            .add_row(GridSize::Fixed(10));

        grid.place("a", GridArea::cell(0, 0)).unwrap();
        grid.place("b", GridArea::cell(0, 1)).unwrap();
        grid.place("c", GridArea::cell(0, 2)).unwrap();
        grid.place("d", GridArea::cell(0, 3)).unwrap();

        let result = grid.solve(Size::new(2, 20)).unwrap();

        let total_width = result["a"].width + result["b"].width + result["c"].width + result["d"].width;
        assert_eq!(total_width, 2);

        assert!(result["a"].x <= 2);
        assert!(result["b"].x <= 2);
        assert!(result["c"].x <= 2);
        assert!(result["d"].x <= 2);
    }

    #[test]
    fn test_grid_layout_solve_gap_larger_than_terminal() {
        let mut grid = GridLayout::new();
        grid.add_col(GridSize::Fixed(10))
            .add_col(GridSize::Fixed(10))
            .add_row(GridSize::Fixed(5))
            .with_gap(100);

        grid.place("a", GridArea::cell(0, 0)).unwrap();
        grid.place("b", GridArea::cell(0, 1)).unwrap();

        let result = grid.solve(Size::new(50, 20)).unwrap();

        assert!(result["a"].x + result["a"].width <= 50, "Zone 'a' exceeds terminal width");
        assert!(result["b"].x + result["b"].width <= 50, "Zone 'b' exceeds terminal width");
        assert!(result["a"].x <= 50);
        assert!(result["b"].x <= 50);
    }

    #[test]
    fn test_grid_layout_solve_gap_equals_terminal_width() {
        let mut grid = GridLayout::new();
        grid.add_col(GridSize::Fixed(5))
            .add_col(GridSize::Fixed(5))
            .add_row(GridSize::Fixed(5))
            .with_gap(10);

        grid.place("a", GridArea::cell(0, 0)).unwrap();
        grid.place("b", GridArea::cell(0, 1)).unwrap();

        let result = grid.solve(Size::new(10, 20)).unwrap();

        assert!(result["a"].x <= 10);
        assert!(result["b"].x <= 10);
        assert!(result["a"].x + result["a"].width <= 10);
        assert!(result["b"].x + result["b"].width <= 10);
    }

    #[test]
    fn test_grid_layout_solve_excessive_gap_with_small_terminal() {
        let mut grid = GridLayout::new();
        grid.add_col(GridSize::percent(50))
            .add_col(GridSize::percent(50))
            .add_row(GridSize::Fixed(5))
            .with_gap(5);

        grid.place("a", GridArea::cell(0, 0)).unwrap();
        grid.place("b", GridArea::cell(0, 1)).unwrap();

        let result = grid.solve(Size::new(2, 20)).unwrap();

        assert!(result["a"].x <= 2, "Zone 'a' x position {} exceeds terminal width 2", result["a"].x);
        assert!(result["b"].x <= 2, "Zone 'b' x position {} exceeds terminal width 2", result["b"].x);

        let max_x_a = result["a"].x + result["a"].width;
        let max_x_b = result["b"].x + result["b"].width;
        assert!(max_x_a <= 2, "Zone 'a' right edge {} exceeds terminal width 2", max_x_a);
        assert!(max_x_b <= 2, "Zone 'b' right edge {} exceeds terminal width 2", max_x_b);
    }

    #[test]
    fn test_grid_layout_solve_gap_consumes_all_space() {
        let mut grid = GridLayout::new();
        grid.add_col(GridSize::flex(1))
            .add_col(GridSize::flex(1))
            .add_col(GridSize::flex(1))
            .add_row(GridSize::Fixed(10))
            .with_gap(50);

        grid.place("a", GridArea::cell(0, 0)).unwrap();
        grid.place("b", GridArea::cell(0, 1)).unwrap();
        grid.place("c", GridArea::cell(0, 2)).unwrap();

        let result = grid.solve(Size::new(100, 20)).unwrap();

        for zone in ["a", "b", "c"] {
            let rect = &result[zone];
            assert!(rect.x <= 100, "Zone '{}' x position exceeds terminal", zone);
            assert!(rect.x + rect.width <= 100, "Zone '{}' extends beyond terminal", zone);
        }
    }
}