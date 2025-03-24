mod bang;
mod cli;
mod config;

use axum::extract::State;
use axum::response::Html;
use axum::{Router, extract::Query, response::Redirect, routing::get};
use clap::{CommandFactory, Parser};
use clap_complete::generate;
use redirector::cli::SubCommand::Completions;
use redirector::cli::{Cli, SubCommand};
use redirector::config::{AppConfig, FileConfig};
use redirector::{BANG_CACHE, resolve, update_bangs};
use serde::Deserialize;
use std::fs::read_to_string;
use std::{
    env,
    net::SocketAddr,
    path::Path,
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
    params.query.map_or_else(
        || Redirect::to("/bangs"),
        |query| {
            let start = Instant::now();
            let redirect_url = resolve(&app_config, &query);
            info!("Redirecting '{}' to '{}'.", query, redirect_url);
            debug!("Request completed in {:?}", start.elapsed());
            Redirect::to(&redirect_url)
        },
    )
}

async fn list_bangs(State(app_config): State<AppConfig>) -> Html<String> {
    if let Ok(cache) = BANG_CACHE.read() {
        let mut html = String::from(
            "<style>:root { background: #181818; color: #ffffff; font-family: monospace; } table { border-collapse: collapse; width: 100vw; } table th { text-align: left; padding: 1rem 0; font-size: 1.25rem; width: 100vw; } table tr { border-bottom: #ffffff10 solid 2px; } table tr:nth-child(2n) { background: #161616; } table tr:nth-child(2n+1) { background: #181818; }</style><html><head><title>Bang Commands</title></head><body><h1>Bang Commands</h1>",
        );

        if let Some(bangs) = &app_config.bangs {
            html.push_str(
                "<h2>Configured Bangs</h2><table><th>Abbr.</th><th>Trigger</th><th>URL</th>",
            );
            for bang in bangs {
                html.push_str(&format!(
                    "<tr><td><strong>{:?}</strong></td><td>{}</td><td>{}</td></tr>",
                    bang.short_name, bang.trigger, bang.url_template
                ));
            }
            html.push_str("</table>");
        }

        html.push_str("<h2>Active Bangs</h2><table><th>Trigger</th><th>URL</th>");
        for (trigger, url_template) in cache.iter() {
            html.push_str(&format!(
                "<tr><td><strong>{trigger}</strong></td><td>{url_template}</td></tr>"
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
    let cli_config = Cli::parse();

    let log_level = match &cli_config.command {
        Some(SubCommand::Serve { .. }) | None => Level::DEBUG,
        _ => Level::INFO,
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_writer(std::io::stderr)
        .init();

    let home_dir = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let config_path = Path::new(&home_dir)
        .join(".config")
        .join("redirector")
        .join("config.toml");

    // Attempt to load the file configuration if it exists.
    let file_config = if config_path.exists() {
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
    };

    let app_config = file_config
        .unwrap_or_default()
        .merge(cli_config.clone().into());

    match cli_config.command {
        Some(SubCommand::Serve { .. }) | None => {
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
        Some(SubCommand::Resolve { query }) => {
            if let Err(e) = update_bangs(&app_config).await {
                error!("Failed to update bang commands: {}", e);
            }
            println!("{}", resolve(&app_config, &query));
        }
        Some(Completions { shell }) => {
            generate(
                shell,
                &mut Cli::command(),
                env!("CARGO_PKG_NAME"),
                &mut std::io::stdout(),
            );
        }
    }
}
