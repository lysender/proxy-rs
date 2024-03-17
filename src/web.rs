use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};

use crate::run::AppState;

pub fn routes_index(state: AppState) -> Router {
    Router::new()
        .route("/", get(handler_index))
        .with_state(state)
}

async fn handler_index() -> impl IntoResponse {
    (StatusCode::OK, Html("<h1>API Proxy</h1>"))
}

pub fn routes_fallback(state: AppState) -> Router {
    // 404 handler
    Router::new().nest_service("/", get(handle_error).with_state(state))
}

async fn handle_error() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, Html("<h1>Not Found</h1>"))
}
