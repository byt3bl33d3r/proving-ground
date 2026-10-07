// UI test input for the project lints. Expected diagnostics are in main.stderr.
#![allow(unused)]

#[tracing::instrument(fields(item_id = 1, badName = 2))]
fn instrumented() {}

fn main() {
    let id = 7_u32;
    let name = "widget";
    tracing::info!(item_id = %id, "item_created");
    tracing::info!("created item {}", id);
    tracing::warn!("created item {id}");
    tracing::error!(?name, "failed");
    tracing::debug!(item_id = id, "literal {{braces}} are fine");
    tracing::event!(tracing::Level::INFO, "event {}", name);
    tracing::info!(itemId = %id, http.route = "/items", "item_created");
    let _span = tracing::info_span!("create_item", RequestId = 1, http.request.method = "GET");
}
