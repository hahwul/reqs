use anyhow::Result;
use clap::Parser;

mod constants;
mod filter;
mod http;
mod mcp;
mod output;
mod processor;
mod types;
mod utils;

use http::build_http_client;
use mcp::run_mcp_server;
use processor::process_urls_from_stdin;
use types::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // If --mcp flag is set, run in MCP server mode
    if cli.mcp {
        return run_mcp_server(cli).await;
    }

    // Build HTTP client from CLI configuration
    let client = build_http_client(&cli)?;

    // Process URLs from stdin
    process_urls_from_stdin(cli, client).await
}
