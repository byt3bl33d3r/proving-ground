//! Item storage behind the [`ItemRepo`] trait. Only [`InMemoryItemRepo`] exists today; a
//! database adapter implements the same trait (see `docs/testing/index.md`).

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Mutex, PoisonError};

use crate::platform::BoxFuture;
use crate::types::{Item, ItemId, Page};

/// Storage failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RepoError {
    /// The store cannot serve requests right now.
    #[error("storage unavailable")]
    Unavailable,
}

/// Future returned by every [`ItemRepo`] method.
pub type RepoFuture<'a, T> = BoxFuture<'a, Result<T, RepoError>>;

/// One page of stored items plus the total count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    /// Items on this page, in id order.
    pub items: Vec<Item>,
    /// Number of stored items.
    pub total: usize,
}

/// Item storage. Listing order is by id, so it is deterministic.
pub trait ItemRepo: Send + Sync + fmt::Debug {
    /// Stores a new item.
    fn insert(&self, item: Item) -> RepoFuture<'_, ()>;
    /// Loads one item.
    fn get(&self, id: ItemId) -> RepoFuture<'_, Option<Item>>;
    /// Loads one page in id order, plus the total number of items.
    fn list(&self, page: Page) -> RepoFuture<'_, Listed>;
    /// Removes an item; returns whether it existed.
    fn delete(&self, id: ItemId) -> RepoFuture<'_, bool>;
}

/// Items in a `BTreeMap` behind a mutex.
#[derive(Debug, Default)]
pub struct InMemoryItemRepo {
    items: Mutex<BTreeMap<ItemId, Item>>,
}

impl InMemoryItemRepo {
    fn with_items<T>(&self, op: impl FnOnce(&mut BTreeMap<ItemId, Item>) -> T) -> T {
        op(&mut self.items.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

impl ItemRepo for InMemoryItemRepo {
    fn insert(&self, item: Item) -> RepoFuture<'_, ()> {
        self.with_items(|items| items.insert(item.id, item));
        Box::pin(std::future::ready(Ok(())))
    }

    fn get(&self, id: ItemId) -> RepoFuture<'_, Option<Item>> {
        let found = self.with_items(|items| items.get(&id).cloned());
        Box::pin(std::future::ready(Ok(found)))
    }

    fn list(&self, page: Page) -> RepoFuture<'_, Listed> {
        let listed = self.with_items(|items| {
            let (start, end) = page.bounds(items.len());
            let len = end.saturating_sub(start);
            Listed {
                items: items.values().skip(start).take(len).cloned().collect(),
                total: items.len(),
            }
        });
        Box::pin(std::future::ready(Ok(listed)))
    }

    fn delete(&self, id: ItemId) -> RepoFuture<'_, bool> {
        let existed = self.with_items(|items| items.remove(&id).is_some());
        Box::pin(std::future::ready(Ok(existed)))
    }
}
