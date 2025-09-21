//! Terminal cursor utility helpers for composing ANSI sequences.
//!
//! These helpers wrap common cursor control sequences so call sites do not need to
//! hand-roll escape codes. All functions return owned `String`s so callers can extend
//! them or write directly to stdout/stderr.

const CSI: &str = "\x1b[";

/// Move the cursor to an absolute 1-based `row` and `column`.
pub fn move_to(row: u16, column: u16) -> String {
    format!("{CSI}{row};{column}H")
}

/// Move the cursor horizontally to the provided 1-based column on the current row.
pub fn move_to_column(column: u16) -> String {
    format!("{CSI}{column}G")
}

/// Move the cursor down `lines`, placing it at column 1 of the destination row.
pub fn move_down_lines(lines: u16) -> String {
    if lines == 0 {
        String::new()
    } else {
        format!("{CSI}{lines}E")
    }
}

/// Move the cursor up `lines`, placing it at column 1 of the destination row.
pub fn move_up_lines(lines: u16) -> String {
    if lines == 0 {
        String::new()
    } else {
        format!("{CSI}{lines}F")
    }
}

/// Move the cursor right by `cols` columns.
pub fn move_right(cols: u16) -> String {
    if cols == 0 {
        String::new()
    } else {
        format!("{CSI}{cols}C")
    }
}

/// Move the cursor left by `cols` columns.
pub fn move_left(cols: u16) -> String {
    if cols == 0 {
        String::new()
    } else {
        format!("{CSI}{cols}D")
    }
}

/// Save the current cursor position.
pub fn save_position() -> &'static str {
    "\x1b[s"
}

/// Restore the most recently saved cursor position.
pub fn restore_position() -> &'static str {
    "\x1b[u"
}

/// Hide the cursor.
pub fn hide() -> &'static str {
    "\x1b[?25l"
}

/// Show the cursor.
pub fn show() -> &'static str {
    "\x1b[?25h"
}

/// Clear from the cursor to the end of the line.
pub fn clear_to_line_end() -> &'static str {
    "\x1b[K"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_position_is_well_formed() {
        assert_eq!(move_to(3, 5), "\x1b[3;5H");
    }

    #[test]
    fn line_navigation_shortcuts() {
        assert_eq!(move_down_lines(0), "");
        assert_eq!(move_down_lines(2), "\x1b[2E");
        assert_eq!(move_up_lines(1), "\x1b[1F");
    }

    #[test]
    fn relative_moves_omit_zero_ops() {
        assert_eq!(move_right(0), "");
        assert_eq!(move_left(3), "\x1b[3D");
    }
}
