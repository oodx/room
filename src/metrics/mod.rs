use crate::logging::{LogEvent, LogFields, LogLevel};
use serde_json::json;
use std::time::Duration;

#[derive(Debug, Default, Clone)]
pub struct RuntimeMetrics {
    events: u64,
    renders: u64,
    dirty_zones: u64,
    zone_updates: u64,
}

impl RuntimeMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_event(&mut self) {
        self.events = self.events.saturating_add(1);
    }

    pub fn record_render(&mut self, dirty_count: usize) {
        self.renders = self.renders.saturating_add(1);
        self.dirty_zones = self.dirty_zones.saturating_add(dirty_count as u64);
    }

    pub fn record_zone_updates(&mut self, count: usize) {
        if count > 0 {
            self.zone_updates = self.zone_updates.saturating_add(count as u64);
        }
    }

    pub fn snapshot(&self, uptime: Duration) -> MetricSnapshot {
        MetricSnapshot {
            uptime_ms: uptime.as_millis() as u64,
            events: self.events,
            renders: self.renders,
            dirty_zones: self.dirty_zones,
            zone_updates: self.zone_updates,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MetricSnapshot {
    pub uptime_ms: u64,
    pub events: u64,
    pub renders: u64,
    pub dirty_zones: u64,
    pub zone_updates: u64,
}

impl MetricSnapshot {
    pub fn to_log_event(&self, target: &str) -> LogEvent {
        let mut fields = LogFields::new();
        fields.insert("uptime_ms".to_string(), json!(self.uptime_ms));
        fields.insert("events".to_string(), json!(self.events));
        fields.insert("renders".to_string(), json!(self.renders));
        fields.insert("dirty_zones".to_string(), json!(self.dirty_zones));
        fields.insert("zone_updates".to_string(), json!(self.zone_updates));
        LogEvent::with_fields(
            LogLevel::Info,
            target.to_string(),
            "runtime_metrics".to_string(),
            fields,
        )
    }

    pub fn as_fields(&self) -> LogFields {
        let mut map = LogFields::new();
        map.insert("uptime_ms".to_string(), json!(self.uptime_ms));
        map.insert("events".to_string(), json!(self.events));
        map.insert("renders".to_string(), json!(self.renders));
        map.insert("dirty_zones".to_string(), json!(self.dirty_zones));
        map.insert("zone_updates".to_string(), json!(self.zone_updates));
        map
    }
}

pub fn snapshot_event(snapshot: &MetricSnapshot, target: &str) -> LogEvent {
    snapshot.to_log_event(target)
}
