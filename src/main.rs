use axum::response::Html;
use axum::{Router, extract::Query, response::Redirect, routing::get};
use clap::Parser;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    net::{IpAddr, SocketAddr},
    path::Path,
    sync::RwLock,
    time::{Duration, Instant},
};
use tokio::net::TcpListener;
use tokio::time::interval;
use tracing::{error, info};

#[derive(Serialize, Deserialize, Debug)]
pub struct Bang {
    /// The category of the bang command (e.g., "Tech", "Entertainment").
    #[serde(rename = "c")]
    pub category: Option<Category>,
    /// The domain associated with the bang command (e.g., "www.example.com").
    #[serde(rename = "d")]
    pub domain: String,
    /// The relevance score of the bang command.
    #[serde(rename = "r")]
    pub relevance: i64,
    /// The short name or abbreviation of the bang command.
    #[serde(rename = "s")]
    pub short_name: String,
    /// The subcategory of the bang command, if applicable.
    #[serde(rename = "sc")]
    pub subcategory: Option<String>,
    /// The trigger text for the bang command (e.g., "g" for Google).
    #[serde(rename = "t")]
    pub trigger: String,
    /// The URL template where the search term is inserted.
    #[serde(rename = "u")]
    pub url_template: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub enum Category {
    Entertainment,
    Multimedia,
    News,
    #[serde(rename = "Online Services")]
    OnlineServices,
    Research,
    Shopping,
    Tech,
    Translation,
}

#[derive(Debug, Deserialize)]
struct SearchParams {
    #[serde(rename = "q")]
    query: Option<String>,
}

/// Main CLI configuration. Fields are optional so that values from a file can be merged in.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Config {
    /// Port to listen on
    #[arg(short, long)]
    port: Option<u16>,

    /// IP to serve the application on
    #[arg(short, long)]
    ip: Option<IpAddr>,

    /// URL to fetch bang commands from
    #[arg(short, long)]
    bangs: Option<String>,
}

/// Configuration read from the file.
#[derive(Deserialize, Debug)]
struct FileConfig {
    port: Option<u16>,
    ip: Option<IpAddr>,
    bangs: Option<String>,
}

/// Final application configuration.
struct AppConfig {
    port: u16,
    ip: IpAddr,
    bangs: String,
}

impl Config {
    /// Merge CLI configuration with an optional file configuration.
    /// CLI options take precedence over file values.
    fn merge(self, file: Option<FileConfig>) -> AppConfig {
        let file = file.unwrap_or(FileConfig {
            port: None,
            ip: None,
            bangs: None,
        });
        AppConfig {
            port: self.port.or(file.port).unwrap_or(3000),
            ip: self
                .ip
                .or(file.ip)
                .unwrap_or_else(|| "0.0.0.0".parse().unwrap()),
            bangs: self
                .bangs
                .or(file.bangs)
                .unwrap_or_else(|| "https://duckduckgo.com/bang.js".to_string()),
        }
    }
}

static BANG_CACHE: Lazy<RwLock<HashMap<String, String>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));
static LAST_UPDATE: Lazy<RwLock<Instant>> = Lazy::new(|| RwLock::new(Instant::now()));
static BANG_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)(!\S+)").unwrap());

async fn update_bangs(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?.text().await?;

    let bang_entries: Vec<Bang> = serde_json::from_str(&response)?;

    let mut cache = BANG_CACHE.write().unwrap();
    cache.clear();
    for bang in bang_entries {
        cache.insert(bang.trigger.clone(), bang.url_template.clone());
    }
    *LAST_UPDATE.write().unwrap() = Instant::now();
    info!("Bang commands updated successfully.");
    Ok(())
}

async fn periodic_update(url: String) {
    let mut interval = interval(Duration::from_secs(24 * 60 * 60)); // 24 hours
    loop {
        interval.tick().await;
        if let Err(e) = update_bangs(&url).await {
            error!("Failed to update bang commands: {}", e);
        }
    }
}

/// Handler function that extracts the `q` parameter and redirects accordingly
async fn handler(Query(params): Query<SearchParams>) -> Redirect {
    if let Some(query) = params.query {
        if let Some(captures) = BANG_REGEX.captures(&query) {
            if let Some(bang) = captures.get(1) {
                let bang = bang.as_str();
                let search_term = query.replacen(bang, "", 1);
                let cache = BANG_CACHE.read().unwrap();
                if let Some(url_template) = cache.get(bang.trim().strip_prefix("!").unwrap_or(bang))
                {
                    let redirect_url = url_template
                        .replace("{{{s}}}", &urlencoding::encode(search_term.trim()))
                        .replace("%2F", "/");
                    info!("Redirecting '{}' to '{}'.", query, redirect_url);
                    return Redirect::to(&redirect_url);
                }
            }
        }
        let default_search_url = format!(
            "https://www.startpage.com/do/dsearch?query={}",
            urlencoding::encode(&query)
        );
        info!("Redirecting '{}' to '{}'.", query, default_search_url);
        Redirect::to(&default_search_url)
    } else {
        Redirect::to("/bangs")
    }
}

async fn list_bangs() -> Html<String> {
    let cache = BANG_CACHE.read().unwrap();
    let mut html = String::from(
        "<html><head><title>Bang Commands</title></head><body><h1>Bang Commands</h1><ul>",
    );
    for (trigger, url_template) in cache.iter() {
        html.push_str(&format!(
            "<li><strong>{}</strong>: {}</li>",
            trigger, url_template
        ));
    }
    html.push_str("</ul></body></html>");
    Html(html)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let cli_config = Config::parse();

    let home_dir = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let config_path = Path::new(&home_dir)
        .join(".config")
        .join("redirector")
        .join("config.toml");

    // Attempt to load the file configuration if it exists.
    let file_config = if config_path.exists() {
        match std::fs::read_to_string(&config_path) {
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
        info!("Configuration file not found at {}.", config_path.display());
        None
    };

    // Merge CLI config with file config.
    let app_config = cli_config.merge(file_config);

    tokio::spawn(periodic_update(app_config.bangs.clone()));

    let app = Router::new()
        .route("/", get(handler))
        .route("/bangs", get(list_bangs));
    let addr = SocketAddr::new(app_config.ip, app_config.port);
    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(e) => {
            error!("Failed to bind to address '{}': {}", addr, e);
            return;
        }
    };
    info!("Server running on '{}'", addr);
    axum::serve(listener, app).await.unwrap();
}
