use clap::Parser;
use config::{Args, Config};
use std::{process, str::FromStr};
use tracing::Level;

mod config;
mod error;
mod proxy;
mod run;

pub use error::{Error, Result};

#[tokio::main]
async fn main() {
    let mut max_log = Level::INFO;
    if let Some(rust_log_max) = std::env::var_os("RUST_LOG_MAX") {
        max_log = Level::from_str(rust_log_max.to_str().unwrap()).unwrap_or_else(|e| {
            eprintln!("Error: {}", e);
            process::exit(1);
        });
    }

    tracing_subscriber::fmt()
        .with_max_level(max_log)
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
