use axum::{Router, extract::Query, response::Redirect, routing::get};
use clap::Parser;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::RwLock,
    time::{Duration, Instant},
};
use tokio::net::TcpListener;
use tokio::time::interval;
use tracing::{error, info};

pub type Bangs = Vec<Bang>;

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

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Config {
    /// Port to listen on
    #[arg(short, long, default_value_t = 3000)]
    port: u16,

    /// URL to serve the application on
    #[arg(short, long, default_value = "0.0.0.0")]
    ip: IpAddr,

    /// URL to fetch bang commands from
    #[arg(short, long, default_value = "https://duckduckgo.com/bang.js")]
    bangs: String,
}

static BANG_CACHE: Lazy<RwLock<HashMap<String, String>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));
static LAST_UPDATE: Lazy<RwLock<Instant>> = Lazy::new(|| RwLock::new(Instant::now()));
static BANG_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r".*(!\w+\s?|\s?!\w+).*").unwrap());

async fn update_bangs(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?.text().await?;

    let bang_entries: Bangs = serde_json::from_str(&response)?;

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
                let search_term = query.replace(bang, "");
                let cache = BANG_CACHE.read().unwrap();
                if let Some(url_template) = cache.get(bang.trim().strip_prefix("!").unwrap_or("")) {
                    let redirect_url =
                        url_template.replace("{{{s}}}", &urlencoding::encode(search_term.as_str()));
                    info!("Redirecting '{}' to '{}'.", query, redirect_url);
                    return Redirect::to(&redirect_url);
                }
            }
        }
        info!("Redirecting '{}' to '{}'.", query, "standard search");
        let default_search_url = format!(
            "https://google.com/search?q={}",
            urlencoding::encode(&query)
        );
        Redirect::to(&default_search_url)
    } else {
        Redirect::to("https://google.com/")
    }
}

#[tokio::main]
async fn main() {
    let config = Config::parse();

    tracing_subscriber::fmt::init();

    if let Err(e) = update_bangs(&config.bangs).await {
        error!("Initial bang update failed: {}", e);
    }

    tokio::spawn(periodic_update(config.bangs.clone()));

    let app = Router::new().route("/", get(handler));
    let addr = SocketAddr::from((config.ip, config.port));
    let listener = match TcpListener::bind(addr).await {
        Result::Ok(listener) => listener,
        Err(e) => {
            error!("Failed to bind to address: {}", e);
            return;
        }
    };
    info!("Server running on {}", addr);
    axum::serve(listener, app).await.unwrap();
}
