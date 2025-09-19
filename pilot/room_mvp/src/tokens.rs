use std::collections::HashMap;

use rsb::token::{Token, format::unescape_token, tokenize_string};

use crate::error::{LayoutError, Result};
use crate::registry::{ZoneContent, ZoneId};

/// Materialised token update for a layout zone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneTokenUpdate {
    pub zone_id: ZoneId,
    pub content: ZoneContent,
}

#[derive(Debug, Default)]
pub struct ZoneTokenRouter {
    default_context: String,
}

impl ZoneTokenRouter {
    pub fn new() -> Self {
        Self {
            default_context: "app".to_string(),
        }
    }

    pub fn with_default_context(context: impl Into<String>) -> Self {
        Self {
            default_context: context.into(),
        }
    }

    pub fn route(&self, stream: &str) -> Result<Vec<ZoneTokenUpdate>> {
        let tokens =
            tokenize_string(stream).map_err(|err| LayoutError::TokenRouting(err.to_string()))?;
        self.route_tokens(&tokens)
    }

    pub fn route_tokens(&self, tokens: &[Token]) -> Result<Vec<ZoneTokenUpdate>> {
        let mut context = self.default_context.clone();
        let mut namespace: Option<String> = None;
        let mut zones: HashMap<ZoneId, ZoneAccumulator> = HashMap::new();

        for token in tokens {
            if token.namespace.is_none() {
                match token.key.as_str() {
                    "ctx" => {
                        context = token.value.clone();
                        continue;
                    }
                    "ns" => {
                        namespace = Some(token.value.clone());
                        continue;
                    }
                    _ => {}
                }
            }

            let ns = token
                .namespace
                .as_ref()
                .map(|ns| ns.to_string())
                .or_else(|| namespace.clone())
                .ok_or_else(|| {
                    LayoutError::TokenRouting(format!("missing namespace for token {}", token))
                })?;

            let zone_id = format!("{}:{}", context, ns);
            let entry = zones.entry(zone_id).or_default();
            let normalized_value = unescape_token(&token.value);
            entry.push(&token.key, normalized_value);
        }

        let mut updates: Vec<ZoneTokenUpdate> = zones
            .into_iter()
            .map(|(zone_id, acc)| ZoneTokenUpdate {
                zone_id,
                content: acc.build_content(),
            })
            .collect();

        updates.sort_by(|a, b| a.zone_id.cmp(&b.zone_id));
        Ok(updates)
    }
}

#[derive(Debug, Default)]
struct ZoneAccumulator {
    override_content: Option<ZoneContent>,
    pairs: Vec<(String, String)>,
}

impl ZoneAccumulator {
    fn push(&mut self, key: &str, value: String) {
        match key {
            "content" | "text" => {
                self.override_content = Some(value);
            }
            _ => {
                self.pairs.push((key.to_string(), value));
            }
        }
    }

    fn build_content(mut self) -> ZoneContent {
        if let Some(content) = self.override_content {
            return content;
        }

        if self.pairs.is_empty() {
            return String::new();
        }

        self.pairs.sort_by(|a, b| a.0.cmp(&b.0));
        self.pairs
            .into_iter()
            .map(|(key, value)| format!("{}: {}", key, value))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_with_context_switch() {
        let router = ZoneTokenRouter::new();
        let updates = router
            .route("ctx=app; ns=chat.timeline; content=Hello; ctx=user; ns=meta; status=ok;")
            .unwrap();
        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].zone_id, "app:chat.timeline");
        assert_eq!(updates[0].content, "Hello");
        assert_eq!(updates[1].zone_id, "user:meta");
        assert_eq!(updates[1].content, "status: ok");
    }
}
