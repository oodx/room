use crate::geometry::Rect;
use crate::registry::ZoneId;

/// Immutable metadata describing a zone in the layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneDescriptor {
    pub id: ZoneId,
    pub rect: Rect,
}

impl ZoneDescriptor {
    pub fn new(id: ZoneId, rect: Rect) -> Self {
        Self { id, rect }
    }
}
