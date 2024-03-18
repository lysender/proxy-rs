# Poor Man's API Proxy written in Rust

This is a simple API proxy written in Rust to be used for local development.

Do not use in production (not blazingly fast).

## Stack and features

- Use axum as the web framework.
- Use reqwest to make the requests to the target API.
- Support for CORS (permissive mode).
- No support for websockets.

## Configuration 

`config.toml` file is required to run the proxy.

```toml
proxy_target_host = "example.com"
proxy_target_secure = true
proxy_target_path = "/api/*rest"
cors = true 
port = 4200
```

Resulting in the following proxy configuration:

```
http://localhost:4200/api/foo/bar -> https://example.com/api/foo/bar
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
