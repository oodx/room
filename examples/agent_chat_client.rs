//! Agent Chat Client - Beautiful multi-agent conversation interface
//!
//! Features:
//! - Scrolling chat history with agent message styling
//! - Beautiful Boxy prompt area at bottom
//! - Context panel that opens with slash commands
//! - Status bar with agent info and stats
//! - Agent-aware message rendering

use boxy::{BoxColors, BoxyConfig, WidthConfig, render_to_string};
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, Direction, EventFlow, LayoutNode, LayoutTree,
    LegacyScreenStrategy, Result, RoomPlugin, RoomRuntime, RuntimeConfig, RuntimeContext,
    RuntimeEvent, ScreenDefinition, ScreenManager, Size, SimulatedLoop,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;

// Chat interface zones
const CHAT_HISTORY_ZONE: &str = "chat:history";
const PROMPT_ZONE: &str = "chat:prompt";
const STATUS_ZONE: &str = "chat:status";

// MEGA FACTS DATABASE! 🎉 (From the legendary chat_facts.sh)
const RANDOM_FACTS: &[&str] = &[
    "🌊 The wave emoji looks like a gremlin from far away! 😂",
    "🐙 Octopuses have three hearts and blue blood!",
    "🍯 Honey never spoils - archaeologists found edible honey in Egyptian tombs!",
    "🦋 Butterflies taste with their feet!",
    "🌙 A day on Venus is longer than its year!",
    "🐧 Penguins have knees hidden inside their bodies!",
    "🌈 Bananas are berries, but strawberries aren't!",
    "🧠 Your brain uses 20% of your body's energy despite being 2% of your weight!",
    "🐠 Goldfish can live for over 40 years with proper care!",
    "⚡ Lightning strikes the Earth 8 million times per day!",
    "🌍 Earth is the only known planet where fire can naturally occur!",
    "🦎 Geckos can run up glass surfaces due to van der Waals forces!",
    "📡 Radio waves from Earth have traveled about 100 light-years into space!",
    "🎵 The longest recorded flight of a chicken is 13 seconds!",
    "🌕 The Moon is moving away from Earth at 3.8cm per year!",
    "🐜 Ants can lift 10-50 times their own body weight!",
    "🎨 Shrimp can see 16 types of color receptors (humans only see 3)!",
    "🌋 There are more possible chess games than atoms in the observable universe!",
    "🦆 Ducks have waterproof feathers that never get wet!",
    "🌪️ A cloud can weigh more than a million pounds!",
    "🦘 Kangaroos can't walk backwards!",
    "🐨 Koalas sleep 18-22 hours per day!",
    "🎯 Your stomach gets an entirely new lining every 3-5 days!",
    "🌊 There's more water in the atmosphere than in all rivers combined!",
    "🦖 T-Rex lived closer in time to humans than to Stegosaurus!",
    "🍄 The largest organism on Earth is a fungus in Oregon!",
    "⭐ Neutron stars are so dense that a teaspoon would weigh 6 billion tons!",
    "🐙 Octopuses can change color faster than a chameleon!",
    "🌱 Bamboo can grow up to 3 feet in a single day!",
    "🦅 Peregrine falcons can dive at speeds over 240 mph!",
    "🧬 You share 50% of your DNA with bananas!",
    "🌊 The pressure at the bottom of the ocean would crush you instantly!",
    "🎪 A group of flamingos is called a 'flamboyance'!",
    "🚀 Space is completely silent because there's no air to carry sound!",
    "🦈 Sharks have been around longer than trees!",
    "🌸 Cherry blossoms are actually related to roses!",
    "🧊 Hot water can freeze faster than cold water (Mpemba effect)!",
    "🐧 Emperor penguins can hold their breath for 20+ minutes!",
    "🌈 There are infinite shades of green your eyes can distinguish!",
    "⚡ Your body produces enough heat in 30 minutes to boil water!",
];

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("🤖 Agent Chat Client");
    println!("Beautiful multi-agent conversation interface\n");

    let layout = build_chat_layout();
    let renderer = AnsiRenderer::with_default();
    let mut config = RuntimeConfig::default();
    config.default_focus_zone = Some(PROMPT_ZONE.to_string());
    config.tick_interval = Duration::from_millis(16);

    // Support both interactive and headless testing
    let is_headless = std::env::var("CI").is_ok() || std::env::var("HEADLESS").is_ok();
    if is_headless {
        config.simulated_loop = Some(SimulatedLoop::ticks(8));
    }

    let mut runtime =
        RoomRuntime::with_config(layout.clone(), renderer, Size::new(120, 45), config)?;

    // Set up screen manager
    let mut screen_manager = ScreenManager::new();
    let layout_for_strategy = layout.clone();
    screen_manager.register_screen(ScreenDefinition::new(
        "chat_client".to_string(),
        "Agent Chat Client".to_string(),
        std::sync::Arc::new(move || {
            Box::new(LegacyScreenStrategy::new(layout_for_strategy.clone()))
                as Box<dyn room_mvp::GlobalZoneStrategy>
        }),
    ));

    runtime.set_screen_manager(screen_manager);
    runtime.register_plugin(AgentChatPlugin::new());

    // Handle both modes
    if is_headless {
        let mut buffer = Vec::new();
        runtime.run(&mut buffer)?;
        println!("{}", String::from_utf8_lossy(&buffer));
        Ok(())
    } else {
        CliDriver::new(runtime).run()?;
        Ok(())
    }
}

/// Chat layout - fixed chat area, full-width prompt, popup context
fn build_chat_layout() -> LayoutTree {
    LayoutTree::new(LayoutNode {
        id: "chat_root".into(),
        direction: Direction::Column,
        constraints: vec![
            Constraint::Fixed(30),  // Chat history (fixed height)
            Constraint::Fixed(4),   // Prompt area (full width)
            Constraint::Fixed(2),   // Status bar
        ],
        children: vec![
            // Chat history takes full width - context panel is rendered separately as popup
            LayoutNode::leaf(CHAT_HISTORY_ZONE),
            LayoutNode::leaf(PROMPT_ZONE),
            LayoutNode::leaf(STATUS_ZONE),
        ],
        gap: 0,
        padding: 0,
    })
}

#[derive(Debug, Clone)]
struct ChatMessage {
    sender: String,
    content: String,
    agent_type: AgentType,
    timestamp: String,
}

#[derive(Debug, Clone)]
enum AgentType {
    User,
    Assistant,
    System,
    CodeAgent,
    TestAgent,
    FactBot,
}

impl AgentType {
    fn color(&self) -> &str {
        match self {
            AgentType::User => "cyan",
            AgentType::Assistant => "green",
            AgentType::System => "amber",
            AgentType::CodeAgent => "purple",
            AgentType::TestAgent => "blue",
            AgentType::FactBot => "magenta",
        }
    }

    fn icon(&self) -> &str {
        match self {
            AgentType::User => "👤",
            AgentType::Assistant => "🤖",
            AgentType::System => "⚙️",
            AgentType::CodeAgent => "💻",
            AgentType::TestAgent => "🧪",
            AgentType::FactBot => "🎲",
        }
    }
}

#[derive(Debug)]
struct ChatState {
    messages: Vec<ChatMessage>,
    current_input: String,
    scroll_offset: usize,
    context_visible: bool,
    context_content: String,
    active_agents: Vec<String>,
}

impl ChatState {
    fn new() -> Self {
        let mut state = Self {
            messages: Vec::new(),
            current_input: String::new(),
            scroll_offset: 0,
            context_visible: false,
            context_content: "Context panel hidden - use /context to show".to_string(),
            active_agents: vec!["Assistant".to_string(), "CodeAgent".to_string(), "FactBot".to_string()],
        };

        // Add welcome messages
        state.add_system_message("Welcome to Agent Chat Client! 🚀");
        state.add_system_message("Connected agents: Assistant, CodeAgent, FactBot");
        state.add_message("Assistant", "Hello! I'm your general assistant. Try /help for commands.", AgentType::Assistant);
        state.add_message("CodeAgent", "Hi! I'm specialized in code generation and analysis. Ready to help!", AgentType::CodeAgent);
        state.add_message("FactBot", "🎲 Greetings! I'm your fact dispenser. Try /fact for random knowledge!", AgentType::FactBot);

        state
    }

    fn add_message(&mut self, sender: &str, content: &str, agent_type: AgentType) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() % 86400;
        let hours = (timestamp / 3600) % 24;
        let minutes = (timestamp / 60) % 60;
        let seconds = timestamp % 60;
        let timestamp = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);
        self.messages.push(ChatMessage {
            sender: sender.to_string(),
            content: content.to_string(),
            agent_type,
            timestamp,
        });

        // Auto-scroll to bottom
        self.scroll_to_bottom();
    }

    fn add_system_message(&mut self, content: &str) {
        self.add_message("System", content, AgentType::System);
    }

    fn scroll_to_bottom(&mut self) {
        const VISIBLE_LINES: usize = 25; // Approximate visible lines in chat area
        if self.messages.len() > VISIBLE_LINES {
            self.scroll_offset = self.messages.len() - VISIBLE_LINES;
        } else {
            self.scroll_offset = 0;
        }
    }

    fn handle_input(&mut self) {
        if self.current_input.trim().is_empty() {
            return;
        }

        let input = self.current_input.trim().to_string();

        if input.starts_with('/') {
            self.handle_slash_command(&input);
        } else {
            // Regular user message
            self.add_message("You", &input, AgentType::User);

            // Simulate agent responses
            self.simulate_agent_response(&input);
        }

        self.current_input.clear();
    }

    fn handle_slash_command(&mut self, command: &str) {
        match command {
            "/help" => {
                self.add_system_message("Available commands:");
                self.add_system_message("/help - Show this help");
                self.add_system_message("/fact - Get random amazing fact! 🎲");
                self.add_system_message("/context - Toggle context panel");
                self.add_system_message("/agents - List active agents");
                self.add_system_message("/clear - Clear chat history");
                self.add_system_message("/quit - Exit (Ctrl+Q)");
            }
            "/fact" => {
                // Get a random fact from the mega database!
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                std::time::SystemTime::now().hash(&mut hasher);
                let index = (hasher.finish() as usize) % RANDOM_FACTS.len();
                let fact = RANDOM_FACTS[index];

                self.add_message("FactBot", &format!("🎲 Random Fact Alert! {}", fact), AgentType::FactBot);
            }
            "/context" => {
                self.context_visible = !self.context_visible;
                if self.context_visible {
                    self.context_content = "📋 Context Panel Active\n\nCurrent conversation:\n- Multi-agent chat session\n- 3 active agents (Assistant, CodeAgent, FactBot)\n- Ready for complex tasks\n- Random facts available on demand!".to_string();
                    self.add_system_message("Context panel opened");
                } else {
                    self.context_content = "Context panel hidden - use /context to show".to_string();
                    self.add_system_message("Context panel closed");
                }
            }
            "/agents" => {
                self.add_system_message(&format!("Active agents: {}", self.active_agents.join(", ")));
            }
            "/clear" => {
                self.messages.clear();
                self.scroll_offset = 0;
                self.add_system_message("Chat history cleared");
            }
            cmd if cmd.starts_with("/code") => {
                let task = cmd.strip_prefix("/code").unwrap_or("").trim();
                if task.is_empty() {
                    self.add_message("CodeAgent", "What would you like me to code? Try: /code hello world", AgentType::CodeAgent);
                } else {
                    self.add_message("CodeAgent", &format!("Analyzing request: {}", task), AgentType::CodeAgent);
                    self.add_message("CodeAgent", "```rust\nfn main() {\n    println!(\"Hello, {}!\");\n}\n```", AgentType::CodeAgent);
                }
            }
            _ => {
                self.add_system_message(&format!("Unknown command: {} (try /help)", command));
            }
        }
    }

    fn simulate_agent_response(&mut self, user_input: &str) {
        // Simple response simulation based on input
        if user_input.to_lowercase().contains("fact") || user_input.to_lowercase().contains("random") {
            std::thread::sleep(Duration::from_millis(400));
            // FactBot loves to share facts!
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            user_input.hash(&mut hasher);
            let index = (hasher.finish() as usize) % RANDOM_FACTS.len();
            let fact = RANDOM_FACTS[index];
            self.add_message("FactBot", &format!("🎲 Did you know? {}", fact), AgentType::FactBot);
        } else if user_input.to_lowercase().contains("code") || user_input.to_lowercase().contains("program") {
            std::thread::sleep(Duration::from_millis(500));
            self.add_message("CodeAgent", "I can help with that! What kind of code are you looking for?", AgentType::CodeAgent);
        } else if user_input.to_lowercase().contains("test") {
            std::thread::sleep(Duration::from_millis(500));
            self.add_message("TestAgent", "I'm here for testing needs! What would you like to test?", AgentType::TestAgent);
        } else {
            std::thread::sleep(Duration::from_millis(300));
            let responses = [
                "That's interesting! Tell me more.",
                "I understand. How can I help with that?",
                "Great question! Let me think about that.",
                "I see what you mean. Would you like me to elaborate?",
                "Try /fact for something amazing! 🎲",
            ];
            let response = &responses[user_input.len() % responses.len()];
            self.add_message("Assistant", response, AgentType::Assistant);
        }
    }
}

struct AgentChatPlugin {
    state: Arc<Mutex<ChatState>>,
}

impl AgentChatPlugin {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ChatState::new())),
        }
    }

    fn update_all_zones(&self, ctx: &mut RuntimeContext) {
        if let Ok(state) = self.state.lock() {
            // Update chat history (includes popup context overlay when visible)
            ctx.set_zone_pre_rendered(CHAT_HISTORY_ZONE, self.render_chat_with_overlay(&state));

            // Update prompt
            ctx.set_zone_pre_rendered(PROMPT_ZONE, self.render_prompt(&state));

            // Update status
            ctx.set_zone_pre_rendered(STATUS_ZONE, self.render_status(&state));
        }
    }

    fn render_chat_with_overlay(&self, state: &ChatState) -> String {
        // Context is now handled inline within the chat history
        self.render_chat_history(state)
    }

    fn render_chat_history(&self, state: &ChatState) -> String {
        const VISIBLE_LINES: usize = 25;
        const CHAT_WIDTH: usize = 110; // Fixed width
        let start_idx = state.scroll_offset;
        let end_idx = (start_idx + VISIBLE_LINES).min(state.messages.len());

        let mut chat_lines = Vec::new();

        for msg in state.messages[start_idx..end_idx].iter() {
            let formatted_msg = format!(
                "[{}] {} {}: {}",
                msg.timestamp,
                msg.agent_type.icon(),
                msg.sender,
                msg.content
            );
            chat_lines.push(formatted_msg);
        }

        // If context is visible, reserve space for popup by reducing chat lines
        let available_lines = if state.context_visible {
            VISIBLE_LINES - 8 // Reserve 8 lines for context popup
        } else {
            VISIBLE_LINES
        };

        // Truncate chat lines to available space
        chat_lines.truncate(available_lines);

        // Add context popup inline if visible
        if state.context_visible {
            chat_lines.push("".to_string()); // Separator
            chat_lines.push("                                       ┌─────────────────────────────────┐".to_string());
            chat_lines.push("                                       │ 📋 Context Panel                │".to_string());
            chat_lines.push("                                       ├─────────────────────────────────┤".to_string());
            let context_line = state.context_content.lines().next().unwrap_or("Context info");
            let truncated_context = if context_line.len() > 31 {
                format!("{}...", &context_line[..28])
            } else {
                context_line.to_string()
            };
            chat_lines.push(format!("                                       │ {:<31} │", truncated_context));
            chat_lines.push("                                       │ Use /context to toggle          │".to_string());
            chat_lines.push("                                       │ Positioned top-right ✨         │".to_string());
            chat_lines.push("                                       └─────────────────────────────────┘".to_string());
        }

        // Pad with empty lines if needed
        while chat_lines.len() < VISIBLE_LINES {
            chat_lines.push("".to_string());
        }

        let config = BoxyConfig {
            text: chat_lines.join("\n"),
            title: Some("🤖 Agent Conversation".to_string()),
            colors: BoxColors {
                box_color: "blue".to_string(),
                text_color: "white".to_string(),
                title_color: Some("cyan".to_string()),
                header_color: Some("bright_blue".to_string()),
                footer_color: None,
                status_color: None,
            },
            width: WidthConfig {
                fixed_width: Some(CHAT_WIDTH),
                enable_wrapping: true,
                ..WidthConfig::default()
            },
            fixed_height: Some(27), // Fixed height
            ..Default::default()
        };

        render_to_string(&config)
    }


    fn render_prompt(&self, state: &ChatState) -> String {
        let prompt_text = if state.current_input.is_empty() {
            "Type a message or /help for commands...".to_string()
        } else {
            state.current_input.clone()
        };

        let config = BoxyConfig {
            text: prompt_text,
            title: Some("💬 Your Message".to_string()),
            colors: BoxColors {
                box_color: "green".to_string(),
                text_color: "white".to_string(),
                title_color: Some("bright_green".to_string()),
                header_color: Some("green".to_string()),
                footer_color: None,
                status_color: None,
            },
            width: WidthConfig {
                fixed_width: Some(118), // Nearly full terminal width (120 - 2 for padding)
                enable_wrapping: false,
                ..WidthConfig::default()
            },
            fixed_height: Some(2),
            ..Default::default()
        };

        render_to_string(&config)
    }

    fn render_status(&self, state: &ChatState) -> String {
        format!(
            "🤖 {} agents • 💬 {} messages • {} | Ctrl+Q: quit | /help: commands",
            state.active_agents.len(),
            state.messages.len(),
            if state.context_visible { "📋 context" } else { "📋 hidden" }
        )
    }
}

impl RoomPlugin for AgentChatPlugin {
    fn name(&self) -> &str {
        "agent_chat"
    }

    fn init(&mut self, _ctx: &mut RuntimeContext) -> Result<()> {
        Ok(())
    }

    fn on_user_ready(&mut self, ctx: &mut RuntimeContext) -> Result<()> {
        self.update_all_zones(ctx);
        Ok(())
    }

    fn on_event(&mut self, ctx: &mut RuntimeContext, event: &RuntimeEvent) -> Result<EventFlow> {
        if let RuntimeEvent::Key(key_event) = event {
            if key_event.kind != KeyEventKind::Press {
                return Ok(EventFlow::Continue);
            }

            // Handle Ctrl+Q to quit
            if key_event.modifiers.contains(KeyModifiers::CONTROL)
                && key_event.code == KeyCode::Char('q')
            {
                ctx.request_exit();
                return Ok(EventFlow::Consumed);
            }

            if let Ok(mut state) = self.state.lock() {
                match key_event.code {
                    KeyCode::Enter => {
                        state.handle_input();
                    }
                    KeyCode::Backspace => {
                        state.current_input.pop();
                    }
                    KeyCode::Char(ch) => {
                        state.current_input.push(ch);
                    }
                    _ => return Ok(EventFlow::Continue),
                }

                drop(state);
                self.update_all_zones(ctx);
                ctx.request_render();
                return Ok(EventFlow::Consumed);
            }
        }

        Ok(EventFlow::Continue)
    }
}