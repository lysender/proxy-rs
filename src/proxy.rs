use axum::{
    extract::{Request, State},
    http::{HeaderName, HeaderValue, StatusCode},
    response::Response,
    routing::get,
    Router,
};
use reqwest::{Client, Method, Response as ReqwestResponse};
use std::str::FromStr;
use tracing::{error, info};

use crate::run::AppState;

pub fn routes_proxy(state: AppState) -> Router {
    Router::new()
        .route(state.config.proxy_target_path.as_str(), get(handler_proxy))
        .with_state(state)
}

async fn handler_proxy(State(state): State<AppState>, request: Request) -> Response<String> {
    let client = Client::new();

    // Build the incoming request into a reqwest request
    let path = request.uri().path();
    let host = &state.config.proxy_target_host;
    let prefix = match state.config.proxy_target_secure {
        true => "https://",
        false => "http://",
    };
    let url = format!("{}{}{}", prefix, host, path);
    let method = request.method().as_str().as_bytes();
    let headers = request.headers();

    let mut r = client.request(Method::from_bytes(method).unwrap(), &url);
    // Populate headers
    for (name, value) in headers {
        if name == "host" {
            // Change origin host to target host
            r = r.header("host", &state.config.proxy_target_host);
        } else {
            r = r.header(name.as_str(), value.to_str().unwrap());
        }
    }

    let response = r.send().await;
    match response {
        Ok(res) => {
            info!("{} {}", request.method().as_str(), path);
            build_proxy_response(res).await
        }
        Err(e) => {
            error!("Error: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(format!("Error: {}", e))
                .unwrap()
        }
    }
}

async fn build_proxy_response(res: ReqwestResponse) -> Response<String> {
    let mut r = Response::builder().status(res.status().as_u16());

    // Inject headers from the remote response
    for (name, value) in res.headers() {
        let header_name = HeaderName::from_str(name.as_str()).unwrap();
        let header_value = HeaderValue::from_bytes(value.as_bytes()).unwrap();
        r = r.header(header_name, header_value);
    }

    let body = res.text().await.unwrap();
    r.body(body).unwrap()
}
