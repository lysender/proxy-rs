use axum::{
    body::{Body, Bytes},
    extract::{OriginalUri, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    response::Response,
    routing::{delete, get, head, patch, post, put},
    Router,
};
use reqwest::{Method as ReqwestMethod, Response as ReqwestResponse};
use std::str::FromStr;

use crate::{
    config::{Config, ProxyTarget},
    run::AppState,
};

pub fn routes_proxy(state: AppState) -> Router {
    Router::new()
        .route("/*rest", get(handler))
        .route("/*rest", head(handler))
        .route("/*rest", post(handler))
        .route("/*rest", put(handler))
        .route("/*rest", patch(handler))
        .route("/*rest", delete(handler))
        .route("/", get(handler))
        .with_state(state)
}

fn find_target<'a>(config: &'a Config, uri: &str) -> Option<&'a ProxyTarget> {
    config
        .targets
        .iter()
        .find(|target| uri.starts_with(&target.source_path))
}

fn default_handler(path: &str) -> Response<Body> {
    // If no matching target, return 404
    // If path is / but there is no target, return default home page response
    match path {
        "/" => Response::builder()
            .status(StatusCode::OK)
            .body(Body::from("<h1>API Proxy</h1>"))
            .unwrap(),
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("<h1>Not Found</h1>"))
            .unwrap(),
    }
}

async fn handler(
    state: State<AppState>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    method: Method,
    body: Bytes,
) -> Response<Body> {
    // Detect from which target the request is for
    // Route request to that target
    let Some(target) = find_target(&state.config, uri.path()) else {
        return default_handler(uri.path());
    };

    // Forward request to target
    let client = state.client.clone();

    // Build the incoming request into a reqwest request
    let mut path = uri.path().to_string();
    // Rewrite paths
    path.replace_range(0..target.source_path.len(), target.dest_path.as_str());

    let host = target.host.as_str();
    let prefix = match target.secure {
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
                r = r.header("host", &target.host);
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
            .body(Body::from(format!("Error: {}", e)))
            .unwrap(),
    }
}

async fn build_proxy_response(res: ReqwestResponse) -> Response<Body> {
    let mut r = Response::builder().status(res.status().as_u16());

    // Inject headers from the remote response
    for (name, value) in res.headers() {
        let header_name = HeaderName::from_str(name.as_str()).unwrap();
        let header_value = HeaderValue::from_bytes(value.as_bytes()).unwrap();
        r = r.header(header_name, header_value);
    }

    let body = res.bytes().await.unwrap();
    r.body(Body::from(body)).unwrap()
}
