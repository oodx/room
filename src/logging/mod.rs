use serde::Serialize;
use serde_json::{Map, Value, json};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use thiserror::Error;

pub type LogFields = Map<String, Value>;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogEvent {
    pub ts_ms: u128,
    pub level: LogLevel,
    pub target: String,
    pub message: String,
    #[serde(skip_serializing_if = "LogFields::is_empty", default)]
    pub fields: LogFields,
}

impl LogEvent {
    pub fn new(level: LogLevel, target: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            ts_ms: current_ms(),
            level,
            target: target.into(),
            message: message.into(),
            fields: LogFields::new(),
        }
    }

    pub fn with_fields(
        level: LogLevel,
        target: impl Into<String>,
        message: impl Into<String>,
        fields: LogFields,
    ) -> Self {
        Self {
            fields,
            ..Self::new(level, target, message)
        }
    }
}

fn current_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

pub type LoggingResult<T> = std::result::Result<T, LoggingError>;

#[derive(Debug, Error)]
pub enum LoggingError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub trait LogSink: Send + Sync {
    fn log(&self, event: &LogEvent) -> LoggingResult<()>;
}

#[derive(Clone)]
pub struct Logger {
    sink: Arc<dyn LogSink>,
}

impl Logger {
    pub fn new<S>(sink: S) -> Self
    where
        S: LogSink + 'static,
    {
        Self {
            sink: Arc::new(sink),
        }
    }

    pub fn log(&self, level: LogLevel, target: &str, message: &str) -> LoggingResult<()> {
        let event = LogEvent::new(level, target.to_string(), message.to_string());
        self.sink.log(&event)
    }

    pub fn log_with_fields(
        &self,
        level: LogLevel,
        target: &str,
        message: &str,
        fields: LogFields,
    ) -> LoggingResult<()> {
        let event = LogEvent::with_fields(level, target.to_string(), message.to_string(), fields);
        self.sink.log(&event)
    }

    pub fn log_event(&self, event: LogEvent) -> LoggingResult<()> {
        self.sink.log(&event)
    }
}

pub struct FileSink {
    path: PathBuf,
    max_bytes: u64,
    writer: Mutex<BufWriter<File>>,
}

impl FileSink {
    pub fn new(path: impl AsRef<Path>, max_bytes: u64) -> LoggingResult<Self> {
        let path = path.as_ref().to_path_buf();
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&path)?;
        Ok(Self {
            path,
            max_bytes,
            writer: Mutex::new(BufWriter::new(file)),
        })
    }

    fn write_line(&self, mut line: String) -> LoggingResult<()> {
        line.push('\n');
        let mut guard = self.writer.lock().expect("logger mutex poisoned");

        if self.should_rotate(guard.get_ref(), line.len() as u64)? {
            let file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&self.path)?;
            *guard = BufWriter::new(file);
        }

        guard.write_all(line.as_bytes())?;
        guard.flush()?;
        Ok(())
    }

    fn should_rotate(&self, file: &File, incoming_len: u64) -> std::io::Result<bool> {
        if self.max_bytes == 0 {
            return Ok(false);
        }
        let current = file.metadata()?.len();
        Ok(current + incoming_len > self.max_bytes)
    }
}

impl LogSink for FileSink {
    fn log(&self, event: &LogEvent) -> LoggingResult<()> {
        let line = serde_json::to_string(event)?;
        self.write_line(line)
    }
}

pub fn field_map() -> LogFields {
    LogFields::new()
}

pub fn field_pair(key: &str, value: Value) -> (String, Value) {
    (key.to_string(), value)
}

pub fn message_event(level: LogLevel, target: &str, message: &str) -> LogEvent {
    LogEvent::new(level, target.to_string(), message.to_string())
}

pub fn event_with_fields(
    level: LogLevel,
    target: &str,
    message: &str,
    fields: impl IntoIterator<Item = (String, Value)>,
) -> LogEvent {
    let mut map = LogFields::new();
    for (k, v) in fields.into_iter() {
        map.insert(k, v);
    }
    LogEvent::with_fields(level, target.to_string(), message.to_string(), map)
}

pub fn json_kv(key: &str, value: impl Into<Value>) -> (String, Value) {
    (key.to_string(), value.into())
}

pub fn json_str(key: &str, value: impl Into<String>) -> (String, Value) {
    (key.to_string(), json!(value.into()))
}
