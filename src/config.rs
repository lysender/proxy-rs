use clap::Parser;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use toml;

#[derive(Clone, Debug, Deserialize)]
pub struct ProxyTarget {
    pub host: String,
    pub secure: bool,
    pub source_path: String,
    pub dest_path: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub targets: Vec<ProxyTarget>,
    pub cors: bool,
    pub port: u16,
}

impl Config {
    pub fn build(filename: &Path) -> Result<Config, &'static str> {
        let toml_str = match fs::read_to_string(filename) {
            Ok(s) => s,
            Err(_) => return Err("Failed to read config file."),
        };
        let config: Config = match toml::from_str(toml_str.as_str()) {
            Ok(c) => c,
            Err(_) => return Err("Failed to parse config file."),
        };

        // Simple validation for proxy targets
        for target in config.targets.iter() {
            if target.host.is_empty() {
                return Err("Proxy target host is required.");
            }
            if target.source_path.is_empty() {
                return Err("Proxy target source path is required.");
            }
            if target.dest_path.is_empty() {
                return Err("Proxy target destination path is required.");
            }
        }
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
