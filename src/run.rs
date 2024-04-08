use std::sync::Arc;
use std::time::Duration;

use axum::extract::{DefaultBodyLimit, FromRef};
use axum::Router;
use reqwest::Client;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::{info, Level};

use crate::config::Config;
use crate::error::Result;
use crate::proxy::routes_proxy;

#[derive(Clone, FromRef)]
pub struct AppState {
    pub config: Config,
    pub client: Arc<Client>,
}

pub async fn run(config: Config) -> Result<()> {
    let state = AppState {
        config: config.clone(),
        client: Arc::new(Client::new()),
    };

    let mut routes_all = Router::new()
        .fallback_service(routes_proxy(state))
        .layer(DefaultBodyLimit::max(8000000))
        .layer(RequestBodyLimitLayer::new(8000000));

    if config.cors {
        let cors = CorsLayer::permissive().max_age(Duration::from_secs(60) * 10);
        routes_all = routes_all.layer(cors);
    }

    routes_all = routes_all.layer(
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
