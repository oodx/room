//! Experimental pilot implementation of the Room layout engine MVP.
//!
//! This crate lives under `concepts/room/pilot` while the API solidifies.
//! The modules follow the RSB `MODULE_SPEC` pattern so we can eventually
//! promote the code into a production crate without major surgery.

pub mod error;
pub mod geometry;
pub mod layout;
pub mod logging;
pub mod metrics;
pub mod registry;
pub mod render;
pub mod runtime;
pub mod tokens;
pub mod width;
pub mod zone;

pub use error::{LayoutError, Result};
pub use geometry::{Rect, Size};
pub use layout::{Constraint, Direction, LayoutNode, LayoutTree};
pub use logging::{LogEvent, LogFields, LogLevel, Logger, LoggingError, LoggingResult};
pub use metrics::{MetricSnapshot, RuntimeMetrics};
pub use registry::{ZoneContent, ZoneId, ZoneRegistry};
pub use render::{AnsiRenderer, RendererSettings};
pub use runtime::diagnostics::{LifecycleLoggerPlugin, MetricsSnapshotPlugin};
pub use runtime::{
    EventFlow, RoomPlugin, RoomRuntime, RuntimeConfig, RuntimeContext, RuntimeEvent,
};
pub use tokens::{ZoneTokenRouter, ZoneTokenUpdate};
pub use width::display_width;
