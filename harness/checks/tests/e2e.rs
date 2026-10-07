//! End-to-end tests against this worktree's running server (`just up`, then `just e2e`).
//! They use the CLI's typed client, so the CLI and server contract is tested over a real socket
//! with full telemetry. `just test` skips this binary; `just budgets` runs after it.
#![cfg(test)]

use std::path::PathBuf;

use demo_app_cli::client::{Client, ClientError};
use demo_app_cli::harness::AppInfo;
use demo_app_core::types::ItemId;

fn client() -> Client {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.harness");
    let Some(app) = AppInfo::read(&dir) else {
        panic!(
            "No running server: run just up first (missing {}/app.json)",
            dir.display()
        );
    };
    Client::new(&app.url).expect("client")
}

fn api_error(result: Result<impl std::fmt::Debug, ClientError>) -> (u16, String, Option<String>) {
    match result {
        Err(ClientError::Api { status, body }) => (status, body.code, body.request_id),
        other => panic!("expected an API error, got {other:?}"),
    }
}

#[tokio::test]
async fn item_lifecycle() {
    let api = client();
    let created = api.create_item("e2e pen").await.expect("create");
    assert_eq!(created.name.as_str(), "e2e pen", "name round-trips");
    assert_eq!(
        api.get_item(created.id).await.expect("get"),
        created,
        "get returns the created item"
    );
    api.delete_item(created.id).await.expect("delete");
    let (status, code, request_id) = api_error(api.get_item(created.id).await);
    assert_eq!(
        (status, code.as_str()),
        (404, "not_found"),
        "gone after delete"
    );
    assert!(
        request_id.is_some(),
        "error bodies carry request_id for just logs-request"
    );
}

#[tokio::test]
async fn validation_errors_are_400_with_codes() {
    let api = client();
    let blank = api_error(api.create_item("   ").await);
    assert_eq!(
        (blank.0, blank.1.as_str()),
        (400, "validation_error"),
        "blank name"
    );
    let zero_limit = api_error(api.list_items(None, Some(0)).await);
    assert_eq!(
        (zero_limit.0, zero_limit.1.as_str()),
        (400, "validation_error"),
        "zero limit"
    );
    let unknown = api_error(api.delete_item(ItemId::from_bits(0)).await);
    assert_eq!(
        (unknown.0, unknown.1.as_str()),
        (404, "not_found"),
        "unknown id"
    );
}

#[tokio::test]
async fn sustained_traffic_lists_every_item() {
    let api = client();
    let mut ids = Vec::new();
    for n in 0..50 {
        let item = api.create_item(&format!("load {n}")).await.expect("create");
        assert_eq!(
            api.get_item(item.id).await.expect("get").id,
            item.id,
            "readable after create"
        );
        ids.push(item.id);
    }
    let mut listed = Vec::new();
    let mut offset = None;
    loop {
        let page = api.list_items(offset, Some(100)).await.expect("list");
        listed.extend(page.items.into_iter().map(|item| item.id));
        match page.next_offset {
            Some(next) => offset = Some(next),
            None => break,
        }
    }
    assert!(
        ids.iter().all(|id| listed.contains(id)),
        "every created item is listed"
    );
    for id in ids {
        api.delete_item(id).await.expect("cleanup");
    }
}
