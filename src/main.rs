mod bang;
mod config;

use crate::bang::Bang;
use crate::config::{AppConfig, Config, FileConfig};
use axum::extract::State;
use axum::response::Html;
use axum::{Router, extract::Query, response::Redirect, routing::get};
use clap::Parser;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use std::{
    collections::HashMap,
    env,
    net::SocketAddr,
    path::Path,
    sync::RwLock,
    time::{Duration, Instant},
};
use tokio::net::TcpListener;
use tokio::time::interval;
use tracing::{Level, debug, error, info};

#[derive(Debug, Deserialize)]
struct SearchParams {
    #[serde(rename = "q")]
    query: Option<String>,
}

static BANG_CACHE: Lazy<RwLock<HashMap<String, String>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));
static LAST_UPDATE: Lazy<RwLock<Instant>> = Lazy::new(|| RwLock::new(Instant::now()));
static BANG_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)(!\S+)").unwrap());

async fn update_bangs(app_config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::get(&app_config.bangs_url).await?.text().await?;
    let bang_entries: Vec<Bang> = serde_json::from_str(&response)?;
    match BANG_CACHE.write() {
        Ok(mut cache) => {
            cache.clear();
            for bang in bang_entries {
                cache.insert(bang.trigger.clone(), bang.url_template.clone());
            }
            if let Some(bangs) = &app_config.bangs {
                for bang in bangs {
                    cache.insert(bang.trigger.clone(), bang.url_template.clone());
                }
            }
            *LAST_UPDATE.write().unwrap() = Instant::now();
            info!("Bang commands updated successfully.");
            Ok(())
        }
        Err(err) => {
            error!("Failed to acquire bang cache write lock.");
            Err(format!("Failed to acquire bang cache write lock: {}", err).into())
        }
    }
}

async fn periodic_update(app_config: AppConfig) {
    let mut interval = interval(Duration::from_secs(24 * 60 * 60)); // 24 hours
    loop {
        interval.tick().await;
        if let Err(e) = update_bangs(&app_config).await {
            error!("Failed to update bang commands: {}", e);
        }
    }
}

/// Handler function that extracts the `q` parameter and redirects accordingly
async fn handler(
    Query(params): Query<SearchParams>,
    State(app_config): State<AppConfig>,
) -> Redirect {
    let start = Instant::now();
    if let Some(query) = params.query {
        if let Some(captures) = BANG_REGEX.captures(&query) {
            if let Some(matched) = captures.get(1) {
                let bang = matched.as_str();
                let search_term = query.replacen(bang, "", 1);
                if let Ok(cache) = BANG_CACHE.read() {
                    if let Some(url_template) =
                        cache.get(bang.trim().strip_prefix("!").unwrap_or(bang))
                    {
                        let redirect_url = url_template
                            .replace("{{{s}}}", &urlencoding::encode(search_term.trim()))
                            .replace("%2F", "/");
                        info!("Redirecting '{}' to '{}'.", query, redirect_url);
                        debug!(
                            "Request completed in {:?}",
                            Instant::now().duration_since(start)
                        );

                        return Redirect::to(&redirect_url);
                    }
                } else {
                    error!("Failed to acquire bang cache read lock.");
                }
            }
        }
        let default_search_url = app_config
            .default_search
            .replace("{}", &urlencoding::encode(&query));
        info!("Redirecting '{}' to '{}'.", query, default_search_url);
        debug!(
            "Request completed in {:?}",
            Instant::now().duration_since(start)
        );

        Redirect::to(&default_search_url)
    } else {
        Redirect::to("/bangs")
    }
}

async fn list_bangs(State(app_config): State<AppConfig>) -> Html<String> {
    if let Ok(cache) = BANG_CACHE.read() {
        let mut html = String::from(
            "<html><head><title>Bang Commands</title></head><body><h1>Bang Commands</h1>",
        );

        if let Some(bangs) = &app_config.bangs {
            html.push_str("<h2>Configured Bangs</h2><ul>");
            for bang in bangs {
                html.push_str(&format!(
                    "<li><strong>{}</strong>: {}</li>",
                    bang.trigger, bang.url_template
                ));
            }
            html.push_str("</ul>");
        }

        html.push_str("<h2>Active Bangs</h2><ul>");
        for (trigger, url_template) in cache.iter() {
            html.push_str(&format!(
                "<li><strong>{}</strong>: {}</li>",
                trigger, url_template
            ));
        }
        html.push_str("</ul></body></html>");
        Html(html)
    } else {
        Html("<html><head><title>Error</title></head><body><h1>Error</h1><p>Failed to acquire bang cache read lock.</p></body></html>".to_string())
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
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

    let app_config = cli_config.merge(file_config);

    tokio::spawn(periodic_update(app_config.clone()));

    let app = Router::new()
        .route("/", get(handler))
        .route("/bangs", get(list_bangs))
        .with_state(app_config.clone());
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
