//! Boxy Dynamic Resize Demo - CORRECTED VERSION
//!
//! Demonstrates proper use of Boxy API with Room's resize events.
//! Shows a full-screen box that dynamically adapts to terminal size changes.
//!
//! Key corrections from initial version:
//! - Uses RuntimeEvent::Resize(size) directly (no shell commands)
//! - Correctly passes TOTAL box height to with_fixed_height() (not just body height)
//! - Proper accounting for layout padding and box chrome
//!
//! ```bash
//! cargo run --example boxy_dynamic_resize
//! ```
//!
//! Try resizing your terminal window to see real-time updates!

use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, Direction, LayoutNode, LayoutTree, RoomPlugin,
    RoomRuntime, RuntimeConfig, RuntimeContext, Size, EventFlow, RuntimeEvent,
};
use room_mvp::runtime::audit::{BootstrapAudit, NullRuntimeAudit};
use std::sync::Arc;
use boxy::api::layout::{BoxBuilder, HeaderBuilder, FooterBuilder};
use boxy::visual::ROUNDED;
use crossterm::event::{KeyCode, KeyModifiers};
use crossterm::terminal;

const DISPLAY_ZONE: &str = "app:display";
const LAYOUT_PADDING: u16 = 2;  // From build_layout() padding

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let layout = build_layout();
    let renderer = AnsiRenderer::with_default();
    let mut config = RuntimeConfig::default();

    // Bootstrap configuration for clean first paint
    config.audit = Some(BootstrapAudit::new(Arc::new(NullRuntimeAudit)));

    // Get actual terminal size
    let (term_width, term_height) = terminal::size().unwrap_or((80, 24));

    let mut runtime = RoomRuntime::with_config(
        layout,
        renderer,
        Size::new(term_width, term_height),
        config,
    )?;

    runtime.register_plugin(DynamicResizePlugin::new(term_width, term_height));

    CliDriver::new(runtime).run()?;
    Ok(())
}

fn build_layout() -> LayoutTree {
    LayoutTree::new(LayoutNode {
        id: "root".into(),
        direction: Direction::Column,
        constraints: vec![Constraint::Flex(1)],
        children: vec![LayoutNode::leaf(DISPLAY_ZONE)],
        gap: 0,
        padding: LAYOUT_PADDING,  // 2 on each side = 4 total width/height
    })
}

struct DynamicResizePlugin {
    term_width: u16,
    term_height: u16,
    resize_count: usize,
}

impl DynamicResizePlugin {
    fn new(width: u16, height: u16) -> Self {
        Self {
            term_width: width,
            term_height: height,
            resize_count: 0,
        }
    }

    fn render_display(&self) -> String {
        // CRITICAL: Calculate total box dimensions correctly
        //
        // Layout consumes: LAYOUT_PADDING * 2 = 4 (2 left + 2 right, 2 top + 2 bottom)
        // Box structure:
        //   - Header: 1 line
        //   - Body: variable (content + padding)
        //   - Footer: 1 line
        //   - Borders: 2 lines (top + bottom)
        // Total chrome: header(1) + footer(1) + borders(2) = 4 lines
        //
        // with_fixed_height() expects TOTAL box height (including all chrome)
        // Room layout padding: 4 (2*LAYOUT_PADDING)
        // Therefore: total_box_height = terminal_height - layout_padding

        let total_box_width = (self.term_width as usize)
            .saturating_sub(LAYOUT_PADDING as usize * 2)  // Left + right padding
            .max(20);

        let total_box_height = (self.term_height as usize)
            .saturating_sub(LAYOUT_PADDING as usize * 2)  // Top + bottom padding
            .max(8);

        // Boxy will internally allocate:
        // - Header: 1 line
        // - Footer: 1 line
        // - Top border: part of header
        // - Bottom border: part of footer
        // - Body: total_box_height - header(1) - footer(1) = total - 2
        let body_height = total_box_height.saturating_sub(2);

        let content = format!(
            "Terminal Dimensions\n\
             ══════════════════════════════\n\n\
             Terminal Size:\n\
             • Width:  {} columns\n\
             • Height: {} rows\n\
             • Total:  {} cells ({}×{})\n\n\
             Layout Calculations:\n\
             • Layout Padding: {} (×2 = {} per axis)\n\
             • Available Space: {}×{}\n\n\
             Box Dimensions (passed to Boxy):\n\
             • Total Box Width:  {} (includes borders)\n\
             • Total Box Height: {} (includes header+footer+borders)\n\
             • Body Height:      {} (total - header - footer)\n\n\
             Resize Events: {}\n\n\
             ════════════════════════════════════════════════════════\n\
             The box fills the entire terminal!\n\
             Try resizing to see it adapt in real-time.\n\
             ════════════════════════════════════════════════════════",
            self.term_width,
            self.term_height,
            self.term_width as usize * self.term_height as usize,
            self.term_width,
            self.term_height,
            LAYOUT_PADDING,
            LAYOUT_PADDING * 2,
            total_box_width,
            total_box_height,
            total_box_width,
            total_box_height,
            body_height,
            self.resize_count,
        );

        // Count content lines for debugging
        let content_line_count = content.lines().count();

        // CORRECT: Pass total_box_height (includes header + footer + borders)
        // Boxy handles the internal split between header/body/footer
        let layout = BoxBuilder::new(&content)
            .with_header(HeaderBuilder::new("🔄 Dynamic Resize Demo - CORRECTED").align_center())
            .with_footer(FooterBuilder::new(&format!("Press [Q] to exit │ Content: {} lines │ Target: {} lines", content_line_count, body_height)).align_center())
            .with_style(ROUNDED)
            .with_fixed_width(total_box_width)
            .with_fixed_height(total_box_height)  // TOTAL height, not body height!
            .with_wrapping(false)  // Disable wrapping to prevent line count inflation
            .build();

        let rendered = layout.render();
        let rendered_lines = rendered.lines().count();

        // Debug: Print to stderr so we can see what's happening
        eprintln!("DEBUG: term_height={}, total_box_height={}, content_lines={}, rendered_lines={}",
                  self.term_height, total_box_height, content_line_count, rendered_lines);

        rendered
    }

    fn update_display(&self, ctx: &mut RuntimeContext) {
        ctx.set_zone_pre_rendered(DISPLAY_ZONE, self.render_display());
    }
}

impl RoomPlugin for DynamicResizePlugin {
    fn name(&self) -> &str {
        "dynamic_resize_corrected"
    }

    fn init(&mut self, ctx: &mut RuntimeContext) -> room_mvp::Result<()> {
        self.update_display(ctx);
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext,
        event: &RuntimeEvent,
    ) -> room_mvp::Result<EventFlow> {
        match event {
            RuntimeEvent::Key(key_event) => {
                match key_event.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                        ctx.request_exit();
                        Ok(EventFlow::Consumed)
                    }
                    KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        ctx.request_exit();
                        Ok(EventFlow::Consumed)
                    }
                    _ => Ok(EventFlow::Continue),
                }
            }
            RuntimeEvent::Resize(new_size) => {
                // CORRECT: Use the size provided by the runtime directly
                // No shell commands, no platform dependencies, no latency
                self.term_width = new_size.width;
                self.term_height = new_size.height;
                self.resize_count += 1;

                // Re-render with new dimensions
                self.update_display(ctx);

                // Request render (though set_zone_pre_rendered should trigger it)
                ctx.request_render();

                Ok(EventFlow::Continue)
            }
            _ => Ok(EventFlow::Continue),
        }
    }
}