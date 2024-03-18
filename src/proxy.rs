use axum::{
    body::Bytes,
    extract::{OriginalUri, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    response::Response,
    routing::{delete, get, head, patch, post, put},
    Router,
};
use reqwest::{Client, Method as ReqwestMethod, Response as ReqwestResponse};
use std::str::FromStr;

use crate::run::AppState;

pub fn routes_proxy(state: AppState) -> Router {
    let path = state.config.proxy_target_path.as_str();
    Router::new()
        .route(path, get(handler))
        .route(path, head(handler))
        .route(path, post(handler))
        .route(path, put(handler))
        .route(path, patch(handler))
        .route(path, delete(handler))
        .with_state(state)
}

async fn handler(
    state: State<AppState>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    method: Method,
    body: Bytes,
) -> Response<String> {
    let client = Client::new();

    // Build the incoming request into a reqwest request
    let path = uri.path();
    let host = &state.config.proxy_target_host;
    let prefix = match state.config.proxy_target_secure {
        true => "https://",
        false => "http://",
    };
    let query = match uri.query() {
        Some(q) => format!("?{}", q),
        None => "".to_string(),
    };
    let url = format!("{}{}{}{}", prefix, host, path, query);

    let mut r = client.request(
        ReqwestMethod::from_bytes(method.as_str().as_bytes()).unwrap(),
        &url,
    );

    // Populate headers
    for (name, value) in headers {
        if let Some(name_val) = name {
            if name_val == "host" {
                // Change origin host to target host
                r = r.header("host", &state.config.proxy_target_host);
            } else {
                r = r.header(name_val.as_str(), value.to_str().unwrap());
            }
        }
    }

    // Populate body into reqwest request
    if body.len() > 0 {
        r = r.body(body);
    }

    let response = r.send().await;
    match response {
        Ok(res) => build_proxy_response(res).await,
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(format!("Error: {}", e))
            .unwrap(),
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
