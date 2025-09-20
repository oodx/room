use std::collections::{HashMap, HashSet};

use blake3::Hash;

use crate::error::{LayoutError, Result};
use crate::geometry::Rect;

pub type ZoneId = String;

/// User facing payload stored for each zone.
pub type ZoneContent = String;

#[derive(Debug, Clone)]
pub struct ZoneState {
    pub rect: Rect,
    pub content: ZoneContent,
    hash: Option<Hash>,
    pub is_dirty: bool,
}

impl ZoneState {
    fn new(rect: Rect) -> Self {
        Self {
            rect,
            content: ZoneContent::new(),
            hash: None,
            is_dirty: true,
        }
    }

    fn update_content(&mut self, content: ZoneContent) {
        let new_hash = blake3::hash(content.as_bytes());
        if self.hash.map(|h| h != new_hash).unwrap_or(true) {
            self.content = content;
            self.hash = Some(new_hash);
            self.is_dirty = true;
        }
    }
}

/// Registry mapping layout zones to their last known states.
#[derive(Debug, Default)]
pub struct ZoneRegistry {
    entries: HashMap<ZoneId, ZoneState>,
    dirty: HashSet<ZoneId>,
}

impl ZoneRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sync_layout(&mut self, solved_rects: &HashMap<ZoneId, Rect>) {
        use std::collections::hash_map::Entry;

        let mut newly_dirty = Vec::new();

        for (id, rect) in solved_rects {
            match self.entries.entry(id.clone()) {
                Entry::Occupied(mut entry) => {
                    let state = entry.get_mut();
                    if state.rect != *rect {
                        state.rect = *rect;
                        state.is_dirty = true;
                        newly_dirty.push(id.clone());
                    }
                }
                Entry::Vacant(vacant) => {
                    vacant.insert(ZoneState::new(*rect));
                    newly_dirty.push(id.clone());
                }
            }
        }

        // Remove zones no longer present.
        let to_remove: Vec<_> = self
            .entries
            .keys()
            .filter(|id| !solved_rects.contains_key(*id))
            .cloned()
            .collect();
        for id in to_remove {
            self.entries.remove(&id);
            self.dirty.remove(&id);
        }

        for id in newly_dirty {
            self.dirty.insert(id);
        }
    }

    pub fn apply_content(&mut self, zone_id: &ZoneId, content: ZoneContent) -> Result<()> {
        let entry = self
            .entries
            .get_mut(zone_id)
            .ok_or_else(|| LayoutError::ZoneNotFound(zone_id.clone()))?;
        entry.update_content(content);
        if entry.is_dirty {
            self.dirty.insert(zone_id.clone());
        }
        Ok(())
    }

    pub fn take_dirty(&mut self) -> Vec<(ZoneId, ZoneState)> {
        let ids: Vec<_> = self.dirty.drain().collect();
        ids.into_iter()
            .filter_map(|id| {
                self.entries.get_mut(&id).map(|state| {
                    state.is_dirty = false;
                    (id.clone(), state.clone())
                })
            })
            .collect()
    }

    pub fn rect_of(&self, zone_id: &ZoneId) -> Option<Rect> {
        self.entries.get(zone_id).map(|state| state.rect)
    }

    pub fn has_dirty(&self) -> bool {
        !self.dirty.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn rect() -> Rect {
        Rect::new(0, 0, 10, 5)
    }

    #[test]
    fn sync_layout_flags_new_zones_as_dirty() {
        let mut registry = ZoneRegistry::new();
        let mut solved = HashMap::new();
        solved.insert("zone".to_string(), rect());

        registry.sync_layout(&solved);
        let dirty = registry.take_dirty();
        assert_eq!(dirty.len(), 1);
        assert_eq!(dirty[0].0, "zone");
    }

    #[test]
    fn apply_content_detects_changes() {
        let mut registry = ZoneRegistry::new();
        let mut solved = HashMap::new();
        solved.insert("zone".to_string(), rect());
        registry.sync_layout(&solved);
        registry.take_dirty();

        registry
            .apply_content(&"zone".to_string(), "hello".to_string())
            .unwrap();
        let dirty = registry.take_dirty();
        assert_eq!(dirty.len(), 1);

        registry
            .apply_content(&"zone".to_string(), "hello".to_string())
            .unwrap();
        let dirty_again = registry.take_dirty();
        assert!(dirty_again.is_empty());
    }
}
