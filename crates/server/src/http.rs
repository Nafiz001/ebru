use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use marbling::{GROUNDS, PIGMENTS};
use serde::{Deserialize, Serialize};
use tower_http::compression::CompressionLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use crate::{AppState, ws};

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/tray", get(tray))
        .route("/pigments", get(pigments))
        .route("/prints", get(prints))
        .route("/prints/{id}", get(print))
        .route("/prints/{id}/image", get(image))
        .route("/prints/{id}/recipe", get(recipe));

    let mut app = Router::new()
        .nest("/api", api)
        .route("/ws", get(ws::upgrade))
        .route("/health", get(|| async { "ok" }));

    if let Some(web) = &state.config.web {
        app = app.fallback_service(ServeDir::new(web).fallback(ServeFile::new(web.join("index.html"))));
    }

    app.layer(CompressionLayer::new()).layer(TraceLayer::new_for_http()).with_state(state)
}

fn problem(status: StatusCode, message: &str) -> Response {
    (status, axum::Json(serde_json::json!({ "error": message }))).into_response()
}

/// The whole tray as a binary snapshot, for a browser to start from.
async fn tray(State(state): State<AppState>) -> Response {
    let bytes = state.workshop.snapshot().await;
    ([(header::CONTENT_TYPE, "application/octet-stream"), (header::CACHE_CONTROL, "no-store")], bytes).into_response()
}

#[derive(Serialize)]
struct PigmentView {
    index: u8,
    name: &'static str,
    hex: String,
    ground: bool,
}

async fn pigments() -> axum::Json<Vec<PigmentView>> {
    axum::Json(
        PIGMENTS
            .iter()
            .enumerate()
            .map(|(i, p)| PigmentView { index: i as u8, name: p.name, hex: p.hex(), ground: GROUNDS.contains(&(i as u8)) })
            .collect(),
    )
}

#[derive(Deserialize)]
struct Page {
    before: Option<i64>,
    limit: Option<u32>,
}

async fn prints(State(state): State<AppState>, Query(page): Query<Page>) -> Response {
    let store = state.store.clone();
    let limit = page.limit.unwrap_or(24).clamp(1, 100);
    match tokio::task::spawn_blocking(move || store.list(page.before, limit)).await {
        Ok(Ok(prints)) => axum::Json(prints).into_response(),
        _ => problem(StatusCode::INTERNAL_SERVER_ERROR, "The gallery couldn't be read."),
    }
}

async fn print(State(state): State<AppState>, Path(id): Path<i64>) -> Response {
    let store = state.store.clone();
    match tokio::task::spawn_blocking(move || store.get(id)).await {
        Ok(Ok(Some(print))) => axum::Json(print).into_response(),
        Ok(Ok(None)) => problem(StatusCode::NOT_FOUND, "There's no print with that number."),
        _ => problem(StatusCode::INTERNAL_SERVER_ERROR, "The gallery couldn't be read."),
    }
}

#[derive(Deserialize)]
struct Size {
    size: Option<String>,
}

async fn image(State(state): State<AppState>, Path(id): Path<i64>, Query(size): Query<Size>) -> Response {
    let path = state.store.image_path(id, size.size.as_deref() == Some("small"));
    match tokio::fs::read(path).await {
        Ok(bytes) => (
            [(header::CONTENT_TYPE, "image/png"), (header::CACHE_CONTROL, "public, max-age=31536000, immutable")],
            bytes,
        )
            .into_response(),
        Err(_) => problem(StatusCode::NOT_FOUND, "There's no print with that number."),
    }
}

/// The ops that made a print, so a browser can replay it.
async fn recipe(State(state): State<AppState>, Path(id): Path<i64>) -> Response {
    let store = state.store.clone();
    match tokio::task::spawn_blocking(move || store.recipe(id)).await {
        Ok(Ok(bytes)) => ([(header::CONTENT_TYPE, "application/json")], bytes).into_response(),
        _ => problem(StatusCode::NOT_FOUND, "There's no print with that number."),
    }
}
