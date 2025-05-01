use clap::{Parser, Subcommand};
use clap_complete::Shell;
use std::net::IpAddr;

/// Main CLI configuration.
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Option<SubCommand>,

    /// URL to fetch bang commands from
    #[arg(short, long)]
    pub bangs_url: Option<String>,

    /// Default search engine URL template (use '{}' as placeholder for the query)
    #[arg(short, long)]
    pub default_search: Option<String>,

    /// Search suggestions URL template (use '{}' as placeholder for the query)
    #[arg(short, long)]
    pub search_suggestions: Option<String>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum SubCommand {
    #[command(about = "Start the redirecting server", display_order = 1)]
    Serve {
        /// Port to listen on
        #[arg(short, long)]
        port: Option<u16>,

        /// IP to serve the application on
        #[arg(short, long)]
        ip: Option<IpAddr>,
    },
    #[command(about = "Resolve a search query", display_order = 2)]
    Resolve {
        /// The search query to resolve
        #[arg(required = true)]
        query: String,
    },
    #[command(about = "Generate shell completions", display_order = 3)]
    Completions {
        #[clap(value_enum)]
        shell: Shell,
    },
}
