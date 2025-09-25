//! Room Example: Boxy MUD Mini Game
//!
//! Explore a compact dungeon rendered with Boxy panels. The map appears across the
//! top row, each room drawn as a colored tile with emoji treasures. The middle
//! section describes the current room and inventory, while the bottom navigation
//! bar shows contextual actions.
//!
//! ```bash
//! cargo run --example mud_boxy_game
//! ```

use std::collections::HashMap;
use std::time::Duration;

use boxy::{BoxColors, BoxyConfig, WidthConfig, render_to_string};
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use room_mvp::runtime::FocusChange;
use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, Direction, LayoutNode, LayoutTree, LegacyScreenStrategy,
    Result, RoomPlugin, RoomRuntime, RuntimeConfig, RuntimeContext, RuntimeEvent, ScreenDefinition,
    ScreenManager, Size, SimulatedLoop,
};

const MAP_ZONE: &str = "mud_boxy:map";
const DETAIL_ZONE: &str = "mud_boxy:detail";
const INVENTORY_ZONE: &str = "mud_boxy:inventory";
const NAV_ZONE: &str = "mud_boxy:navigation";
const STATUS_ZONE: &str = "mud_boxy:status";
const TICK_INTERVAL: Duration = Duration::from_millis(250);

fn main() -> Result<()> {
    let layout = build_layout();
    let renderer = AnsiRenderer::with_default();
    let mut config = RuntimeConfig::default();
    // Start in room exploration mode - no default focus on navigation
    config.tick_interval = TICK_INTERVAL;

    // Support both interactive and headless testing
    let is_headless = std::env::var("CI").is_ok() || std::env::var("HEADLESS").is_ok();
    if is_headless {
        config.simulated_loop = Some(SimulatedLoop::ticks(5));
    }

    let mut runtime =
        RoomRuntime::with_config(layout.clone(), renderer, Size::new(100, 40), config)?;

    let mut screen_manager = ScreenManager::new();
    screen_manager.register_screen(ScreenDefinition::new(
        "mud-boxy",
        "Boxy MUD Mini Game",
        std::sync::Arc::new(move || Box::new(LegacyScreenStrategy::new(layout.clone()))),
    ));
    runtime.set_screen_manager(screen_manager);
    runtime.activate_screen("mud-boxy")?;

    runtime.register_plugin(BoxyMudPlugin::new());

    // Handle both interactive and headless execution
    if is_headless {
        let mut buffer = Vec::new();
        runtime.run(&mut buffer)?;
        println!("{}", String::from_utf8_lossy(&buffer));
        Ok(())
    } else {
        CliDriver::new(runtime)
            .run()
            .map_err(|err| room_mvp::LayoutError::Backend(err.to_string()))
    }
}

fn build_layout() -> LayoutTree {
    LayoutTree::new(LayoutNode {
        id: "mud_boxy:root".into(),
        direction: Direction::Column,
        constraints: vec![
            Constraint::Fixed(17),   // MAP_ZONE (increased for 3 rows of tiles)
            Constraint::Flex(1),     // middle section (detail + inventory)
            Constraint::Fixed(3),    // STATUS_ZONE
            Constraint::Fixed(8),    // NAV_ZONE (increased for action buttons)
        ],
        children: vec![
            LayoutNode::leaf(MAP_ZONE),
            LayoutNode {
                id: "mud_boxy:middle".into(),
                direction: Direction::Row,
                constraints: vec![Constraint::Flex(1), Constraint::Fixed(32)],
                children: vec![
                    LayoutNode::leaf(DETAIL_ZONE),
                    LayoutNode::leaf(INVENTORY_ZONE),
                ],
                gap: 1,
                padding: 0,
            },
            LayoutNode::leaf(STATUS_ZONE),
            LayoutNode::leaf(NAV_ZONE),
        ],
        gap: 1,
        padding: 1,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum DirectionKey {
    North,
    South,
    East,
    West,
}

impl DirectionKey {
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

struct RoomSeed {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    position: (i32, i32),
    neighbors: &'static [(DirectionKey, &'static str)],
    emoji: &'static [&'static str],
}

impl RoomSeed {
    const fn new(
        id: &'static str,
        name: &'static str,
        description: &'static str,
        position: (i32, i32),
        neighbors: &'static [(DirectionKey, &'static str)],
        emoji: &'static [&'static str],
    ) -> Self {
        Self {
            id,
            name,
            description,
            position,
            neighbors,
            emoji,
        }
    }
}

const ROOM_SEEDS: &[RoomSeed] = &[
    RoomSeed::new(
        "atrium",
        "Sunlit Atrium",
        "Sunlight streams through fractured glass, scattering prisms across the marble floor.",
        (0, 0),
        &[
            (DirectionKey::North, "observatory"),
            (DirectionKey::East, "gallery"),
            (DirectionKey::South, "scriptorium"),
        ],
        &["🌤️", "✨"],
    ),
    RoomSeed::new(
        "observatory",
        "Celestial Observatory",
        "Bronze telescopes aim toward painted constellations that shimmer unnaturally.",
        (0, 1),
        &[
            (DirectionKey::South, "atrium"),
            (DirectionKey::East, "workshop"),
        ],
        &["🔭", "🌌"],
    ),
    RoomSeed::new(
        "gallery",
        "Crystal Gallery",
        "Portraits watch with gemstone eyes while glass sculptures hum with resonant tones.",
        (1, 0),
        &[
            (DirectionKey::West, "atrium"),
            (DirectionKey::East, "garden"),
            (DirectionKey::South, "vault"),
        ],
        &["🖼️", "💎"],
    ),
    RoomSeed::new(
        "garden",
        "Echoing Garden",
        "Iridescent plants bloom in slow motion around reflecting pools that ripple backwards.",
        (2, 0),
        &[
            (DirectionKey::West, "gallery"),
            (DirectionKey::South, "vault"),
        ],
        &["🌺", "🪞"],
    ),
    RoomSeed::new(
        "scriptorium",
        "Silent Scriptorium",
        "Ink quills float over parchment, writing forgotten alphabets as candles never burn down.",
        (0, -1),
        &[
            (DirectionKey::North, "atrium"),
            (DirectionKey::East, "vault"),
        ],
        &["📜", "🪄"],
    ),
    RoomSeed::new(
        "vault",
        "Prismatic Vault",
        "Segmented vault doors lie ajar, releasing a soft hum that vibrates through your bones.",
        (1, -1),
        &[
            (DirectionKey::North, "gallery"),
            (DirectionKey::West, "scriptorium"),
            (DirectionKey::East, "garden"),
        ],
        &["🔐", "🌈"],
    ),
    RoomSeed::new(
        "workshop",
        "Arcane Workshop",
        "Tools orbit workbenches, forged from alloys that seem to phase in and out of reality.",
        (1, 1),
        &[
            (DirectionKey::West, "observatory"),
            (DirectionKey::South, "gallery"),
        ],
        &["⚙️", "🧪"],
    ),
];

struct Room {
    name: String,
    description: String,
    neighbors: HashMap<DirectionKey, String>,
    emoji: Vec<String>,
}

struct BoxyMudPlugin {
    rooms: HashMap<String, Room>,
    positions: HashMap<(i32, i32), String>,
    current_room: String,
    inventory: Vec<String>,
    status: String,
    action_hint: String,
    selected_action: usize,
    nav_has_focus: bool,
}

impl BoxyMudPlugin {
    fn new() -> Self {
        let mut rooms = HashMap::new();
        let mut positions = HashMap::new();
        for seed in ROOM_SEEDS {
            positions.insert(seed.position, seed.id.to_string());
            rooms.insert(
                seed.id.to_string(),
                Room {
                    name: seed.name.to_string(),
                    description: seed.description.to_string(),
                    neighbors: seed
                        .neighbors
                        .iter()
                        .map(|(dir, target)| (*dir, (*target).to_string()))
                        .collect(),
                    emoji: seed.emoji.iter().map(|e| (*e).to_string()).collect(),
                },
            );
        }

        Self {
            rooms,
            positions,
            current_room: "atrium".to_string(),
            inventory: Vec::new(),
            status: String::from("You arrive in the Sunlit Atrium."),
            action_hint: String::from("Use Tab to focus navigation, arrows to move between rooms"),
            selected_action: 0,
            nav_has_focus: false,  // Start in room exploration mode
        }
    }

    fn update_all(&mut self, ctx: &mut RuntimeContext<'_>) {
        self.normalize_selection();
        ctx.set_zone_pre_rendered(MAP_ZONE, self.render_map());
        ctx.set_zone(DETAIL_ZONE, self.render_detail());
        ctx.set_zone(STATUS_ZONE, self.render_status());
        ctx.set_zone(INVENTORY_ZONE, self.render_inventory());
        ctx.set_zone_pre_rendered(NAV_ZONE, self.render_navigation());

        if self.nav_has_focus {
            ctx.show_cursor();
            // cursor row: Boxy panel includes header line, then actions start at row 2.
            ctx.set_cursor_in_zone(NAV_ZONE, (self.selected_action + 1) as i32, 3);
        } else {
            ctx.hide_cursor();
        }
    }

    fn normalize_selection(&mut self) {
        let actions = self.available_actions();
        if actions.is_empty() {
            self.selected_action = 0;
        } else if self.selected_action >= actions.len() {
            self.selected_action = actions.len() - 1;
        }
    }

    fn render_map(&self) -> String {
        let (min_x, max_x, min_y, max_y) = self
            .positions
            .keys()
            .fold((i32::MAX, i32::MIN, i32::MAX, i32::MIN), |acc, &(x, y)| {
                (acc.0.min(x), acc.1.max(x), acc.2.min(y), acc.3.max(y))
            });

        let mut rows: Vec<Vec<String>> = Vec::new();
        for y in (min_y..=max_y).rev() {
            let mut row_panels: Vec<Vec<String>> = Vec::new();
            for x in min_x..=max_x {
                let lines = if let Some(id) = self.positions.get(&(x, y)) {
                    let is_current = id == &self.current_room;
                    let room = self.rooms.get(id).expect("room exists");
                    render_room_tile(room, is_current)
                } else {
                    render_empty_tile()
                };
                row_panels.push(lines);
            }
            rows.push(stitch_row(row_panels));
        }

        rows.into_iter()
            .flat_map(|chunk| chunk.into_iter().chain(std::iter::once(String::new())))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn render_detail(&self) -> String {
        let room = self
            .rooms
            .get(&self.current_room)
            .expect("current room exists");
        let mut text = format!("Room: {}\n\n{}\n", room.name, room.description);

        if room.emoji.is_empty() {
            text.push_str("\nNothing here sparkles today.\n");
        } else {
            text.push_str("\nYou notice: ");
            text.push_str(&room.emoji.join("  "));
            text.push('\n');
        }

        let mut exits: Vec<_> = room
            .neighbors
            .iter()
            .map(|(dir, id)| {
                let label = self
                    .rooms
                    .get(id)
                    .map(|r| r.name.as_str())
                    .unwrap_or("Unknown");
                format!("{} → {}", dir.label(), label)
            })
            .collect();
        exits.sort();
        text.push_str("\nExits:\n");
        if exits.is_empty() {
            text.push_str("  None\n");
        } else {
            for exit in exits {
                text.push_str(&format!("  {exit}\n"));
            }
        }

        text
    }

    fn render_inventory(&self) -> String {
        let mut lines = vec![format!("Inventory ({} items)", self.inventory.len())];
        if self.inventory.is_empty() {
            lines.push("  • Pockets empty".to_string());
        } else {
            for item in &self.inventory {
                lines.push(format!("  • {item}"));
            }
        }
        lines.join("\n")
    }

    fn render_status(&self) -> String {
        format!("Status: {}", self.status)
    }

    fn render_navigation(&self) -> String {
        let actions = self.available_actions();
        let mut lines = vec![self.action_hint.clone()];
        for (idx, action) in actions.iter().enumerate() {
            let marker = if self.nav_has_focus && idx == self.selected_action {
                "▶"
            } else {
                " "
            };
            lines.push(format!(
                "{marker}{}: {}",
                idx + 1,
                self.action_label(*action)
            ));
        }
        let instruction = if self.nav_has_focus {
            "Tab: switch to map · Arrows: select actions · Esc: quit"
        } else {
            "Tab: switch to navigation · Arrows: move between rooms · Esc: quit"
        };
        lines.push(instruction.to_string());

        let config = BoxyConfig {
            text: lines.join("\n"),
            title: Some("Navigation".to_string()),
            colors: BoxColors {
                box_color: "teal".to_string(),
                text_color: "auto".to_string(),
                title_color: Some("white".to_string()),
                header_color: None,
                footer_color: None,
                status_color: None,
            },
            width: WidthConfig {
                fixed_width: None,
                enable_wrapping: true,
                ..WidthConfig::default()
            },
            fixed_height: None, // Let Boxy calculate the height automatically
            ..Default::default()
        };
        render_to_string(&config)
    }

    fn available_actions(&self) -> Vec<ActionKind> {
        let mut actions = vec![ActionKind::Inspect];
        if let Some(room) = self.rooms.get(&self.current_room) {
            if !room.emoji.is_empty() {
                actions.push(ActionKind::Collect);
            }
        }
        if !self.inventory.is_empty() {
            actions.push(ActionKind::Drop);
        }
        actions.push(ActionKind::Inventory);
        actions
    }

    fn action_label(&self, action: ActionKind) -> &'static str {
        match action {
            ActionKind::Inspect => "Inspect",
            ActionKind::Collect => "Collect",
            ActionKind::Drop => "Drop",
            ActionKind::Inventory => "Inventory",
        }
    }

    fn perform_action(&mut self, index: usize, action: ActionKind) {
        self.selected_action = index;
        match action {
            ActionKind::Inspect => {
                let detail = self
                    .rooms
                    .get(&self.current_room)
                    .map(|room| room.description.clone())
                    .unwrap_or_else(|| "Nothing catches your eye.".into());
                self.status = detail;
            }
            ActionKind::Collect => {
                if let Some(room) = self.rooms.get_mut(&self.current_room) {
                    if let Some(item) = room.emoji.pop() {
                        self.inventory.push(item.clone());
                        self.status = format!("You collect {item}.");
                    } else {
                        self.status = "There is nothing left to take.".into();
                    }
                }
            }
            ActionKind::Drop => {
                if let Some(item) = self.inventory.pop() {
                    if let Some(room) = self.rooms.get_mut(&self.current_room) {
                        room.emoji.push(item.clone());
                    }
                    self.status = format!("You gently place {item} back.");
                } else {
                    self.status = "Your hands are empty.".into();
                }
            }
            ActionKind::Inventory => {
                if self.inventory.is_empty() {
                    self.status = "Your inventory is empty.".into();
                } else {
                    self.status = format!("You carry: {}", self.inventory.join(", "));
                }
            }
        }
    }

    fn move_to(&mut self, direction: DirectionKey) {
        let next = self
            .rooms
            .get(&self.current_room)
            .and_then(|room| room.neighbors.get(&direction))
            .cloned();
        if let Some(id) = next {
            if let Some(target) = self.rooms.get(&id) {
                self.status = format!(
                    "You step {} into the {}.",
                    direction.label().to_lowercase(),
                    target.name
                );
            }
            self.current_room = id;
        } else {
            self.status = format!(
                "A barrier blocks your path {}.",
                direction.label().to_lowercase()
            );
        }
    }

    fn cycle_action(&mut self, ctx: &mut RuntimeContext<'_>, direction: i32) {
        let len = self.available_actions().len();
        if len == 0 {
            return;
        }
        let current = self.selected_action as i32;
        let next = (current + direction).rem_euclid(len as i32);
        self.selected_action = next as usize;
        self.update_all(ctx);
    }

    fn apply_focus_change(&mut self, ctx: &mut RuntimeContext<'_>, has_focus: bool) {
        if self.nav_has_focus != has_focus {
            self.nav_has_focus = has_focus;
            self.update_all(ctx);
        } else if has_focus {
            ctx.show_cursor();
            ctx.set_cursor_in_zone(NAV_ZONE, (self.selected_action + 1) as i32, 3);
        } else {
            ctx.hide_cursor();
        }
    }
}

impl RoomPlugin for BoxyMudPlugin {
    fn name(&self) -> &str {
        "mud_boxy"
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
                && matches!(key_event.code, KeyCode::Char('q') | KeyCode::Char('c'))
            {
                ctx.request_exit();
                return Ok(room_mvp::EventFlow::Consumed);
            }

            match key_event.code {
                KeyCode::Tab => {
                    // Toggle focus between navigation and map/room exploration
                    if self.nav_has_focus {
                        self.status = "[Focus] Room exploration mode".into();
                        self.apply_focus_change(ctx, false);
                    } else {
                        self.status = "[Focus] Navigation actions ready".into();
                        self.apply_focus_change(ctx, true);
                    }
                    self.update_all(ctx);
                    return Ok(room_mvp::EventFlow::Consumed);
                }
                KeyCode::Esc => {
                    ctx.request_exit();
                    return Ok(room_mvp::EventFlow::Consumed);
                }
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    let idx = (ch as u8) - b'1';
                    if let Some(action) = self.available_actions().get(idx as usize) {
                        self.perform_action(idx as usize, *action);
                        self.update_all(ctx);
                        return Ok(room_mvp::EventFlow::Consumed);
                    } else {
                        self.status = format!("Action {ch} is unavailable.");
                    }
                }
                KeyCode::Left | KeyCode::Up => {
                    if self.nav_has_focus {
                        // Navigation focused: arrow keys cycle through actions
                        self.cycle_action(ctx, -1);
                        return Ok(room_mvp::EventFlow::Consumed);
                    } else {
                        // Map focused: arrow keys move between rooms
                        if let Some(direction) =
                            DirectionKey::from_key(if matches!(key_event.code, KeyCode::Left) {
                                KeyCode::Left
                            } else {
                                KeyCode::Up
                            })
                        {
                            self.move_to(direction);
                            self.update_all(ctx);
                            return Ok(room_mvp::EventFlow::Consumed);
                        }
                    }
                }
                KeyCode::Right | KeyCode::Down => {
                    if self.nav_has_focus {
                        // Navigation focused: arrow keys cycle through actions
                        self.cycle_action(ctx, 1);
                        return Ok(room_mvp::EventFlow::Consumed);
                    } else {
                        // Map focused: arrow keys move between rooms
                        if let Some(direction) =
                            DirectionKey::from_key(if matches!(key_event.code, KeyCode::Right) {
                                KeyCode::Right
                            } else {
                                KeyCode::Down
                            })
                        {
                            self.move_to(direction);
                            self.update_all(ctx);
                            return Ok(room_mvp::EventFlow::Consumed);
                        }
                    }
                }
                other => {
                    if let Some(direction) = DirectionKey::from_key(other) {
                        self.move_to(direction);
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
        let to_nav = change
            .to
            .as_ref()
            .map(|target| target.zone == NAV_ZONE)
            .unwrap_or(false);
        let from_nav = change
            .from
            .as_ref()
            .map(|target| target.zone == NAV_ZONE)
            .unwrap_or(false);

        if to_nav {
            self.status = "[Focus] Navigation actions ready".into();
            self.apply_focus_change(ctx, true);
        } else if from_nav {
            self.status = "[Focus] Navigation released".into();
            self.apply_focus_change(ctx, false);
        }

        self.update_all(ctx);

        Ok(())
    }
}

#[derive(Clone, Copy)]
enum ActionKind {
    Inspect,
    Collect,
    Drop,
    Inventory,
}

fn render_room_tile(room: &Room, focused: bool) -> Vec<String> {
    let colors = BoxColors {
        box_color: if focused {
            "gold".into()
        } else {
            "slate".into()
        },
        text_color: "auto".into(),
        title_color: Some("white".into()),
        header_color: if focused {
            Some("yellow".into())
        } else {
            Some("white".into())
        },
        footer_color: None,
        status_color: None,
    };

    let config = BoxyConfig {
        text: if room.emoji.is_empty() {
            "(no treasures)".into()
        } else {
            room.emoji.join("  ")
        },
        title: Some(room.name.clone()),
        colors,
        width: WidthConfig {
            fixed_width: Some(22),
            enable_wrapping: true,
            ..WidthConfig::default()
        },
        fixed_height: Some(5),
        ..Default::default()
    };
    render_to_string(&config)
        .lines()
        .map(|line| line.to_string())
        .collect()
}

fn render_empty_tile() -> Vec<String> {
    let config = BoxyConfig {
        text: "".into(),
        title: Some("---".into()),
        colors: BoxColors {
            box_color: "charcoal".into(),
            text_color: "auto".into(),
            title_color: Some("white".into()),
            header_color: Some("gray".into()),
            footer_color: None,
            status_color: None,
        },
        width: WidthConfig {
            fixed_width: Some(22),
            enable_wrapping: true,
            ..WidthConfig::default()
        },
        fixed_height: Some(5),
        ..Default::default()
    };
    render_to_string(&config)
        .lines()
        .map(|line| line.to_string())
        .collect()
}

fn stitch_row(panels: Vec<Vec<String>>) -> Vec<String> {
    if panels.is_empty() {
        return vec![];
    }
    let height = panels[0].len();
    let mut combined = vec![String::new(); height];
    for panel in panels {
        for (idx, line) in panel.into_iter().enumerate() {
            if let Some(target) = combined.get_mut(idx) {
                if !target.is_empty() {
                    target.push(' ');
                }
                target.push_str(&line);
            }
        }
    }
    combined
}
