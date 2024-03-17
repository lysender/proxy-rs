use clap::Parser;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use toml;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub proxy_target_host: String,
    pub proxy_target_path: String,
    pub port: u16,
}

impl Config {
    pub fn build(filename: &Path) -> Result<Config, &'static str> {
        let toml_str = match fs::read_to_string(filename) {
            Ok(s) => s,
            Err(_) => return Err("Failed to read file"),
        };
        let config: Config = match toml::from_str(toml_str.as_str()) {
            Ok(c) => c,
            Err(_) => return Err("Failed to parse TOML"),
        };
        Ok(config)
    }
}

/// HTTP proxy for remote server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// TOML configuration file
    #[arg(short, long, value_name = "config.toml")]
    pub config: PathBuf,
}
