use std::io::{self, Write};

use crossterm::cursor::{Hide, Show};
use crossterm::execute;
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use thiserror::Error;

use crate::{LayoutError, RoomRuntime, Size};

pub type DriverResult<T> = std::result::Result<T, CliDriverError>;

#[derive(Debug, Error)]
pub enum CliDriverError {
    #[error("runtime error: {0}")]
    Runtime(#[from] LayoutError),
    #[error("terminal error: {0}")]
    Terminal(String),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

/// Minimal terminal driver that owns a `RoomRuntime` and manages raw mode +
/// alternate screen transitions. Reusable for any CLI adapter that wants the
/// runtime loop without copying boilerplate.
pub struct CliDriver {
    runtime: RoomRuntime,
}

impl CliDriver {
    pub fn new(runtime: RoomRuntime) -> Self {
        Self { runtime }
    }

    pub fn run(mut self) -> DriverResult<()> {
        let mut stdout = io::stdout();
        self.enter(&mut stdout)?;
        let result = self.run_inner(&mut stdout);
        self.exit(&mut stdout);
        result
    }

    fn run_inner(&mut self, stdout: &mut impl Write) -> DriverResult<()> {
        let (width, height) = terminal::size()?;
        self.runtime.resize(Size::new(width, height))?;
        self.runtime.run(stdout)?;
        Ok(())
    }

    fn enter(&self, stdout: &mut impl Write) -> DriverResult<()> {
        terminal::enable_raw_mode().map_err(|err| CliDriverError::Terminal(err.to_string()))?;
        execute!(stdout, EnterAlternateScreen, Hide, Clear(ClearType::All))?;
        Ok(())
    }

    fn exit(&self, stdout: &mut impl Write) {
        execute!(stdout, Show, LeaveAlternateScreen).ok();
        terminal::disable_raw_mode().ok();
    }
}
