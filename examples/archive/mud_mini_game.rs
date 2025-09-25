//! Room Example: MUD Mini Game
//!
//! Navigate a tiny Room-powered dungeon crawl. Use the arrow keys to walk between
//! rooms, read descriptions, and grab a few sparkling gems. Number keys trigger
//! contextual actions (look around, collect or drop gems, inspect your inventory).
//!
//! ```bash
//! cargo run --example mud_mini_game
//! ```

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use room_mvp::runtime::FocusChange;
use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, Direction, LayoutNode, LayoutTree, LegacyScreenStrategy,
    Result, RoomPlugin, RoomRuntime, RuntimeConfig, RuntimeContext, RuntimeEvent, ScreenDefinition,
    ScreenManager, Size,
};

const MAP_ZONE: &str = "mud:map";
const DESCRIPTION_ZONE: &str = "mud:description";
const MENU_ZONE: &str = "mud:menu";
const INVENTORY_ZONE: &str = "mud:inventory";
const STATUS_ZONE: &str = "mud:status";

fn main() -> Result<()> {
    println!("Room Example · MUD Mini Game\n");
    println!("Use the arrow keys to explore. Number keys trigger actions. Press Esc to exit.\n");

    let layout = build_layout();
    let renderer = AnsiRenderer::with_default();
    let mut config = RuntimeConfig::default();
    config.default_focus_zone = Some(MENU_ZONE.to_string());

    let mut runtime =
        RoomRuntime::with_config(layout.clone(), renderer, Size::new(90, 28), config)?;

    let mut screen_manager = ScreenManager::new();
    screen_manager.register_screen(ScreenDefinition::new(
        "mud-mini",
        "Room MUD Mini Game",
        std::sync::Arc::new(move || Box::new(LegacyScreenStrategy::new(layout.clone()))),
    ));
    runtime.set_screen_manager(screen_manager);
    runtime.activate_screen("mud-mini")?;

    runtime.register_plugin(MudGamePlugin::new());

    CliDriver::new(runtime)
        .run()
        .map_err(|err| room_mvp::LayoutError::Backend(err.to_string()))
}

fn build_layout() -> LayoutTree {
    LayoutTree::new(LayoutNode {
        id: "mud:root".into(),
        direction: Direction::Column,
        constraints: vec![
            Constraint::Fixed(9),
            Constraint::Flex(1),
            Constraint::Fixed(6),
            Constraint::Fixed(2),
        ],
        children: vec![
            LayoutNode {
                id: "mud:top".into(),
                direction: Direction::Row,
                constraints: vec![Constraint::Fixed(32), Constraint::Flex(1)],
                children: vec![
                    LayoutNode::leaf(MAP_ZONE),
                    LayoutNode::leaf(DESCRIPTION_ZONE),
                ],
                gap: 1,
                padding: 0,
            },
            LayoutNode {
                id: "mud:middle".into(),
                direction: Direction::Row,
                constraints: vec![Constraint::Flex(1), Constraint::Fixed(28)],
                children: vec![
                    LayoutNode::leaf(MENU_ZONE),
                    LayoutNode::leaf(INVENTORY_ZONE),
                ],
                gap: 1,
                padding: 0,
            },
            LayoutNode::leaf(STATUS_ZONE),
        ],
        gap: 1,
        padding: 1,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum CardinalDirection {
    North,
    South,
    East,
    West,
}

impl CardinalDirection {
    fn from_key(code: KeyCode) -> Option<Self> {
        match code {
            KeyCode::Up => Some(Self::North),
            KeyCode::Down => Some(Self::South),
            KeyCode::Left => Some(Self::West),
            KeyCode::Right => Some(Self::East),
            _ => None,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::North => "North",
            Self::South => "South",
            Self::East => "East",
            Self::West => "West",
        }
    }
}

struct Room {
    id: String,
    name: String,
    description: String,
    neighbors: HashMap<CardinalDirection, String>,
    gems: Vec<String>,
}

struct RoomSeed {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    position: (i32, i32),
    neighbors: &'static [(CardinalDirection, &'static str)],
    gems: &'static [&'static str],
}

impl RoomSeed {
    const fn new(
        id: &'static str,
        name: &'static str,
        description: &'static str,
        position: (i32, i32),
        neighbors: &'static [(CardinalDirection, &'static str)],
        gems: &'static [&'static str],
    ) -> Self {
        Self {
            id,
            name,
            description,
            position,
            neighbors,
            gems,
        }
    }
}

const ROOM_SEEDS: &[RoomSeed] = &[
    RoomSeed::new(
        "atrium",
        "Atrium",
        "Sunlight filters through a cracked glass ceiling. Dust motes shimmer over a mosaic floor.",
        (0, 0),
        &[
            (CardinalDirection::North, "observatory"),
            (CardinalDirection::East, "gallery"),
            (CardinalDirection::South, "library"),
        ],
        &["Sunshard"],
    ),
    RoomSeed::new(
        "observatory",
        "Observatory",
        "Ancient telescopes point toward painted stars. The air smells faintly of ozone.",
        (0, 1),
        &[
            (CardinalDirection::South, "atrium"),
            (CardinalDirection::East, "workshop"),
        ],
        &["Stargazer Gem"],
    ),
    RoomSeed::new(
        "gallery",
        "Gallery",
        "Portraits of long-forgotten explorers hang askew. A velvet rope guards nothing in particular.",
        (1, 0),
        &[
            (CardinalDirection::West, "atrium"),
            (CardinalDirection::South, "vault"),
            (CardinalDirection::East, "garden"),
        ],
        &["Amber Fragment"],
    ),
    RoomSeed::new(
        "library",
        "Library",
        "Shelves bow under the weight of leather-bound tomes. A gentle breeze flips a page now and then.",
        (0, -1),
        &[
            (CardinalDirection::North, "atrium"),
            (CardinalDirection::East, "vault"),
        ],
        &[],
    ),
    RoomSeed::new(
        "vault",
        "Vault",
        "A heavy door hangs open. Inside, ornate lockboxes line the walls, most pried open long ago.",
        (1, -1),
        &[
            (CardinalDirection::North, "gallery"),
            (CardinalDirection::West, "library"),
            (CardinalDirection::East, "garden"),
        ],
        &["Prismatic Core"],
    ),
    RoomSeed::new(
        "workshop",
        "Workshop",
        "Tools float midair, frozen by some forgotten spell. The hum of latent energy fills the space.",
        (1, 1),
        &[
            (CardinalDirection::West, "observatory"),
            (CardinalDirection::South, "gallery"),
        ],
        &[],
    ),
    RoomSeed::new(
        "garden",
        "Glass Garden",
        "Crystal flowers chime softly when the breeze moves them. Pathways spiral around reflective pools.",
        (2, 0),
        &[
            (CardinalDirection::West, "gallery"),
            (CardinalDirection::South, "vault"),
        ],
        &["Iridescent Bloom"],
    ),
];

struct MudGamePlugin {
    rooms: HashMap<String, Room>,
    positions: HashMap<(i32, i32), String>,
    current_room: String,
    inventory: Vec<String>,
    status: String,
    selected_action: usize,
    menu_has_focus: bool,
}

impl MudGamePlugin {
    fn new() -> Self {
        let mut rooms = HashMap::new();
        let mut positions = HashMap::new();

        for seed in ROOM_SEEDS {
            positions.insert(seed.position, seed.id.to_string());
            rooms.insert(
                seed.id.to_string(),
                Room {
                    id: seed.id.to_string(),
                    name: seed.name.to_string(),
                    description: seed.description.to_string(),
                    neighbors: seed
                        .neighbors
                        .iter()
                        .map(|(dir, target)| (*dir, (*target).to_string()))
                        .collect(),
                    gems: seed.gems.iter().map(|g| (*g).to_string()).collect(),
                },
            );
        }

        Self {
            rooms,
            positions,
            current_room: "atrium".to_string(),
            inventory: Vec::new(),
            status: String::from("You step into the atrium. Sunlight warms the stone."),
            selected_action: 0,
            menu_has_focus: true,
        }
    }

    fn update_all(&mut self, ctx: &mut RuntimeContext<'_>) {
        self.normalize_selection();
        ctx.set_zone(MAP_ZONE, self.render_map());
        ctx.set_zone(DESCRIPTION_ZONE, self.render_description());
        ctx.set_zone(MENU_ZONE, self.render_menu());
        ctx.set_zone(INVENTORY_ZONE, self.render_inventory());
        ctx.set_zone(STATUS_ZONE, self.render_status());

        if self.menu_has_focus {
            ctx.show_cursor();
            ctx.set_cursor_in_zone(MENU_ZONE, (self.selected_action + 1) as i32, 2);
        } else {
            ctx.hide_cursor();
        }
    }

    fn normalize_selection(&mut self) {
        let len = self.available_actions().len();
        if len == 0 {
            self.selected_action = 0;
        } else if self.selected_action >= len {
            self.selected_action = len - 1;
        }
    }

    fn render_map(&self) -> String {
        let (min_x, max_x, min_y, max_y) = self
            .positions
            .keys()
            .fold((i32::MAX, i32::MIN, i32::MAX, i32::MIN), |acc, &(x, y)| {
                (acc.0.min(x), acc.1.max(x), acc.2.min(y), acc.3.max(y))
            });

        let mut lines = vec!["Map (arrow keys to move)".to_string()];
        for y in (min_y..=max_y).rev() {
            let mut line = String::new();
            for x in min_x..=max_x {
                if let Some(id) = self.positions.get(&(x, y)) {
                    if id == &self.current_room {
                        line.push_str("[*]");
                    } else {
                        line.push_str("[ ]");
                    }
                } else {
                    line.push_str("   ");
                }
            }
            lines.push(line);
        }

        lines.push(String::new());
        lines.push("Legend:".to_string());
        for room in self.rooms.values() {
            let marker = if room.id == self.current_room {
                "[*]"
            } else {
                "[ ]"
            };
            lines.push(format!("{marker} {}", room.name));
        }
        lines.join("\n")
    }

    fn render_description(&self) -> String {
        let room = self
            .rooms
            .get(&self.current_room)
            .expect("current room exists");
        let mut output = format!("Room: {}\n{}\n\n", room.name, room.description);

        if room.gems.is_empty() {
            output.push_str("No loose gems glitter here.\n");
        } else {
            output.push_str("Gems on a nearby pedestal:\n");
            for gem in &room.gems {
                output.push_str(&format!("  • {gem}\n"));
            }
        }

        let mut exits: Vec<_> = room
            .neighbors
            .iter()
            .map(|(dir, id)| {
                let name = self
                    .rooms
                    .get(id)
                    .map(|r| r.name.as_str())
                    .unwrap_or("Unknown");
                format!("{} → {}", dir.label(), name)
            })
            .collect();
        exits.sort();
        if exits.is_empty() {
            output.push_str("\nNo obvious exits. Spooky.\n");
        } else {
            output.push_str("\nExits:\n");
            for exit in exits {
                output.push_str(&format!("  {exit}\n"));
            }
        }

        output
    }

    fn render_menu(&self) -> String {
        let actions = self.available_actions();
        let mut lines = vec!["Actions (press number)".to_string()];
        for (idx, action) in actions.iter().enumerate() {
            let prefix = if self.menu_has_focus && idx == self.selected_action {
                ">"
            } else {
                " "
            };
            lines.push(format!("{prefix}{}. {}", idx + 1, action.label));
        }
        lines.join("\n")
    }

    fn render_inventory(&self) -> String {
        let mut lines = vec![format!("Inventory ({} items)", self.inventory.len())];
        if self.inventory.is_empty() {
            lines.push("  • Nothing but lint.".to_string());
        } else {
            for gem in &self.inventory {
                lines.push(format!("  • {gem}"));
            }
        }
        lines.join("\n")
    }

    fn render_status(&self) -> String {
        format!("Status: {}\nPress Esc to leave the dungeon.", self.status)
    }

    fn available_actions(&self) -> Vec<ActionItem> {
        let mut actions = vec![ActionItem::new("Look around", ActionKind::Look)];

        if let Some(room) = self.rooms.get(&self.current_room) {
            if !room.gems.is_empty() {
                actions.push(ActionItem::new("Collect a gem", ActionKind::CollectGem));
            }
        }

        if !self.inventory.is_empty() {
            actions.push(ActionItem::new("Drop a gem", ActionKind::DropGem));
        }

        actions.push(ActionItem::new(
            "Inspect inventory",
            ActionKind::InspectInventory,
        ));
        actions
    }

    fn handle_action(&mut self, index: usize, action: ActionKind) {
        self.selected_action = index;
        match action {
            ActionKind::Look => {
                let description = self
                    .rooms
                    .get(&self.current_room)
                    .map(|room| room.description.clone())
                    .unwrap_or_else(|| "You see nothing of note.".into());
                self.status = description;
            }
            ActionKind::CollectGem => {
                if let Some(room) = self.rooms.get_mut(&self.current_room) {
                    if let Some(gem) = room.gems.pop() {
                        self.status = format!("You pocket the {gem}.");
                        self.inventory.push(gem);
                    } else {
                        self.status = "There are no gems left to collect.".into();
                    }
                }
            }
            ActionKind::DropGem => {
                if let Some(gem) = self.inventory.pop() {
                    if let Some(room) = self.rooms.get_mut(&self.current_room) {
                        room.gems.push(gem.clone());
                    }
                    self.status = format!("You set the {gem} gently on the floor.");
                } else {
                    self.status = "Your pockets are empty.".into();
                }
            }
            ActionKind::InspectInventory => {
                if self.inventory.is_empty() {
                    self.status = "Your inventory is empty.".into();
                } else {
                    let mut msg = String::from("You carry:");
                    for gem in &self.inventory {
                        msg.push_str(&format!(" \u{2022} {gem}"));
                    }
                    self.status = msg;
                }
            }
        }
    }

    fn apply_focus_change(&mut self, ctx: &mut RuntimeContext<'_>, has_focus: bool) {
        if self.menu_has_focus != has_focus {
            self.menu_has_focus = has_focus;
            self.update_all(ctx);
        } else if has_focus {
            ctx.show_cursor();
            ctx.set_cursor_in_zone(MENU_ZONE, (self.selected_action + 1) as i32, 2);
        } else {
            ctx.hide_cursor();
        }
    }

    fn handle_move(&mut self, direction: CardinalDirection) {
        let next = self
            .rooms
            .get(&self.current_room)
            .and_then(|room| room.neighbors.get(&direction))
            .cloned();
        if let Some(next_id) = next {
            if let Some(next_room) = self.rooms.get(&next_id) {
                self.status = format!(
                    "You walk {} into the {}.",
                    direction.label().to_lowercase(),
                    next_room.name
                );
                self.current_room = next_id;
            }
        } else {
            self.status = format!(
                "A crumbling wall blocks your path {}.",
                direction.label().to_lowercase()
            );
        }
    }
}

impl RoomPlugin for MudGamePlugin {
    fn name(&self) -> &str {
        "mud_game"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.update_all(ctx);
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<room_mvp::EventFlow> {
        if let RuntimeEvent::Key(key_event) = event {
            if key_event.kind != KeyEventKind::Press {
                return Ok(room_mvp::EventFlow::Continue);
            }

            if key_event.modifiers.contains(KeyModifiers::CONTROL)
                && key_event.code == KeyCode::Char('q')
            {
                ctx.request_exit();
                return Ok(room_mvp::EventFlow::Consumed);
            }

            match key_event.code {
                KeyCode::Esc => {
                    ctx.request_exit();
                    return Ok(room_mvp::EventFlow::Consumed);
                }
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    let idx = (ch as u8) - b'1';
                    if let Some(action) = self.available_actions().get(idx as usize) {
                        self.handle_action(idx as usize, action.kind);
                    } else {
                        self.status = format!("Action {ch} is not available here.");
                    }
                }
                other => {
                    if let Some(direction) = CardinalDirection::from_key(other) {
                        self.handle_move(direction);
                    }
                }
            }

            self.update_all(ctx);
            return Ok(room_mvp::EventFlow::Consumed);
        }

        Ok(room_mvp::EventFlow::Continue)
    }

    fn on_focus_change(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        change: &FocusChange,
    ) -> Result<()> {
        let to_menu = change
            .to
            .as_ref()
            .map(|target| target.zone == MENU_ZONE)
            .unwrap_or(false);
        let from_menu = change
            .from
            .as_ref()
            .map(|target| target.zone == MENU_ZONE)
            .unwrap_or(false);

        if to_menu {
            self.apply_focus_change(ctx, true);
        } else if from_menu {
            self.apply_focus_change(ctx, false);
        }

        Ok(())
    }
}

struct ActionItem {
    label: String,
    kind: ActionKind,
}

impl ActionItem {
    fn new(label: impl Into<String>, kind: ActionKind) -> Self {
        Self {
            label: label.into(),
            kind,
        }
    }
}

#[derive(Clone, Copy)]
enum ActionKind {
    Look,
    CollectGem,
    DropGem,
    InspectInventory,
}
