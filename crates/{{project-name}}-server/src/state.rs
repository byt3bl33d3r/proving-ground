//! Shared application state.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use {{crate_name}}_core::domain::ItemService;
use {{crate_name}}_core::platform::{Clock, Rng};
use {{crate_name}}_core::repo::InMemoryItemRepo;

use crate::metrics::HttpMetrics;

/// Everything handlers need. Cheap to clone.
#[derive(Clone, Debug)]
pub struct AppState {
    items: ItemService,
    clock: Arc<dyn Clock>,
    rng: Arc<dyn Rng>,
    ready: Arc<AtomicBool>,
    request_timeout: Duration,
    http_metrics: HttpMetrics,
}

impl AppState {
    /// State with an empty in-memory repository.
    pub fn new(clock: Arc<dyn Clock>, rng: Arc<dyn Rng>, request_timeout: Duration) -> Self {
        let repo = Arc::new(InMemoryItemRepo::default());
        let items = ItemService::new(repo, Arc::clone(&clock), Arc::clone(&rng));
        Self {
            items,
            clock,
            rng,
            ready: Arc::new(AtomicBool::new(false)),
            request_timeout,
            http_metrics: HttpMetrics::new(),
        }
    }

    /// The items service.
    pub const fn items(&self) -> &ItemService {
        &self.items
    }

    /// The clock shared with the domain.
    pub fn clock(&self) -> Arc<dyn Clock> {
        Arc::clone(&self.clock)
    }

    /// The RNG shared with the domain.
    pub fn rng(&self) -> Arc<dyn Rng> {
        Arc::clone(&self.rng)
    }

    /// Per-request timeout.
    pub const fn request_timeout(&self) -> Duration {
        self.request_timeout
    }

    pub(crate) const fn http_metrics(&self) -> &HttpMetrics {
        &self.http_metrics
    }

    /// Marks the service ready: call once the router is serving and telemetry is initialized.
    pub fn mark_ready(&self) {
        self.ready.store(true, Ordering::Release);
    }

    /// Whether `/readyz` reports ready.
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }
}
