use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, ToSocketAddrs};
use std::time::Duration;

use serde::de::DeserializeOwned;
use thiserror::Error;

use crate::{LayoutError, RoomRuntime, RuntimeConfig, RuntimeEvent, Size};

pub type DriverResult<T> = std::result::Result<T, SocketDriverError>;

#[derive(Debug, Error)]
pub enum SocketDriverError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("decode error: {0}")]
    Decode(String),
    #[error("runtime error: {0}")]
    Runtime(#[from] LayoutError),
}

/// Strategy trait used by SocketDriver to decode inbound payloads and emit optional responses.
pub trait SocketStrategy {
    type Inbound: DeserializeOwned;
    type Outbound;

    fn decode(&self, payload: Self::Inbound) -> Result<Vec<RuntimeEvent>, SocketDriverError>;
    fn encode(&self, frame: Self::Outbound) -> Result<String, SocketDriverError>;
    fn after_events(&self, _runtime: &RoomRuntime) -> Option<Self::Outbound> {
        None
    }
}

/// Minimal TCP transport that delegates protocol-specific logic to a SocketStrategy.
pub struct SocketDriver<S: SocketStrategy> {
    listener: TcpListener,
    runtime: RoomRuntime,
    initial_size: Size,
    config: RuntimeConfig,
    strategy: S,
}

impl<S: SocketStrategy> SocketDriver<S> {
    pub fn bind<A>(
        addr: A,
        runtime: RoomRuntime,
        initial_size: Size,
        strategy: S,
    ) -> DriverResult<Self>
    where
        A: ToSocketAddrs,
    {
        let listener = TcpListener::bind(addr)?;
        Ok(Self {
            listener,
            runtime,
            initial_size,
            config: RuntimeConfig::default(),
            strategy,
        })
    }

    pub fn with_config(mut self, config: RuntimeConfig) -> Self {
        self.config = config;
        self
    }

    pub fn run(mut self) -> DriverResult<()> {
        *self.runtime.config_mut() = self.config.clone();
        for stream in self.listener.incoming() {
            let stream = stream?;
            stream.set_nodelay(true).ok();
            self.runtime.resize(self.initial_size)?;

            let inbound = BufReader::new(stream.try_clone()?);
            for line in inbound.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                let payload: S::Inbound = serde_json::from_str(&line)
                    .map_err(|err| SocketDriverError::Decode(err.to_string()))?;
                let events = self.strategy.decode(payload)?;
                let mut writer = stream.try_clone()?;
                self.runtime.run_scripted(&mut writer, events)?;
                if let Some(outbound) = self.strategy.after_events(&self.runtime) {
                    let encoded = self.strategy.encode(outbound)?;
                    if !encoded.is_empty() {
                        writer.write_all(encoded.as_bytes())?;
                        writer.write_all(b"\n")?;
                        writer.flush()?;
                    }
                }
            }
            break;
        }
        Ok(())
    }
}

/// Default stub strategy: simple JSON events (key/resize/tick/paste), no outbound frames.
pub struct JsonEventStrategy;

#[derive(serde::Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum JsonInbound {
    Key {
        code: String,
        modifiers: Vec<String>,
    },
    Resize {
        width: u16,
        height: u16,
    },
    Tick {
        ms: u64,
    },
    Paste {
        data: String,
    },
}

impl SocketStrategy for JsonEventStrategy {
    type Inbound = JsonInbound;
    type Outbound = ();

    fn decode(&self, payload: Self::Inbound) -> Result<Vec<RuntimeEvent>, SocketDriverError> {
        Ok(vec![match payload {
            JsonInbound::Key { code, modifiers } => {
                RuntimeEvent::Key(build_key_event(code, modifiers))
            }
            JsonInbound::Resize { width, height } => RuntimeEvent::Resize(Size::new(width, height)),
            JsonInbound::Tick { ms } => RuntimeEvent::Tick {
                elapsed: Duration::from_millis(ms),
            },
            JsonInbound::Paste { data } => RuntimeEvent::Paste(data),
        }])
    }

    fn encode(&self, _: Self::Outbound) -> Result<String, SocketDriverError> {
        Ok(String::new())
    }
}

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

fn build_key_event(code: String, modifiers: Vec<String>) -> KeyEvent {
    let modifiers = parse_modifiers(&modifiers);
    let key_code = parse_key_code(&code);
    KeyEvent {
        code: key_code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn parse_modifiers(modifiers: &[String]) -> KeyModifiers {
    let mut result = KeyModifiers::empty();
    for m in modifiers {
        match m.to_ascii_lowercase().as_str() {
            "control" | "ctrl" => result |= KeyModifiers::CONTROL,
            "alt" => result |= KeyModifiers::ALT,
            "shift" => result |= KeyModifiers::SHIFT,
            _ => {}
        }
    }
    result
}

fn parse_key_code(code: &str) -> KeyCode {
    match code.to_ascii_lowercase().as_str() {
        "enter" => KeyCode::Enter,
        "backspace" => KeyCode::Backspace,
        "esc" | "escape" => KeyCode::Esc,
        "tab" => KeyCode::Tab,
        other if other.len() == 1 => {
            let ch = other.chars().next().unwrap();
            KeyCode::Char(ch)
        }
        other => KeyCode::Char(other.chars().next().unwrap_or(' ')),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_key_event_defaults_to_press() {
        let event = build_key_event("a".into(), vec!["shift".into()]);
        assert_eq!(event.kind, KeyEventKind::Press);
        assert!(event.modifiers.contains(KeyModifiers::SHIFT));
    }
}
