use clap::Parser;
use config::{Args, Config};
use std::process;

mod config;
mod error;
mod proxy;
mod run;

#[tokio::main]
async fn main() {
    // Set the RUST_LOG, if it hasn't been explicitly defined
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "api_proxy=info,tower_http=info")
    }

    // tracing_subscriber::fmt::init();
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    let args = Args::parse();
    let config = Config::build(args.config.as_path()).unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        process::exit(1);
    });

    if let Err(e) = run::run(config).await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
