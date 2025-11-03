# Reverse Proxy Server written in Rust

A simple reverse proxy server written in Rust intended for local development.

Do not use in production (not blazingly fast).

## Stack and features

- Use `axum` as the web framework.
- Use `reqwest` to make the requests to the target API.
- Support for CORS (permissive mode).
- Proxies multiple targets
- Supports binary files like images
- Supports simple auth middleware
- No support for websockets.

## Configuration 

`config.toml` file is required to run the proxy.

```toml
targets = [
    # JSON API without authentication middleware
    { host = "example.com", secure = true, source_path = "/api", dest_path = "/api/v1", use_auth = false },

    # Static files
    { host = "example2.com", secure = true, source_path = "/assets", dest_path = "/static/assets", use_auth = false },

    # Other endpoints
    { host = "localhost:3000", secure = false, source_path = "/webhooks", dest_path = "/webhooks/main", use_auth = false },

    # Use a certain endpoint as catch all proxy target
    { host = "localhost:4200", secure = false, source_path = "/", dest_path = "/", use_auth = false },
]

cors = true
port = 4200

# When true, behaves like AWS API gateway that returns 200 OK on error responses
ignore_errors = false
```

Resulting in the following proxy configuration:

```
http://localhost:4200/api/foo/bar -> https://example.com/api/v1/foo/bar
http://localhost:4200/assets/img/angular.jpg -> https://example2.com/static/assets/img/angular.jpg
http://localhost:4200/webhooks/stripe -> http://localhost:3000/webhooks/main/stripe
http://localhost:4200/ -> http://localhost:4200/
http://localhost:4200/any/path -> http://localhost:4200/any/path
```

## Auth Middleware

Auth middleware allows proxy to inject additional headers or replace headers
by fetching auth data from the specified auth server, before sending it to target hosts.

Note: Currently, it only supports simple GET/POST requests with no body for data.

```toml
# Proxy targets
targets = [
    # JSON API with authentication middleware
    { host = "example.com", secure = true, source_path = "/api", dest_path = "/api/v1", use_auth = true },

    # Static files
    { host = "example2.com", secure = true, source_path = "/assets", dest_path = "/static/assets", use_auth = false },

    # Other endpoints
    { host = "localhost:3000", secure = false, source_path = "/webhooks", dest_path = "/webhooks/main", use_auth = false },

    # Use this endpoint as catch all proxy target
    { host = "localhost:4200", secure = false, source_path = "/", dest_path = "/", use_auth = false },
]

cors = true
port = 4200

# When true, behaves like AWS API gateway that returns 200 OK on error responses
ignore_errors = false

# Optional auth middleware
# Before forwarding the request to target host,
# request for authentication headers first from this server.
# Inject the specified response headers into the request to the target host.
# Note: header names are more likely in lowercase
[auth]
host = "127.0.0.1:9000"
secure = false
path = "/auth"
request_headers = ["authorization"]
response_headers = ["authorization"]
method = "POST"
```

## Running

Development mode:

```bash
cargo run -- -c /path/to/config.toml
```

Release mode (build it first then copy the binary to your $PATH):

```bash
cargo build --release
```

Run the binary:

```bash
api-proxy -c /path/to/config.toml
```
