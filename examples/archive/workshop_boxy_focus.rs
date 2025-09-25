//! Room Workshop: Boxy Focus & Cursor Signals
//!
//! Demonstrates focus transitions and cursor updates using a compact Boxy-rendered
//! adventure board. Move between rooms with the arrow keys, toggle focus between the
//! map and actions with Tab/Shift+Tab, and watch the focus/cursor log update in real time.
//!
//! ```bash
//! cargo run --example workshop_boxy_focus
//! ```

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use boxy::{BoxColors, BoxyConfig, WidthConfig, render_to_string};
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use room_mvp::runtime::audit::{BootstrapAudit, NullRuntimeAudit};
use room_mvp::runtime::focus::{FocusController, ensure_focus_registry};
use room_mvp::runtime::{CursorEvent, FocusChange};
use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, Direction, LayoutNode, LayoutTree, Result, RoomPlugin,
    RoomRuntime, RuntimeConfig, RuntimeContext, RuntimeEvent, Size,
};

const INSTRUCTIONS_ZONE: &str = "workshop:boxy_focus.instructions";
const MAP_ZONE: &str = "workshop:boxy_focus.map";
const DETAIL_ZONE: &str = "workshop:boxy_focus.detail";
const ACTION_ZONE: &str = "workshop:boxy_focus.actions";
const LOG_ZONE: &str = "workshop:boxy_focus.log";
const MAX_LOG_LINES: usize = 12;

fn main() -> Result<()> {
    let layout = build_layout();
    let renderer = AnsiRenderer::with_default();

    let mut config = RuntimeConfig::default();
    config.audit = Some(BootstrapAudit::new(Arc::new(NullRuntimeAudit)));
    config.default_focus_zone = Some(MAP_ZONE.to_string());
    config.tick_interval = Duration::from_millis(200);

    let mut runtime = RoomRuntime::with_config(layout, renderer, Size::new(96, 30), config)?;
    runtime.register_plugin(BoxyFocusWorkshop::new());

    CliDriver::new(runtime)
        .run()
        .map_err(|err| room_mvp::LayoutError::Backend(err.to_string()))
}

fn build_layout() -> LayoutTree {
    LayoutTree::new(LayoutNode {
        id: "workshop:boxy_focus.root".into(),
        direction: Direction::Column,
        constraints: vec![
            Constraint::Fixed(5),  // instructions
            Constraint::Fixed(11), // map
            Constraint::Fixed(6),  // detail panel
            Constraint::Fixed(5),  // actions panel
            Constraint::Fixed(6),  // log output
        ],
        children: vec![
            LayoutNode::leaf(INSTRUCTIONS_ZONE),
            LayoutNode::leaf(MAP_ZONE),
            LayoutNode::leaf(DETAIL_ZONE),
            LayoutNode::leaf(ACTION_ZONE),
            LayoutNode::leaf(LOG_ZONE),
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

    fn label(self) -> &'static str {
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
        "Glass shards scatter rainbows across the marble floor.",
        (0, 0),
        &[
            (DirectionKey::East, "gallery"),
            (DirectionKey::South, "scriptorium"),
        ],
        &["🌤️", "✨"],
    ),
    RoomSeed::new(
        "gallery",
        "Crystal Gallery",
        "Portraits watch with gemstone eyes as glass sculptures hum.",
        (1, 0),
        &[
            (DirectionKey::West, "atrium"),
            (DirectionKey::South, "vault"),
        ],
        &["🖼️", "💎"],
    ),
    RoomSeed::new(
        "scriptorium",
        "Silent Scriptorium",
        "Quills float over parchment, sketching forgotten glyphs.",
        (0, -1),
        &[
            (DirectionKey::North, "atrium"),
            (DirectionKey::East, "vault"),
        ],
        &["🪶", "📜"],
    ),
    RoomSeed::new(
        "vault",
        "Luminous Vault",
        "An iris of light guards a pedestal of shimmering relics.",
        (1, -1),
        &[
            (DirectionKey::North, "gallery"),
            (DirectionKey::West, "scriptorium"),
        ],
        &["🔐", "💠"],
    ),
];

#[derive(Clone)]
struct Room {
    name: String,
    description: String,
    neighbors: HashMap<DirectionKey, String>,
    emoji: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FocusPane {
    Map,
    Actions,
}

impl FocusPane {
    fn zone(self) -> &'static str {
        match self {
            FocusPane::Map => MAP_ZONE,
            FocusPane::Actions => ACTION_ZONE,
        }
    }
}

struct BoxyFocusWorkshop {
    rooms: HashMap<String, Room>,
    positions: HashMap<(i32, i32), String>,
    current_room: String,
    focus: FocusPane,
    controller: Option<FocusController>,
    log: VecDeque<String>,
    status: String,
    selected_action: usize,
}

impl BoxyFocusWorkshop {
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
            current_room: "atrium".into(),
            focus: FocusPane::Map,
            controller: None,
            log: VecDeque::new(),
            status: "Use arrows to explore. Tab switches focus.".into(),
            selected_action: 0,
        }
    }

    fn ensure_controller(&mut self, ctx: &RuntimeContext<'_>) -> Result<&mut FocusController> {
        if self.controller.is_none() {
            let registry = ensure_focus_registry(ctx)
                .map_err(|err| room_mvp::LayoutError::Backend(format!("focus registry: {err}")))?;
            self.controller = Some(FocusController::new("boxy-focus", registry));
        }
        Ok(self
            .controller
            .as_mut()
            .expect("focus controller initialized"))
    }

    fn render(&mut self, ctx: &mut RuntimeContext<'_>) {
        ctx.set_zone(INSTRUCTIONS_ZONE, instructions());
        ctx.set_zone_pre_rendered(MAP_ZONE, self.render_map());
        ctx.set_zone(DETAIL_ZONE, self.render_detail());
        ctx.set_zone_pre_rendered(ACTION_ZONE, self.render_actions());
        self.flush_log(ctx);
        self.update_cursor(ctx);
    }

    fn render_map(&self) -> String {
        let (min_x, max_x, min_y, max_y) = self.positions.keys().fold(
            (i32::MAX, i32::MIN, i32::MAX, i32::MIN),
            |(min_x, max_x, min_y, max_y), &(x, y)| {
                (min_x.min(x), max_x.max(x), min_y.min(y), max_y.max(y))
            },
        );

        let mut rows: Vec<Vec<String>> = Vec::new();
        for y in (min_y..=max_y).rev() {
            let mut panels: Vec<Vec<String>> = Vec::new();
            for x in min_x..=max_x {
                let tile = if let Some(id) = self.positions.get(&(x, y)) {
                    let focused = id == &self.current_room;
                    let room = self.rooms.get(id).expect("room exists");
                    render_room_tile(room, focused)
                } else {
                    render_empty_tile()
                };
                panels.push(tile);
            }
            rows.push(stitch_row(panels));
        }

        rows.into_iter()
            .flat_map(|chunk| chunk.into_iter().chain(std::iter::once(String::new())))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn render_detail(&self) -> String {
        let room = self.rooms.get(&self.current_room).expect("room exists");
        let mut text = format!("Room: {}\n\n{}\n", room.name, room.description);
        if room.emoji.is_empty() {
            text.push_str("\nNothing sparkling remains.\n");
        } else {
            text.push_str("\nYou notice: ");
            text.push_str(&room.emoji.join("  "));
            text.push('\n');
        }
        text.push_str("\nExits:\n");
        for (dir, target) in &room.neighbors {
            if let Some(dest) = self.rooms.get(target) {
                text.push_str(&format!("  {} → {}\n", dir.label(), dest.name));
            }
        }
        text
    }

    fn render_actions(&self) -> String {
        let actions = self.available_actions();
        let mut lines = Vec::new();
        lines.push(match self.focus {
            FocusPane::Actions => "▶ Actions (focus)".into(),
            FocusPane::Map => "Actions".into(),
        });
        lines.push("---------".into());
        for (idx, action) in actions.iter().enumerate() {
            let marker = if self.focus == FocusPane::Actions && idx == self.selected_action {
                "▶"
            } else {
                " "
            };
            lines.push(format!("{marker} {}", self.action_label(*action)));
        }
        lines.push("Tab/Shift+Tab switches focus · Enter activates".into());

        let height = lines.len().max(5);
        let config = BoxyConfig {
            text: lines.join("\n"),
            title: None,
            colors: BoxColors {
                box_color: "teal".into(),
                text_color: "auto".into(),
                title_color: Some("white".into()),
                header_color: None,
                footer_color: None,
                status_color: None,
            },
            width: WidthConfig {
                fixed_width: Some(70),
                enable_wrapping: true,
                ..WidthConfig::default()
            },
            fixed_height: Some(height),
            ..Default::default()
        };

        render_to_string(&config)
    }

    fn log_event(&mut self, message: impl Into<String>) {
        if self.log.len() == MAX_LOG_LINES {
            self.log.pop_front();
        }
        self.log.push_back(message.into());
    }

    fn flush_log(&self, ctx: &mut RuntimeContext<'_>) {
        let mut output = String::from("Focus & Cursor Log\n====================\n");
        for line in &self.log {
            output.push_str(line);
            output.push('\n');
        }
        ctx.set_zone(LOG_ZONE, output);
    }

    fn available_actions(&self) -> Vec<ActionKind> {
        let mut actions = vec![ActionKind::Inspect];
        if let Some(room) = self.rooms.get(&self.current_room) {
            if !room.emoji.is_empty() {
                actions.push(ActionKind::Collect);
            }
        }
        actions.push(ActionKind::Rest);
        actions
    }

    fn action_label(&self, action: ActionKind) -> &'static str {
        match action {
            ActionKind::Inspect => "Inspect room",
            ActionKind::Collect => "Collect shimmering relic",
            ActionKind::Rest => "Take a quiet breath",
        }
    }

    fn perform_action(&mut self, action: ActionKind) {
        match action {
            ActionKind::Inspect => {
                let detail = self
                    .rooms
                    .get(&self.current_room)
                    .map(|room| room.description.clone())
                    .unwrap_or_else(|| "Nothing remarkable right now.".into());
                self.status = detail;
                self.log_event("Action: Inspect current room");
            }
            ActionKind::Collect => {
                if let Some(room) = self.rooms.get_mut(&self.current_room) {
                    if let Some(item) = room.emoji.pop() {
                        self.status = format!("You collect {}", item);
                        self.log_event(format!("Collected {item}"));
                    } else {
                        self.status = "Nothing left to collect.".into();
                        self.log_event("Collection attempt failed (empty)");
                    }
                }
            }
            ActionKind::Rest => {
                self.status = "You steady your breath and observe.".into();
                self.log_event("Took a quiet moment");
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
                self.status = format!("You travel {} into the {}.", direction.label(), target.name);
                self.log_event(format!("Moved {}", direction.label()));
            }
            self.current_room = id;
        } else {
            self.status = format!("A barrier blocks your path {}.", direction.label());
            self.log_event(format!("Blocked moving {}", direction.label()));
        }
    }

    fn set_focus(&mut self, ctx: &mut RuntimeContext<'_>, pane: FocusPane) -> Result<()> {
        if self.focus == pane {
            return Ok(());
        }
        let controller = self.ensure_controller(ctx)?;
        controller.focus(pane.zone());
        Ok(())
    }

    fn cycle_action(&mut self, delta: i32) {
        let actions = self.available_actions();
        if actions.is_empty() {
            return;
        }
        let len = actions.len() as i32;
        let next = (self.selected_action as i32 + delta).rem_euclid(len);
        self.selected_action = next as usize;
    }

    fn update_cursor(&self, ctx: &mut RuntimeContext<'_>) {
        match self.focus {
            FocusPane::Map => {
                if let Some(pos) = self.positions.iter().find_map(|(pos, id)| {
                    if id == &self.current_room {
                        Some(*pos)
                    } else {
                        None
                    }
                }) {
                    let (min_x, _, _min_y, max_y) = self.positions.keys().fold(
                        (i32::MAX, i32::MIN, i32::MAX, i32::MIN),
                        |(min_x, max_x, min_y, max_y), &(x, y)| {
                            (min_x.min(x), max_x.max(x), min_y.min(y), max_y.max(y))
                        },
                    );
                    const TILE_WIDTH: usize = 23; // 22 chars + spacer
                    const TILE_HEIGHT: usize = 6; // 5 lines + spacer
                    const COL_OFFSET: usize = 11;
                    const ROW_OFFSET: usize = 3;
                    let col = (pos.0 - min_x) as usize * TILE_WIDTH + COL_OFFSET;
                    let row = (max_y - pos.1) as usize * TILE_HEIGHT + ROW_OFFSET;
                    ctx.show_cursor();
                    ctx.set_cursor_in_zone(MAP_ZONE, row as i32, col as i32);
                } else {
                    ctx.hide_cursor();
                }
            }
            FocusPane::Actions => {
                ctx.show_cursor();
                ctx.set_cursor_in_zone(ACTION_ZONE, (self.selected_action + 2) as i32, 2);
            }
        }
    }
}

impl RoomPlugin for BoxyFocusWorkshop {
    fn name(&self) -> &str {
        "workshop_boxy_focus"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.ensure_controller(ctx)?.focus(MAP_ZONE);
        self.focus = FocusPane::Map;
        self.render(ctx);
        self.log_event("Focus initialized on map");
        self.flush_log(ctx);
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<room_mvp::EventFlow> {
        if let RuntimeEvent::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return Ok(room_mvp::EventFlow::Continue);
            }

            match (self.focus, key.code) {
                (_, KeyCode::Esc) => {
                    ctx.request_exit();
                    return Ok(room_mvp::EventFlow::Consumed);
                }
                (_, KeyCode::Char('c')) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    ctx.request_exit();
                    return Ok(room_mvp::EventFlow::Consumed);
                }
                (_, KeyCode::Tab) => {
                    let next = match self.focus {
                        FocusPane::Map => FocusPane::Actions,
                        FocusPane::Actions => FocusPane::Map,
                    };
                    self.set_focus(ctx, next)?;
                    return Ok(room_mvp::EventFlow::Consumed);
                }
                (FocusPane::Map, code) => {
                    if let Some(direction) = DirectionKey::from_key(code) {
                        self.move_to(direction);
                        self.render(ctx);
                        return Ok(room_mvp::EventFlow::Consumed);
                    }
                }
                (FocusPane::Actions, KeyCode::Up) => {
                    self.cycle_action(-1);
                    self.render(ctx);
                    return Ok(room_mvp::EventFlow::Consumed);
                }
                (FocusPane::Actions, KeyCode::Down) => {
                    self.cycle_action(1);
                    self.render(ctx);
                    return Ok(room_mvp::EventFlow::Consumed);
                }
                (FocusPane::Actions, KeyCode::Enter | KeyCode::Char(' ')) => {
                    if let Some(action) =
                        self.available_actions().get(self.selected_action).cloned()
                    {
                        self.perform_action(action);
                        self.render(ctx);
                        return Ok(room_mvp::EventFlow::Consumed);
                    }
                }
                _ => {}
            }
        }
        Ok(room_mvp::EventFlow::Continue)
    }

    fn on_focus_change(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        change: &FocusChange,
    ) -> Result<()> {
        if let Some(target) = change.to.as_ref() {
            if target.zone == MAP_ZONE {
                self.focus = FocusPane::Map;
                self.log_event("Focus → map");
            } else if target.zone == ACTION_ZONE {
                self.focus = FocusPane::Actions;
                self.log_event("Focus → actions");
            } else {
                self.log_event(format!("Focus → {}", target.zone));
            }
        } else {
            self.log_event("Focus cleared");
        }

        if let Some(from) = change.from.as_ref() {
            self.log_event(format!("Focus left {}", from.zone));
        }

        self.render(ctx);
        Ok(())
    }

    fn on_cursor_event(&mut self, ctx: &mut RuntimeContext<'_>, event: &CursorEvent) -> Result<()> {
        match event {
            CursorEvent::Moved(cursor) => self.log_event(format!(
                "Cursor moved to ({}, {})",
                cursor.position.0, cursor.position.1
            )),
            CursorEvent::Shown(_) => self.log_event("Cursor shown"),
            CursorEvent::Hidden(_) => self.log_event("Cursor hidden"),
        }
        self.flush_log(ctx);
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum ActionKind {
    Inspect,
    Collect,
    Rest,
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

fn instructions() -> &'static str {
    "Boxy Focus & Cursor Workshop\n\
     --------------------------------\n\
     • Arrow keys move between rooms when the map has focus.\n\
     • Tab / Shift+Tab switch focus between the map and the actions list.\n\
     • Enter (or Space) triggers the highlighted action.\n\
     • Watch the log for FocusChanged and cursor events.\n\
     • Esc or Ctrl+C exits."
}
