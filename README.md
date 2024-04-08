# Reverse Proxy Server written in Rust

A simple reverse proxy server written in Rust intended for local development.

Do not use in production (not blazingly fast).

## Stack and features

- Use axum as the web framework.
- Use reqwest to make the requests to the target API.
- Support for CORS (permissive mode).
- Proxies multiple targets
- Supports binary files like images
- No support for websockets.

## Configuration 

`config.toml` file is required to run the proxy.

```toml
targets = [
    { host = "example.com", secure = true, source_path = "/api", dest_path = "/api/v1" },
    { host = "example2.com", secure = true, source_path = "/assets", dest_path = "/static/assets" },
    { host = "localhost:3000", secure = false, source_path = "/webhooks", dest_path = "/webhooks/main" },
    { host = "localhost:4200", secure = false, source_path = "/", dest_path = "/" },
]

cors = true 
port = 4200
```

Resulting in the following proxy configuration:

```
http://localhost:4200/api/foo/bar -> https://example.com/api/v1/foo/bar
http://localhost:4200/assets/img/angular.jpg -> https://example2.com/static/assets/img/angular.jpg
http://localhost:4200/webhooks/stripe -> http://localhost:3000/webhooks/main/stripe
http://localhost:4200/ -> http://localhost:4200/
http://localhost:4200/any/path -> http://localhost:4200/any/path
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
