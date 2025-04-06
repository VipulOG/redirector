pub mod bang;
pub mod cli;
pub mod config;

use crate::bang::Bang;
use crate::config::AppConfig;
use anyhow::anyhow;
use memchr::memmem::find;
use regex::Regex;
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};
use std::time::{Duration, Instant};
use tracing::{debug, error, info};

static BANG_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(!\S+)").unwrap());
pub static BANG_CACHE: LazyLock<RwLock<HashMap<String, String>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static LAST_UPDATE: LazyLock<RwLock<Instant>> = LazyLock::new(|| RwLock::new(Instant::now()));

/// Get the bang command from the query.
/// this is the first '!' that is not preceded by a non-space character and followed by a space.
#[inline]
pub fn get_bang(query: &str) -> Option<&str> {
    let indices = query.match_indices('!');
    for (i, _) in indices {
        if query.as_bytes().get(i + 1).is_some()
            && query.as_bytes()[i + 1] != b' '
            && ((i > 0 && query.as_bytes()[i - 1] == b' ') || i == 0)
        {
            if let Some(bang) = query[i..].split_whitespace().next() {
                if bang.len() > 1 {
                    return Some(bang);
                }
            }
        }
    }
    None
}

#[allow(clippy::inline_always)]
#[inline(always)]
pub fn resolve(app_config: &AppConfig, query: &str) -> String {
    // let mut start = Instant::now();
    if let Some(bang) = get_bang(query) {
        // info!("bang search took {:?}", start.elapsed());
        // start = Instant::now();
        let search_term = query.replacen(bang, "", 1);
        // info!("search term took {:?}", start.elapsed());
        // start = Instant::now();
        if let Ok(cache) = BANG_CACHE.read() {
            // info!("cache read took {:?}", start.elapsed());
            // start = Instant::now();
            let key_lower = bang[1..].to_ascii_lowercase().to_owned();
            if let Some(mut url_template) = cache.get(&key_lower).cloned() {
                // info!("cache get took {:?}", start.elapsed());
                // start = Instant::now();
                return if find(url_template.as_bytes(), b"{{{s}}}").is_none() {
                    // info!("find took {:?}", start.elapsed());
                    url_template.push_str(&urlencoding::encode(search_term.trim()));
                    url_template.replace("%2F", "/")
                } else {
                    // info!("find took {:?}", start.elapsed());
                    url_template
                        .replace("{{{s}}}", &urlencoding::encode(search_term.trim()))
                        .replace("%2F", "/")
                };
            }
        } else {
            error!("Failed to acquire bang cache read lock.");
        }
    }
    // info!("default search took {:?}", start.elapsed());
    app_config
        .default_search
        .replace("{}", &urlencoding::encode(query))
}

/// Update the bang cache with the latest bang commands.
///
/// # Errors
/// If it fails to update the bang cache.
pub fn update_bangs(app_config: &AppConfig) -> anyhow::Result<()> {
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

    let response = reqwest::blocking::get(&app_config.bangs_url)?.text()?;
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
