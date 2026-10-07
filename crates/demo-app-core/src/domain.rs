//! Domain services. Every operation is a `verb_noun` span; failures record `error` and
//! `otel.status_code = "ERROR"` on it. Time comes only from `Clock`, randomness only from `Rng`.

use std::sync::Arc;
use std::time::Duration;

use tracing::field::{Empty, display};
use tracing::{Span, info, instrument};

use crate::buggify;
use crate::fields;
use crate::platform::{Clock, Rng};
use crate::repo::{ItemRepo, RepoError};
use crate::types::{Item, ItemError, ItemId, ItemName, ItemPage, Page};

/// Extra latency of the `buggify!` slow path in `get`.
pub const SLOW_PATH_DELAY: Duration = Duration::from_millis(300);
/// Extra latency of the `buggify!` delayed write in `create`.
pub const DELAYED_WRITE: Duration = Duration::from_millis(150);

/// The items service.
#[derive(Debug, Clone)]
pub struct ItemService {
    repo: Arc<dyn ItemRepo>,
    clock: Arc<dyn Clock>,
    rng: Arc<dyn Rng>,
}

impl ItemService {
    /// Builds the service from its platform seams.
    pub fn new(repo: Arc<dyn ItemRepo>, clock: Arc<dyn Clock>, rng: Arc<dyn Rng>) -> Self {
        Self { repo, clock, rng }
    }

    /// Validates `name` and stores a new item.
    #[instrument(name = "create_item", skip_all, fields(item_id = Empty, error = Empty, error.type = Empty, otel.status_code = Empty))]
    pub async fn create(&self, name: &str) -> Result<Item, ItemError> {
        let name = ItemName::new(name).map_err(|e| failed(e.into()))?;
        if buggify!() {
            return Err(failed(ItemError::Unavailable));
        }
        let item = Item {
            id: ItemId::from_bits(self.rng.next_u64()),
            name,
            created_at_ms: self.clock.now(),
        };
        Span::current().record(fields::ITEM_ID, display(item.id));
        if buggify!() {
            self.clock.sleep(DELAYED_WRITE).await;
        }
        self.repo
            .insert(item.clone())
            .await
            .map_err(storage_failed)?;
        info!(item_id = %item.id, "item_created");
        Ok(item)
    }

    /// Loads one item by its textual id.
    #[instrument(name = "get_item", skip_all, fields(item_id = %id, error = Empty, error.type = Empty, otel.status_code = Empty))]
    pub async fn get(&self, id: &str) -> Result<Item, ItemError> {
        let id = ItemId::parse(id).map_err(|e| failed(ItemError::Validation(e.into())))?;
        if buggify!() {
            self.clock.sleep(SLOW_PATH_DELAY).await;
        }
        match self.repo.get(id).await.map_err(storage_failed)? {
            Some(item) => Ok(item),
            None => Err(failed(ItemError::NotFound(id))),
        }
    }

    /// Lists one page of items in id order.
    #[instrument(name = "list_items", skip_all, fields(error = Empty, error.type = Empty, otel.status_code = Empty))]
    pub async fn list(
        &self,
        offset: Option<u32>,
        limit: Option<u32>,
    ) -> Result<ItemPage, ItemError> {
        let page = Page::new(offset, limit).map_err(|e| failed(e.into()))?;
        let listed = self.repo.list(page).await.map_err(storage_failed)?;
        let (_, end) = page.bounds(listed.total);
        Ok(ItemPage {
            items: listed.items,
            next_offset: Page::next_offset(end, listed.total),
        })
    }

    /// Deletes one item by its textual id.
    #[instrument(name = "delete_item", skip_all, fields(item_id = %id, error = Empty, error.type = Empty, otel.status_code = Empty))]
    pub async fn delete(&self, id: &str) -> Result<(), ItemError> {
        let id = ItemId::parse(id).map_err(|e| failed(ItemError::Validation(e.into())))?;
        if self.repo.delete(id).await.map_err(storage_failed)? {
            info!(item_id = %id, "item_deleted");
            Ok(())
        } else {
            Err(failed(ItemError::NotFound(id)))
        }
    }
}

/// Records a failure on the current span and passes the error through.
fn failed(err: ItemError) -> ItemError {
    let span = Span::current();
    span.record(fields::ERROR, display(&err));
    span.record(fields::ERROR_TYPE, err.code());
    span.record(fields::OTEL_STATUS_CODE, "ERROR");
    err
}

fn storage_failed(err: RepoError) -> ItemError {
    match err {
        RepoError::Unavailable => failed(ItemError::Unavailable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{SeededRng, TokioClock};
    use crate::repo::InMemoryItemRepo;
    use crate::types::{Timestamp, ValidationError};

    fn service() -> ItemService {
        ItemService::new(
            Arc::new(InMemoryItemRepo::default()),
            Arc::new(TokioClock::new(Timestamp::from_millis(1_700_000_000_000))),
            Arc::new(SeededRng::new(42)),
        )
    }

    #[tokio::test(start_paused = true)]
    async fn create_get_list_delete() {
        let items = service();
        let created = items.create(" pen ").await.expect("create");
        assert_eq!(created.name.as_str(), "pen", "name is trimmed");
        let id = created.id.to_string();
        assert_eq!(
            items.get(&id).await,
            Ok(created.clone()),
            "get returns the item"
        );
        let page = items.list(None, None).await.expect("list");
        assert_eq!(page.items, vec![created], "list has the item");
        assert_eq!(items.delete(&id).await, Ok(()), "delete succeeds");
        assert!(
            matches!(items.get(&id).await, Err(ItemError::NotFound(_))),
            "gone after delete"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn rejects_invalid_input() {
        let items = service();
        assert_eq!(
            items.create("").await,
            Err(ItemError::Validation(ValidationError::EmptyName)),
            "empty name"
        );
        assert!(
            matches!(items.get("nope").await, Err(ItemError::Validation(_))),
            "bad id"
        );
        assert!(
            matches!(
                items.list(None, Some(0)).await,
                Err(ItemError::Validation(_))
            ),
            "limit 0"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn list_paginates_in_id_order() {
        let items = service();
        for name in ["a", "b", "c"] {
            items.create(name).await.expect("create");
        }
        let first = items.list(None, Some(2)).await.expect("first page");
        assert_eq!(first.items.len(), 2, "page size");
        assert_eq!(first.next_offset, Some(2), "next offset");
        let rest = items
            .list(first.next_offset, Some(2))
            .await
            .expect("second page");
        assert_eq!((rest.items.len(), rest.next_offset), (1, None), "last page");
        let ids: Vec<ItemId> = first
            .items
            .iter()
            .chain(&rest.items)
            .map(|item| item.id)
            .collect();
        assert!(ids.is_sorted(), "id order");
    }
}
