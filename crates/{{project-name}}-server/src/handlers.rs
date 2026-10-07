//! Route handlers. Each one validates input, calls the domain and maps errors to [`ApiError`].

use axum::Json;
use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use {{crate_name}}_core::types::{CreateItem, Item, ItemPage};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{ApiError, AppState};

/// Query string of `GET /items`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListParams {
    offset: Option<u32>,
    limit: Option<u32>,
}

pub(crate) async fn create_item(
    State(state): State<AppState>,
    body: Result<Json<CreateItem>, JsonRejection>,
) -> Result<(StatusCode, Json<Item>), ApiError> {
    let Json(request) = body.map_err(|rejection| ApiError::from_json(&rejection))?;
    let item = state.items().create(&request.name).await?;
    Ok((StatusCode::CREATED, Json(item)))
}

pub(crate) async fn get_item(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Item>, ApiError> {
    Ok(Json(state.items().get(&id).await?))
}

pub(crate) async fn list_items(
    State(state): State<AppState>,
    params: Result<Query<ListParams>, QueryRejection>,
) -> Result<Json<ItemPage>, ApiError> {
    let Query(params) = params.map_err(|rejection| ApiError::from_query(&rejection))?;
    Ok(Json(state.items().list(params.offset, params.limit).await?))
}

pub(crate) async fn delete_item(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    state.items().delete(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn readyz(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    if state.is_ready() {
        (StatusCode::OK, Json(json!({ "status": "ready" })))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "starting" })),
        )
    }
}

pub(crate) async fn no_route() -> ApiError {
    ApiError::new(
        StatusCode::NOT_FOUND,
        "no_route",
        "no route matches this method and path",
    )
}
