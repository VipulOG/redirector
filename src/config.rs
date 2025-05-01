use crate::bang::Bang;
use crate::cli::{Cli, SubCommand};
use crate::update_bangs;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt::Write;
use std::fs::read_to_string;
use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, error, info};

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
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct AppConfig {
    pub port: u16,
    pub ip: IpAddr,
    pub bangs_url: String,
    pub default_search: String,
    pub search_suggestions: String,
    pub bangs: Option<Vec<Bang>>,
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub config: Arc<RwLock<AppConfig>>,
}

impl AppState {
    #[must_use]
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }

    #[must_use]
    pub fn get_config(&self) -> AppConfig {
        self.config.read().clone()
    }
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

/// Reloads configuration from disk while preserving CLI options.
pub async fn reload_config(app_state: &AppState) {
    // Get new file config
    let file_config = get_file_config();

    if let Some(config) = file_config {
        let mut config_clone = {
            let current_config = app_state.config.read();
            current_config.clone()
        };

        config_clone.bangs = config.bangs;

        // Reload bang cache with the clone
        if let Err(e) = update_bangs(&config_clone).await {
            error!("Failed to update bang commands: {}", e);
            return;
        }

        {
            let mut current_config = app_state.config.write();
            *current_config = config_clone;
        }

        info!("Configuration reloaded successfully");
    } else {
        debug!("No configuration file found, nothing was changed.");
    }
}

pub fn get_file_config() -> Option<FileConfig> {
    let home_dir = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let config_path = Path::new(&home_dir)
        .join(".config")
        .join("redirector")
        .join("config.toml");

    // Attempt to load the file configuration if it exists.
    if config_path.exists() {
        match read_to_string(&config_path) {
            Ok(contents) => match toml::from_str::<FileConfig>(&contents) {
                Ok(conf) => Some(conf),
                Err(e) => {
                    error!(
                        "Failed to parse configuration file at {}: {}",
                        config_path.display(),
                        e
                    );
                    None
                }
            },
            Err(e) => {
                error!(
                    "Failed to read configuration file at {}: {}",
                    config_path.display(),
                    e
                );
                None
            }
        }
    } else {
        debug!("Configuration file not found at {}.", config_path.display());
        None
    }
}

#[allow(clippy::cognitive_complexity)]
pub fn append_file_config(bang: Bang) {
    let home_dir = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let config_path = Path::new(&home_dir)
        .join(".config")
        .join("redirector")
        .join("config.toml");

    // Attempt to load the file configuration if it exists.
    if config_path.exists() {
        match read_to_string(&config_path) {
            Ok(mut contents) => {
                // append the new bang to the config file
                // TODO: dont use unwrap
                write!(contents, "\n[[bangs]]").unwrap();
                write!(contents, "\ntrigger = \"{}\"", bang.trigger).unwrap();
                write!(contents, "\nurl_template = \"{}\"", bang.url_template).unwrap();
                if let Some(category) = bang.category {
                    write!(contents, "\ncategory = \"{category}\"").unwrap();
                }
                if let Some(domain) = bang.domain {
                    write!(contents, "\ndomain = \"{domain}\"").unwrap();
                }
                if let Some(relevance) = bang.relevance {
                    write!(contents, "\nrelevance = {relevance}").unwrap();
                }
                if let Some(short_name) = bang.short_name {
                    write!(contents, "\nshort_name = \"{short_name}\"").unwrap();
                }
                if let Some(subcategory) = bang.subcategory {
                    write!(contents, "\nsubcategory = \"{subcategory}\"").unwrap();
                }
                writeln!(contents).unwrap();

                if let Err(e) = std::fs::write(&config_path, contents) {
                    error!(
                        "Failed to write to configuration file at {}: {}",
                        config_path.display(),
                        e
                    );
                } else {
                    info!("Configuration file updated successfully.");
                }
            }
            Err(e) => {
                error!(
                    "Failed to read configuration file at {}: {}",
                    config_path.display(),
                    e
                );
            }
        }
    } else {
        debug!("Configuration file not found at {}.", config_path.display());
    }
}
