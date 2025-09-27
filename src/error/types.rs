use thiserror::Error;

use crate::layout::GridError;

/// Unified result type for the Room MVP crate.
pub type Result<T> = std::result::Result<T, LayoutError>;

/// Errors surfaced by the layout engine MVP.
#[derive(Debug, Error)]
pub enum LayoutError {
    #[error("layout tree is empty")]
    EmptyLayout,
    #[error("zone `{0}` not found")]
    ZoneNotFound(String),
    #[error("token routing failure: {0}")]
    TokenRouting(String),
    #[error("terminal backend error: {0}")]
    Backend(String),
    #[error("grid layout error: {0}")]
    Grid(#[from] GridError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
