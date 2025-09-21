use std::sync::{Arc, RwLock};

use super::RuntimeContext;
use super::shared_state::SharedStateError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusEntry {
    pub owner: String,
    pub zone_id: String,
}

#[derive(Default)]
pub struct FocusRegistry {
    inner: RwLock<Option<FocusEntry>>,
}

impl FocusRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_focus(&self, owner: impl Into<String>, zone_id: impl Into<String>) {
        let entry = FocusEntry {
            owner: owner.into(),
            zone_id: zone_id.into(),
        };
        if let Ok(mut guard) = self.inner.write() {
            *guard = Some(entry);
        }
    }

    pub fn clear_focus(&self, owner: &str) {
        if let Ok(mut guard) = self.inner.write() {
            if guard.as_ref().map(|e| e.owner.as_str()) == Some(owner) {
                *guard = None;
            }
        }
    }

    pub fn current(&self) -> Option<FocusEntry> {
        self.inner.read().ok().and_then(|guard| guard.clone())
    }
}

pub struct FocusController {
    owner: String,
    registry: SharedFocus,
    last_zone: Option<String>,
}

impl FocusController {
    pub fn new(owner: impl Into<String>, registry: SharedFocus) -> Self {
        Self {
            owner: owner.into(),
            registry,
            last_zone: None,
        }
    }

    pub fn focus(&mut self, zone_id: impl Into<String>) {
        let zone = zone_id.into();
        self.registry.set_focus(&self.owner, zone.clone());
        self.last_zone = Some(zone);
    }

    pub fn release(&self) {
        self.registry.clear_focus(&self.owner);
    }

    pub fn current(&self) -> Option<FocusEntry> {
        self.registry.current()
    }

    pub fn last_zone(&self) -> Option<&str> {
        self.last_zone.as_deref()
    }
}

pub type SharedFocus = Arc<FocusRegistry>;

pub fn ensure_focus_registry(ctx: &RuntimeContext<'_>) -> Result<SharedFocus, SharedStateError> {
    ctx.shared_init::<FocusRegistry, _>(FocusRegistry::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get_focus() {
        let registry = FocusRegistry::new();
        registry.set_focus("plugin", "zone");
        let entry = registry.current().unwrap();
        assert_eq!(entry.owner, "plugin");
        assert_eq!(entry.zone_id, "zone");
    }

    #[test]
    fn clear_focus_by_owner() {
        let registry = FocusRegistry::new();
        registry.set_focus("plugin", "zone");
        registry.clear_focus("plugin");
        assert!(registry.current().is_none());
    }

    #[test]
    fn clear_other_owner_noop() {
        let registry = FocusRegistry::new();
        registry.set_focus("plugin", "zone");
        registry.clear_focus("other");
        assert!(registry.current().is_some());
    }
}
