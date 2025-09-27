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
pub use layout::{Constraint, Direction, GridArea, GridError, GridLayout, GridSize, Layout, LayoutNode, LayoutTree};
pub use logging::{LogEvent, LogFields, LogLevel, Logger, LoggingError, LoggingResult};
pub use metrics::{MetricSnapshot, RuntimeMetrics};
pub use registry::{ZoneContent, ZoneId, ZoneRegistry};
pub use render::{AnsiRenderer, RendererSettings};
pub use runtime::BootstrapControls;
pub use runtime::audit::{
    BootstrapAudit, NullRuntimeAudit, RuntimeAudit, RuntimeAuditEvent, RuntimeAuditEventBuilder,
    RuntimeAuditStage,
};
pub use runtime::bundles::{
    DEFAULT_HINTS_ZONE, DEFAULT_INPUT_ZONE, DEFAULT_STATUS_ZONE, DefaultCliBundleConfig,
    DiagnosticsConfig, DiagnosticsMetricsConfig, InputSharedState, SharedInputState,
    default_cli_bundle, ensure_input_state, try_input_state,
};
pub use runtime::diagnostics::{LifecycleLoggerPlugin, MetricsSnapshotPlugin};
pub use runtime::driver::cli::{CliDriver, CliDriverError, DriverResult};
pub use runtime::driver::socket::{SocketDriver, SocketDriverError};
pub use runtime::focus::{
    FocusController, FocusEntry, FocusRegistry, SharedFocus, ensure_focus_registry,
};
pub use runtime::screens::{
    GlobalZoneStrategy, LegacyScreenStrategy, ScreenActivation, ScreenDefinition, ScreenFactory,
    ScreenLifecycleEvent, ScreenManager, ScreenMetadata, ScreenNavigator, ScreenState,
};
pub use runtime::shared_state::{SharedState, SharedStateError};
pub use runtime::{
    BoxConfig, CollapseMode, EventFlow, PluginBundle, RoomPlugin, RoomRuntime, RuntimeConfig,
    RuntimeContext, RuntimeEvent, SimulatedLoop,
};
pub use tokens::{ZoneTokenRouter, ZoneTokenUpdate};
pub use width::display_width;
pub mod cursor;
