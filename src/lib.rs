pub mod bang;
pub mod cli;
pub mod config;

use crate::bang::Bang;
use crate::config::AppConfig;
use anyhow::anyhow;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use tracing::{debug, error};

static BANG_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(!\S+)").unwrap());
pub static BANG_CACHE: Lazy<RwLock<HashMap<String, String>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));
static LAST_UPDATE: Lazy<RwLock<Instant>> = Lazy::new(|| RwLock::new(Instant::now()));

#[allow(clippy::inline_always)]
#[inline]
pub fn resolve(app_config: &AppConfig, query: &str) -> String {
    if let Some(captures) = BANG_REGEX.captures(query) {
        if let Some(matched) = captures.get(1) {
            let bang = matched.as_str();
            let search_term = query.replacen(bang, "", 1);
            if let Ok(cache) = BANG_CACHE.read() {
                if let Some(mut url_template) = cache
                    .get(&bang.strip_prefix('!').unwrap_or(bang).to_lowercase())
                    .cloned()
                {
                    if !url_template.contains(r"{{{s}}}") {
                        url_template.push_str("{{{s}}}");
                    }
                    return url_template
                        .replace("{{{s}}}", &urlencoding::encode(search_term.trim()))
                        .replace("%2F", "/");
                }
            } else {
                error!("Failed to acquire bang cache read lock.");
            }
        }
    }
    app_config
        .default_search
        .replace("{}", &urlencoding::encode(query))
}

/// Update the bang cache with the latest bang commands.
///
/// # Errors
/// If it fails to update the bang cache.
pub async fn update_bangs(app_config: &AppConfig) -> anyhow::Result<()> {
    let cache_path = std::env::temp_dir().join("bang_cache.json");
    let cache_age_limit = Duration::from_secs(24 * 60 * 60);

    if let Ok(metadata) = std::fs::metadata(&cache_path) {
        if let Ok(modified) = metadata.modified() {
            if modified.elapsed()? < cache_age_limit {
                if let Ok(contents) = std::fs::read_to_string(&cache_path) {
                    let bang_entries: Vec<Bang> = serde_json::from_str(&contents)?;
                    update_cache(bang_entries, app_config)?;
                    return Ok(());
                }
            }
        }
    }

    let response = reqwest::get(&app_config.bangs_url).await?.text().await?;
    let bang_entries: Vec<Bang> = serde_json::from_str(&response)?;

    std::fs::write(cache_path, &response)?;
    update_cache(bang_entries, app_config)
}

/// Update the bang cache with the provided bang commands.
///
/// # Errors
/// If it fails to get the write lock on the bang cache or the last update time.
fn update_cache(bang_entries: Vec<Bang>, app_config: &AppConfig) -> anyhow::Result<()> {
    let mut cache = BANG_CACHE
        .write()
        .map_err(|e| anyhow!("Failed to obtain bang cache write lock: {:?}", e))?;
    cache.clear();
    for bang in bang_entries {
        cache.insert(bang.trigger.clone(), bang.url_template.clone());
    }
    if let Some(bangs) = &app_config.bangs {
        for bang in bangs {
            cache.insert(bang.trigger.clone(), bang.url_template.clone());
        }
    }
    drop(cache);
    *LAST_UPDATE
        .write()
        .map_err(|e| anyhow!("Failed to obtain last update write lock: {:?}", e))? = Instant::now();
    debug!("Bang commands updated successfully.");
    Ok(())
}
