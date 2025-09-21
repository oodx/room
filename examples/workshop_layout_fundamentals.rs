//! Room Workshop: Layout Fundamentals
//!
//! This self-guided workshop demonstrates how `LayoutTree` solves constraints in the
//! Room layout engine. Run different scenarios to inspect how nodes are measured and
//! positioned.
//!
//! ```bash
//! cargo run --example workshop_layout_fundamentals                # default (basic)
//! cargo run --example workshop_layout_fundamentals -- with-gap    # gap + padding
//! cargo run --example workshop_layout_fundamentals -- nested      # nested sidebar
//! ```
//!
//! Each run prints the resolved rectangles for the configured terminal size along
//! with guidance for experimentation.

use std::collections::BTreeMap;

use room_mvp::{Constraint, Direction, LayoutNode, LayoutTree, Result, Size};

fn main() -> Result<()> {
    let scenario = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "basic".to_string());

    let (tree, description) = match scenario.as_str() {
        "basic" => basic_three_row_layout(),
        "with-gap" | "with_gap" => layout_with_gap_and_padding(),
        "nested" => nested_sidebar_layout(),
        other => {
            eprintln!("Unknown scenario `{other}`. Try `basic`, `with-gap`, or `nested`.");
            std::process::exit(1);
        }
    };

    println!("Room Workshop: Layout Fundamentals");
    println!("Scenario: {scenario}\n");
    println!("{description}");

    inspect_layout(tree, Size::new(80, 24))?;
    Ok(())
}

/// Scenario 1: Basic three-row layout with fixed header/footer and flexible body.
fn basic_three_row_layout() -> (LayoutTree, &'static str) {
    let tree = LayoutTree::new(LayoutNode {
        id: "workshop:root".into(),
        direction: Direction::Column,
        constraints: vec![
            Constraint::Fixed(3),
            Constraint::Flex(1),
            Constraint::Fixed(3),
        ],
        children: vec![
            LayoutNode::leaf("workshop:header"),
            LayoutNode::leaf("workshop:body"),
            LayoutNode::leaf("workshop:footer"),
        ],
        gap: 0,
        padding: 0,
    });

    let description = "Step 1: Inspect the classic header/body/footer composition.\n\
                       • Header/footer are fixed to 3 rows each.\n\
                       • The body consumes remaining vertical space via FLEX(1).\n\
                       Try editing the constraints or terminal size in the source.";

    (tree, description)
}

/// Scenario 2: Demonstrates padding and gaps in a column layout.
fn layout_with_gap_and_padding() -> (LayoutTree, &'static str) {
    let tree = LayoutTree::new(LayoutNode {
        id: "workshop:root".into(),
        direction: Direction::Column,
        constraints: vec![
            Constraint::Fixed(4),
            Constraint::Flex(1),
            Constraint::Fixed(4),
        ],
        children: vec![
            LayoutNode::leaf("workshop:toolbar"),
            LayoutNode::leaf("workshop:content"),
            LayoutNode::leaf("workshop:status"),
        ],
        gap: 1,
        padding: 1,
    });

    let description = "Step 2: Explore how padding and gap influence allocations.\n\
                       • Padding clamps the usable area before allocation.\n\
                       • Gap consumes space between siblings.\n\
                       Change the padding or gap values and re-run to observe effects.";

    (tree, description)
}

/// Scenario 3: Adds a nested row layout with a sidebar to illustrate composition.
fn nested_sidebar_layout() -> (LayoutTree, &'static str) {
    let body = LayoutNode {
        id: "workshop:body".into(),
        direction: Direction::Row,
        constraints: vec![Constraint::Fixed(24), Constraint::Flex(2)],
        children: vec![
            LayoutNode::leaf("workshop:body.sidebar"),
            LayoutNode::leaf("workshop:body.timeline"),
        ],
        gap: 1,
        padding: 1,
    };

    let tree = LayoutTree::new(LayoutNode {
        id: "workshop:root".into(),
        direction: Direction::Column,
        constraints: vec![
            Constraint::Fixed(3),
            Constraint::Flex(1),
            Constraint::Fixed(3),
        ],
        children: vec![
            LayoutNode::leaf("workshop:header"),
            body,
            LayoutNode::leaf("workshop:footer"),
        ],
        gap: 1,
        padding: 0,
    });

    let description = "Step 3: Nest layouts to build a classic sidebar timeline view.\n\
                       • Outer column reserves header/footer.\n\
                       • Inner row creates a fixed-width sidebar and flexible main panel.\n\
                       Try resizing the terminal (adjust Size::new) to observe recalculation.";

    (tree, description)
}

fn inspect_layout(tree: LayoutTree, size: Size) -> Result<()> {
    let solved = tree.solve(size)?;
    let ordered: BTreeMap<_, _> = solved.into_iter().collect();

    println!("\nTerminal Size: {size:?}\n");
    println!("Resolved Rects (id → Rect):\n-------------------------------");
    for (id, rect) in ordered {
        println!("{id:<30} -> {:?}", rect);
    }

    println!(
        "\nWorkshop Tip: edit the source to add new nodes or tweak constraints, then rerun to \nsee how the solver reacts."
    );
    Ok(())
}
