use std::collections::HashMap;

use crate::error::{LayoutError, Result};
use crate::geometry::{Rect, Size};

/// Layout direction for a container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Row,
    Column,
}

/// Space distribution rules for child nodes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Constraint {
    Fixed(u16),
    Percent(u8),
    Min(u16),
    Max(u16),
    Flex(u16),
}

/// Unique identifier for layout nodes.
pub type NodeId = String;

/// Layout node representation (container or leaf).
#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub id: NodeId,
    pub direction: Direction,
    pub constraints: Vec<Constraint>,
    pub children: Vec<LayoutNode>,
    pub gap: u16,
    pub padding: u16,
}

impl LayoutNode {
    pub fn leaf(id: impl Into<NodeId>) -> Self {
        Self {
            id: id.into(),
            direction: Direction::Row,
            constraints: Vec::new(),
            children: Vec::new(),
            gap: 0,
            padding: 0,
        }
    }

    pub fn container(
        id: impl Into<NodeId>,
        direction: Direction,
        constraints: Vec<Constraint>,
        children: Vec<LayoutNode>,
    ) -> Self {
        Self {
            id: id.into(),
            direction,
            constraints,
            children,
            gap: 0,
            padding: 0,
        }
    }

    pub fn with_gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    pub fn with_padding(mut self, padding: u16) -> Self {
        self.padding = padding;
        self
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

/// Layout tree orchestrator.
#[derive(Debug, Clone)]
pub struct LayoutTree {
    pub root: LayoutNode,
}

impl LayoutTree {
    pub fn new(root: LayoutNode) -> Self {
        Self { root }
    }

    /// Solve the layout tree for a given terminal size, returning rects keyed by node id.
    pub fn solve(&self, size: Size) -> Result<HashMap<NodeId, Rect>> {
        if self.root.children.is_empty() {
            return Err(LayoutError::EmptyLayout);
        }

        let mut rects = HashMap::new();
        self.solve_node(
            &self.root,
            Rect::new(0, 0, size.width, size.height),
            &mut rects,
        )?;
        Ok(rects)
    }

    fn solve_node(
        &self,
        node: &LayoutNode,
        rect: Rect,
        accum: &mut HashMap<NodeId, Rect>,
    ) -> Result<()> {
        accum.insert(node.id.clone(), rect);

        if node.children.is_empty() {
            return Ok(());
        }

        let axis_length = match node.direction {
            Direction::Row => rect.width,
            Direction::Column => rect.height,
        };

        let available = axis_length.saturating_sub(node.padding.saturating_mul(2));
        let gap_total = if node.children.is_empty() {
            0
        } else {
            node.gap
                .saturating_mul(node.children.len().saturating_sub(1) as u16)
        };
        let distributable = available.saturating_sub(gap_total);

        let mut cursor = match node.direction {
            Direction::Row => rect.x + node.padding,
            Direction::Column => rect.y + node.padding,
        };

        let default_constraint = Constraint::Flex(1);

        let solved_segments = distribute(
            distributable,
            node.children.len(),
            &node.constraints,
            default_constraint,
        );

        for (idx, child) in node.children.iter().enumerate() {
            let span = solved_segments[idx];

            let child_rect = match node.direction {
                Direction::Row => Rect::new(
                    cursor,
                    rect.y + node.padding,
                    span,
                    rect.height.saturating_sub(node.padding.saturating_mul(2)),
                ),
                Direction::Column => Rect::new(
                    rect.x + node.padding,
                    cursor,
                    rect.width.saturating_sub(node.padding.saturating_mul(2)),
                    span,
                ),
            };

            self.solve_node(child, child_rect, accum)?;
            cursor = cursor.saturating_add(span).saturating_add(node.gap);
        }

        Ok(())
    }
}

fn distribute(
    distributable: u16,
    child_count: usize,
    raw_constraints: &[Constraint],
    default_constraint: Constraint,
) -> Vec<u16> {
    if child_count == 0 {
        return Vec::new();
    }

    let mut segments = build_segments(
        child_count,
        raw_constraints,
        default_constraint,
        distributable,
    );

    let total_available = distributable as u32;
    let mut used: u32 = segments.iter().map(|s| s.length).sum();

    if used > total_available {
        shrink_segments(&mut segments, used - total_available);
        used = segments.iter().map(|s| s.length).sum();
    }

    let remaining = total_available.saturating_sub(used);
    if remaining > 0 {
        distribute_flex(&mut segments, remaining);
    }

    segments
        .into_iter()
        .map(|segment| segment.length.min(u16::MAX as u32) as u16)
        .collect()
}

#[derive(Debug, Clone)]
struct Segment {
    length: u32,
    min: u32,
    max: Option<u32>,
    flex: u32,
    locked: bool,
}

fn build_segments(
    child_count: usize,
    raw_constraints: &[Constraint],
    default_constraint: Constraint,
    distributable: u16,
) -> Vec<Segment> {
    let mut segments = vec![
        Segment {
            length: 0,
            min: 0,
            max: None,
            flex: match default_constraint {
                Constraint::Flex(weight) => weight.max(1) as u32,
                _ => 1,
            } as u32,
            locked: matches!(
                default_constraint,
                Constraint::Fixed(_) | Constraint::Percent(_)
            ),
        };
        child_count
    ];

    for (idx, segment) in segments.iter_mut().enumerate() {
        if idx < raw_constraints.len() {
            apply_constraint(segment, raw_constraints[idx], distributable);
        } else {
            apply_constraint(segment, default_constraint, distributable);
        }
    }

    segments
}

fn apply_constraint(segment: &mut Segment, constraint: Constraint, distributable: u16) {
    match constraint {
        Constraint::Fixed(value) => {
            segment.length = value as u32;
            segment.min = value as u32;
            segment.max = Some(value as u32);
            segment.flex = 0;
            segment.locked = true;
        }
        Constraint::Percent(percent) => {
            let value = ((distributable as f32) * (percent as f32 / 100.0)).round() as u32;
            segment.length = value;
            segment.min = value;
            segment.max = Some(value);
            segment.flex = 0;
            segment.locked = true;
        }
        Constraint::Min(min) => {
            segment.length = min as u32;
            segment.min = min as u32;
            segment.flex = segment.flex.max(1);
            segment.locked = false;
        }
        Constraint::Max(max) => {
            segment.max = Some(max as u32);
            segment.flex = segment.flex.max(1);
            segment.locked = false;
        }
        Constraint::Flex(weight) => {
            segment.flex = weight.max(1) as u32;
            segment.locked = false;
        }
    }
}

fn shrink_segments(segments: &mut [Segment], mut over: u32) {
    if over == 0 {
        return;
    }

    while over > 0 {
        let mut changed = false;
        for segment in segments.iter_mut() {
            if segment.length > segment.min && (!segment.locked || segment.length > segment.min) {
                segment.length -= 1;
                over = over.saturating_sub(1);
                changed = true;
                if over == 0 {
                    break;
                }
            }
        }

        if !changed {
            break;
        }
    }
}

fn distribute_flex(segments: &mut [Segment], remaining: u32) {
    if segments.is_empty() {
        return;
    }

    let total_flex: u32 = segments.iter().map(|s| s.flex).sum();
    if total_flex == 0 {
        return;
    }

    let mut leftover = remaining;
    for segment in segments.iter_mut() {
        if segment.flex == 0 {
            continue;
        }

        let share = (remaining * segment.flex) / total_flex;
        let mut addition = share.min(leftover);
        let max_allow = segment
            .max
            .map(|max| max.saturating_sub(segment.length))
            .unwrap_or(u32::MAX);
        if addition > max_allow {
            addition = max_allow;
        }

        segment.length = segment.length.saturating_add(addition);
        leftover = leftover.saturating_sub(addition);
    }

    if leftover == 0 {
        return;
    }

    let count = segments.len();
    let mut idx = 0;
    let mut attempts = 0;
    while leftover > 0 && attempts < count * 4 {
        let segment = &mut segments[idx % count];
        let max_allow = segment
            .max
            .map(|max| max.saturating_sub(segment.length))
            .unwrap_or(u32::MAX);
        if segment.flex > 0 && max_allow > 0 {
            segment.length += 1;
            leftover -= 1;
        }
        idx += 1;
        attempts += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distribute_single_child() {
        let values = distribute(80, 1, &[], Constraint::Flex(1));
        assert_eq!(values, vec![80]);
    }

    #[test]
    fn row_layout_with_mixed_constraints() {
        let root = LayoutNode {
            id: "root".into(),
            direction: Direction::Row,
            constraints: vec![
                Constraint::Fixed(20),
                Constraint::Percent(25),
                Constraint::Flex(1),
            ],
            children: vec![
                LayoutNode::leaf("left"),
                LayoutNode::leaf("middle"),
                LayoutNode::leaf("right"),
            ],
            gap: 2,
            padding: 1,
        };

        let tree = LayoutTree::new(root);
        let rects = tree.solve(Size::new(100, 20)).unwrap();

        assert_eq!(rects.get("left").unwrap().width, 20);
        assert_eq!(rects.get("middle").unwrap().width, 24);
        assert_eq!(rects.get("right").unwrap().width, 50);
        assert_eq!(rects.get("left").unwrap().x, 1);
        assert_eq!(rects.get("middle").unwrap().x, 23);
        assert_eq!(rects.get("right").unwrap().x, 49);
        assert_eq!(rects.get("right").unwrap().height, 18);
    }

    #[test]
    fn column_layout_respects_min_and_max() {
        let root = LayoutNode {
            id: "root".into(),
            direction: Direction::Column,
            constraints: vec![Constraint::Min(6), Constraint::Max(4), Constraint::Flex(1)],
            children: vec![
                LayoutNode::leaf("top"),
                LayoutNode::leaf("middle"),
                LayoutNode::leaf("bottom"),
            ],
            gap: 1,
            padding: 1,
        };

        let tree = LayoutTree::new(root);
        let rects = tree.solve(Size::new(40, 20)).unwrap();

        let top = rects.get("top").unwrap();
        let middle = rects.get("middle").unwrap();
        let bottom = rects.get("bottom").unwrap();

        assert!(top.height >= 6);
        assert!(middle.height <= 4);
        assert!(bottom.height > 0);

        let total = top.height + middle.height + bottom.height + 2 + 2; // gaps + padding*2
        assert_eq!(total, 20);

        assert_eq!(middle.y, top.bottom() + 1);
        assert_eq!(bottom.y, middle.bottom() + 1);
    }
}
