use crate::bang::Bang;
use clap::Parser;
use serde::Deserialize;
use std::net::IpAddr;

/// Main CLI configuration.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    /// Port to listen on
    #[arg(short, long)]
    port: Option<u16>,

    /// IP to serve the application on
    #[arg(short, long)]
    ip: Option<IpAddr>,

    /// URL to fetch bang commands from
    #[arg(short, long)]
    bangs_url: Option<String>,

    /// Default search engine URL template (use '{}' as placeholder for the query)
    #[arg(short, long)]
    default_search: Option<String>,
}

/// Configuration read from the file.
#[derive(Deserialize, Debug)]
pub struct FileConfig {
    pub(crate) port: Option<u16>,
    pub(crate) ip: Option<IpAddr>,
    pub(crate) bangs_url: Option<String>,
    pub(crate) default_search: Option<String>,
    pub(crate) bangs: Option<Vec<Bang>>,
}

/// Final application configuration.
#[derive(Clone)]
pub struct AppConfig {
    pub(crate) port: u16,
    pub(crate) ip: IpAddr,
    pub(crate) bangs_url: String,
    pub(crate) default_search: String,
    pub(crate) bangs: Option<Vec<Bang>>,
}

impl Config {
    /// Merge CLI configuration with an optional file configuration.
    /// CLI options take precedence over file values.
    pub(crate) fn merge(self, file: Option<FileConfig>) -> AppConfig {
        let file = file.unwrap_or(FileConfig {
            port: None,
            ip: None,
            bangs_url: None,
            default_search: None,
            bangs: None,
        });
        AppConfig {
            port: self.port.or(file.port).unwrap_or(3000),
            ip: self
                .ip
                .or(file.ip)
                .unwrap_or_else(|| "0.0.0.0".parse().unwrap()),
            bangs_url: self
                .bangs_url
                .or(file.bangs_url)
                .unwrap_or_else(|| "https://duckduckgo.com/bang.js".to_string()),
            default_search: self
                .default_search
                .or(file.default_search)
                .unwrap_or_else(|| "https://www.startpage.com/do/dsearch?query={}".to_string()),
            bangs: file.bangs,
        }
    }
}
