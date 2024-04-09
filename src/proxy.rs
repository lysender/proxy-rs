use axum::{
    body::{Body, Bytes},
    extract::{OriginalUri, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    response::Response,
    routing::any,
    Router,
};
use reqwest::{
    header::HeaderMap as ReqwestHeaderMap, Client, Method as ReqwestMethod,
    Response as ReqwestResponse,
};
use std::{str::FromStr, sync::Arc};

use crate::{
    config::{Config, ProxyAuth, ProxyTarget},
    error::Result,
    run::AppState,
};

pub fn routes_proxy(state: AppState) -> Router {
    // Route root and fallback to the same proxy handler
    Router::new()
        .route("/", any(proxy_handler))
        .fallback(any(proxy_handler))
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

async fn proxy_handler(
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
    for (name, value) in headers.iter() {
        if name == "host" {
            // Change origin host to target host
            r = r.header("host", &target.host);
        } else {
            r = r.header(name, value);
        }
    }

    // Inject auth headers if specified
    if target.use_auth {
        let Some(auth) = &state.config.auth else {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Proxy auth config missing."))
                .unwrap();
        };

        match fetch_auth(state.client.clone(), &headers, auth).await {
            Ok(auth_headers) => {
                // Inject headers from the auth response
                for name in auth.response_headers.iter() {
                    if let Some(value) = auth_headers.get(name) {
                        r = r.header(name, value);
                    }
                }
            }
            Err(e) => {
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(format!("Proxy auth error: {}", e)))
                    .unwrap();
            }
        };
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

    // Stream target response into the response body
    // instead of loading them all into memory
    let stream = res.bytes_stream();
    r.body(Body::from_stream(stream)).unwrap()
}

async fn fetch_auth(
    client: Arc<Client>,
    headers: &HeaderMap,
    auth: &ProxyAuth,
) -> Result<ReqwestHeaderMap> {
    // Build auth url
    let host = auth.host.as_str();
    let prefix = match auth.secure {
        true => "https://",
        false => "http://",
    };
    let url = format!("{}{}{}", prefix, host, auth.path.as_str());
    let mut req = client.request(
        ReqwestMethod::from_bytes(auth.method.as_str().as_bytes()).unwrap(),
        url,
    );

    // Populate headers from the original request
    for name in auth.request_headers.iter() {
        if let Some(value) = headers.get(name) {
            req = req.header(name, value.to_str().unwrap());
        }
    }

    let res = req.send().await?;
    Ok(res.headers().clone())
}
