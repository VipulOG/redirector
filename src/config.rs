use std::net::IpAddr;

use serde::Deserialize;

use crate::bang::Bang;
use crate::cli::{Cli, SubCommand};

const DEFAULT_SEARCH: &str = "https://www.qwant.com/?q={}";
const DEFAULT_SEARCH_SUGGESTIONS: &str = "https://search.brave.com/api/suggest?q={}";

/// Configuration read from the file.
#[derive(Deserialize, Debug, Default)]
pub struct FileConfig {
    pub port: Option<u16>,
    pub ip: Option<IpAddr>,
    pub bangs_url: Option<String>,
    pub default_search: Option<String>,
    pub search_suggestions: Option<String>,
    pub bangs: Option<Vec<Bang>>,
}

/// Configuration read from the CLI.
#[derive(Debug, Default)]
pub struct Config {
    pub port: Option<u16>,
    pub ip: Option<IpAddr>,
    pub bangs_url: Option<String>,
    pub default_search: Option<String>,
    pub search_suggestions: Option<String>,
}

/// Final application configuration.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct AppConfig {
    pub port: u16,
    pub ip: IpAddr,
    pub bangs_url: String,
    pub default_search: String,
    pub search_suggestions: String,
    pub bangs: Option<Vec<Bang>>,
}

impl Config {
    /// Merge CLI configuration with an optional file configuration.
    /// CLI options take precedence over file values and fall back on `AppConfig` defaults.
    #[allow(dead_code, clippy::must_use_candidate)]
    pub fn merge(self, file: Option<FileConfig>) -> AppConfig {
        let default = AppConfig::default();
        let file = file.unwrap_or(FileConfig {
            port: None,
            ip: None,
            bangs_url: None,
            default_search: None,
            search_suggestions: None,
            bangs: None,
        });
        AppConfig {
            port: self.port.or(file.port).unwrap_or(default.port),
            ip: self.ip.or(file.ip).unwrap_or(default.ip),
            bangs_url: self
                .bangs_url
                .or(file.bangs_url)
                .unwrap_or(default.bangs_url),
            default_search: self
                .default_search
                .or(file.default_search)
                .unwrap_or(default.default_search),
            search_suggestions: self
                .search_suggestions
                .or(file.search_suggestions)
                .unwrap_or(default.search_suggestions),
            bangs: file.bangs,
        }
    }
}

impl FileConfig {
    /// Merge CLI configuration with an optional file configuration.
    /// CLI options take precedence over file values.
    #[allow(dead_code, clippy::must_use_candidate)]
    pub fn merge(self, config: Config) -> AppConfig {
        AppConfig {
            port: config.port.or(self.port).unwrap_or(3000),
            ip: config
                .ip
                .or(self.ip)
                .unwrap_or_else(|| IpAddr::from([0, 0, 0, 0])),
            bangs_url: config
                .bangs_url
                .or(self.bangs_url)
                .unwrap_or_else(|| "https://duckduckgo.com/bang.js".to_string()),
            default_search: config
                .default_search
                .or(self.default_search)
                .unwrap_or_else(|| DEFAULT_SEARCH.to_string()),
            search_suggestions: config
                .search_suggestions
                .or(self.search_suggestions)
                .unwrap_or_else(|| DEFAULT_SEARCH_SUGGESTIONS.to_string()),
            bangs: self.bangs,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            ip: IpAddr::from([0, 0, 0, 0]),
            bangs_url: "https://duckduckgo.com/bang.js".to_string(),
            default_search: DEFAULT_SEARCH.to_string(),
            search_suggestions: DEFAULT_SEARCH_SUGGESTIONS.to_string(),
            bangs: None,
        }
    }
}

impl From<Cli> for Config {
    fn from(cli: Cli) -> Self {
        match cli.command {
            Some(SubCommand::Serve { port, ip }) => Self {
                port,
                ip,
                bangs_url: cli.bangs_url,
                default_search: cli.default_search,
                search_suggestions: cli.search_suggestions,
            },
            Some(SubCommand::Resolve { query: _ }) => Self {
                port: None,
                ip: None,
                bangs_url: cli.bangs_url,
                default_search: cli.default_search,
                search_suggestions: cli.search_suggestions,
            },
            _ => Self::default(),
        }
    }
}
