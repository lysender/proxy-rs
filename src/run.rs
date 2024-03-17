use axum::extract::FromRef;
use axum::routing::{get, get_service};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use axum::{Json, Router};
use hyper::body::Body;
use hyper::{Request, Response, StatusCode};
use serde_json::json;
use std::error::Error;
use std::path::PathBuf;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::info;
use tracing::Level;

use std::net::SocketAddr;

use crate::config::Config;
use crate::error::Result;

#[derive(Clone, FromRef)]
pub struct AppState {
    pub config: Config,
}

pub async fn run(config: Config) -> Result<()> {
    let state = AppState {
        config: config.clone(),
    };

    let routes_all = Router::new()
        .merge(routes_index(state.clone()))
        .fallback_service(routes_fallback(state))
        .layer(
            ServiceBuilder::new().layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                    .on_response(DefaultOnResponse::new().level(Level::INFO)),
            ),
        );

    // Setup the server
    let ip = "127.0.0.1";
    let addr = format!("{}:{}", ip, config.port);
    info!("Listening on {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, routes_all.into_make_service())
        .await
        .unwrap();

    Ok(())
}

fn routes_index(state: AppState) -> Router {
    Router::new()
        .route("/", get(handler_index))
        .with_state(state)
}

async fn handler_index() -> impl IntoResponse {
    (StatusCode::OK, Html("<h1>API Proxy</h1>"))
}

fn routes_fallback(state: AppState) -> Router {
    // 404 handler
    Router::new().nest_service("/", get(handle_error).with_state(state))
}

async fn handle_error() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, Html("<h1>Not Found</h1>"))
}
