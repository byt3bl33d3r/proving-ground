//! Shadow model: what the server must contain, given only acknowledged operations.

use std::collections::{BTreeMap, BTreeSet};

/// Acknowledged state plus operations whose outcome is unknown (timeouts, partitions).
#[derive(Debug, Default)]
pub struct Model {
    /// Items whose create was acknowledged and that were not deleted since.
    pub live: BTreeMap<String, String>,
    /// Names whose create may or may not have been applied.
    pub maybe_created: BTreeSet<String>,
    /// Ids whose delete may or may not have been applied.
    pub maybe_deleted: BTreeSet<String>,
    /// Ids whose delete was acknowledged.
    pub deleted: BTreeSet<String>,
}

impl Model {
    /// Checks the server's final listing (`id -> name`) against the model.
    pub fn check(&self, listed: &BTreeMap<String, String>) -> Result<(), String> {
        for (id, name) in &self.live {
            if listed.get(id) != Some(name) && !self.maybe_deleted.contains(id) {
                return Err(format!(
                    "acknowledged create {id} ({name}) is missing from list"
                ));
            }
        }
        if let Some(id) = self.deleted.iter().find(|id| listed.contains_key(*id)) {
            return Err(format!("acknowledged delete {id} is still listed"));
        }
        let unexpected = listed
            .iter()
            .find(|(id, name)| !self.live.contains_key(*id) && !self.maybe_created.contains(*name));
        match unexpected {
            Some((id, name)) => Err(format!("listed item {id} ({name}) was never created")),
            None => Ok(()),
        }
    }
}
