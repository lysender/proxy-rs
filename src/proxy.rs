use axum::{
    Router,
    body::{Body, Bytes},
    extract::{ConnectInfo, OriginalUri, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    response::Response,
    routing::any,
};
use reqwest::{
    Client, Method as ReqwestMethod, Response as ReqwestResponse,
    header::HeaderMap as ReqwestHeaderMap,
};
use std::net::SocketAddr;
use std::str::FromStr;
use tracing::debug;

use crate::{
    Result,
    config::{Config, ProxyAuth, ProxyTarget},
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
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
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
    debug!("Forward request to: {}", url);

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
            // Exclude some headers if target user auth
            let mut skip_header = false;
            if let Some(auth) = &state.config.auth {
                if auth.request_headers.contains(&name.to_string()) {
                    skip_header = true;
                }
            }

            // Will extract this header separately
            if name == "x-forwarded-for" {
                skip_header = true;
            }

            if !skip_header {
                r = r.header(name, value);
            }
        }
    }

    // Inject auth headers if specified
    if target.use_auth {
        debug!("Using auth middleware");

        let Some(auth) = &state.config.auth else {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Proxy auth config missing."))
                .unwrap();
        };

        match fetch_auth(state.client.clone(), &headers, auth).await {
            Ok(auth_headers) => {
                debug!("Injecting auth headers.");
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

    // Add x-forwarded-for header
    if let Some(forwarded_for) = headers.get("x-forwarded-for") {
        let mut new_value = forwarded_for.to_str().unwrap().to_string();
        if new_value.len() > 0 {
            // Append client address
            new_value = format!("{}, {}", new_value, addr.ip());
            r = r.header("x-forwarded-for", new_value);
        }
    } else {
        // Add client address
        r = r.header("x-forwarded-for", addr.ip().to_string());
    }

    // Populate body into reqwest request
    if body.len() > 0 {
        r = r.body(body);
    }

    let response = r.send().await;
    match response {
        Ok(res) => build_proxy_response(res, target.ignore_errors).await,
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from(format!("Error: {}", e)))
            .unwrap(),
    }
}

async fn build_proxy_response(res: ReqwestResponse, ignore_errors: bool) -> Response<Body> {
    let mut status = res.status();
    if !status.is_success() && ignore_errors {
        status = StatusCode::OK;
    }
    let mut r = Response::builder().status(status.as_u16());

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
    client: Client,
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

    match req.send().await {
        Ok(res) => Ok(res.headers().clone()),
        Err(e) => {
            let msg = format!("Failed to fetch auth: {}", e);
            Err(msg.into())
        }
    }
}
