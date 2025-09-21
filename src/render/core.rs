use std::io::Write;

use crate::display_width;
use crate::error::Result;
use crate::geometry::Rect;
use crate::registry::{ZoneId, ZoneState};

/// Renderer runtime parameters.
#[derive(Debug, Clone)]
pub struct RendererSettings {
    pub restore_cursor: Option<(u16, u16)>,
}

impl Default for RendererSettings {
    fn default() -> Self {
        Self {
            restore_cursor: None,
        }
    }
}

/// ANSI escape code renderer writing directly to a terminal handle.
pub struct AnsiRenderer {
    settings: RendererSettings,
}

impl AnsiRenderer {
    pub fn new(settings: RendererSettings) -> Self {
        Self { settings }
    }

    pub fn with_default() -> Self {
        Self::new(RendererSettings::default())
    }

    pub fn settings_mut(&mut self) -> &mut RendererSettings {
        &mut self.settings
    }

    pub fn render(&mut self, writer: &mut impl Write, dirty: &[(ZoneId, ZoneState)]) -> Result<()> {
        for (_id, state) in dirty {
            render_zone(writer, state)?;
        }

        if let Some((row, col)) = self.settings.restore_cursor {
            write!(writer, "\x1b[{};{}H", row + 1, col + 1)?;
        }

        writer.flush()?;
        Ok(())
    }
}

fn render_zone(writer: &mut impl Write, state: &ZoneState) -> Result<()> {
    let Rect {
        x,
        y,
        width,
        height,
    } = state.rect;

    if width == 0 || height == 0 {
        return Ok(());
    }

    let mut rendered_lines = if state.is_pre_rendered {
        state
            .content
            .lines()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
    } else {
        wrap_to_width(&state.content, width)
    };

    if rendered_lines.len() > height as usize {
        rendered_lines.truncate(height as usize);
    }

    while rendered_lines.len() < height as usize {
        rendered_lines.push(String::new());
    }

    for (offset, line) in rendered_lines.iter_mut().enumerate() {
        pad_line(line, width);
        write!(writer, "\x1b[{};{}H", y + offset as u16 + 1, x + 1)?;
        write!(writer, "{}", line)?;
    }

    Ok(())
}

fn wrap_to_width(content: &str, width: u16) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }

    let mut lines = Vec::new();
    for raw in content.split('\n') {
        if raw.is_empty() {
            lines.push(String::new());
            continue;
        }

        let mut current = String::new();
        for ch in raw.chars() {
            if current.is_empty() && ch == ' ' {
                continue;
            }
            current.push(ch);
            let display = display_width(&current) as u16;
            if display > width {
                current.pop();
                if !current.is_empty() {
                    let mut flushed = current.clone();
                    if flushed.starts_with(' ') {
                        flushed = flushed.trim_start().to_string();
                    }
                    lines.push(flushed);
                } else {
                    // Character wider than available width, skip it.
                    lines.push(String::new());
                }
                current.clear();
                current.push(ch);
            } else if display == width {
                let mut flushed = current.clone();
                if flushed.starts_with(' ') {
                    flushed = flushed.trim_start().to_string();
                }
                lines.push(flushed);
                current.clear();
            }
        }

        if !current.is_empty() {
            let mut flushed = current;
            if flushed.starts_with(' ') {
                flushed = flushed.trim_start().to_string();
            }
            lines.push(flushed);
        }
    }

    lines
}

fn pad_line(line: &mut String, width: u16) {
    let mut display = display_width(line) as u16;
    while display < width {
        line.push(' ');
        display += 1;
    }

    if display > width {
        // Truncate any overshoot caused by ANSI codes being stripped differently.
        while (display_width(line) as u16) > width {
            line.pop();
        }
        while (display_width(line) as u16) < width {
            line.push(' ');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ZoneRegistry;
    use std::collections::HashMap;

    #[test]
    fn wrap_basic() {
        let lines = wrap_to_width("hello world", 5);
        assert_eq!(lines, vec!["hello".to_string(), "world".to_string()]);
    }

    #[test]
    fn renderer_writes_cursor_sequences() {
        let mut registry = ZoneRegistry::new();
        let mut solved = HashMap::new();
        solved.insert("zone".to_string(), Rect::new(2, 3, 5, 2));
        registry.sync_layout(&solved);
        registry.take_dirty();
        registry
            .apply_content(&"zone".to_string(), "hi".to_string())
            .unwrap();
        let dirty = registry.take_dirty();

        let mut output = Vec::new();
        let mut renderer = AnsiRenderer::with_default();
        renderer.render(&mut output, &dirty).unwrap();

        let rendered = String::from_utf8(output).unwrap();
        assert!(rendered.contains("\u{1b}[4;3Hhi"));
        assert!(rendered.contains("\u{1b}[5;3H"));
    }
}
