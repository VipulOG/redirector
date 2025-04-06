#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use super::*;
    use redirector::config::AppConfig;
    use std::sync::RwLockWriteGuard;
    use redirector::{get_bang, resolve, update_bangs, BANG_CACHE};

    #[test]
    fn test_get_bang() {
        // Valid bang queries
        assert_eq!(get_bang("!gh search term"), Some("!gh"));
        assert_eq!(get_bang("search !gh term"), Some("!gh"));
        assert_eq!(get_bang("!gh"), Some("!gh"));
        assert_eq!(get_bang("!multi-word"), Some("!multi-word"));
        assert_eq!(get_bang("  !gh search"), Some("!gh"));
        assert_eq!(get_bang("!g rust programming"), Some("!g"));

        // Invalid bang queries
        assert_eq!(get_bang("search!gh term"), None); // No space before !
        assert_eq!(get_bang("search! gh term"), None); // Space after !
        assert_eq!(get_bang("!"), None); // Single ! is not a bang
        assert_eq!(get_bang(""), None); // Empty string
        assert_eq!(get_bang("no bang here"), None); // No bang
        assert_eq!(get_bang("a!!gh"), None); // No space before !
    }

    #[test]
    fn test_resolve_with_bang() {
        let config = AppConfig::default();
        update_bangs(&config).unwrap();

        // Test with template that has {{{s}}}
        let result = resolve(&config, "!g rust programming");
        assert_eq!(result, "https://www.google.com/search?q=rust%20programming");

        // Test with template that doesn't have {{{s}}}
        let result = resolve(&config, "!gh rust programming");
        assert_eq!(result, "https://github.com/search?utf8=%E2%9C%93&q=rust%20programming");

        // Test with bang at different position
        let result = resolve(&config, "rust !yt programming");
        assert_eq!(result, "https://www.youtube.com/results?search_query=rust%20%20programming");
    }

    #[test]
    fn test_resolve_without_bang() {
        let config = AppConfig::default();

        update_bangs(&config).unwrap();

        // Test with no bang
        let result = resolve(&config, "rust programming");
        assert_eq!(result, config.default_search.replace("{}", "rust%20programming"));

        // Test with non-matching bang
        let result = resolve(&config, "!nonexistent rust programming");
        assert_eq!(result, config.default_search.replace("{}", "%21nonexistent%20rust%20programming"));
    }

    #[test]
    fn test_resolve_edge_cases() {
        let config = AppConfig::default();

        update_bangs(&config).unwrap();

        // Empty query
        let result = resolve(&config, "");
        assert_eq!(result, config.default_search.replace("{}", ""));

        // URL encoding special chars
        let result = resolve(&config, "!g c++ & rust/wasm");
        assert_eq!(result, "https://www.google.com/search?q=c%2B%2B%20%26%20rust/wasm");

        // Only a bang with no search term
        let result = resolve(&config, "!g");
        assert_eq!(result, "https://www.google.com/search?q=");
    }
}