use anyhow::Result;
use reqwest::{Client, redirect::Policy};
use std::time::Duration;

use crate::constants::DEFAULT_REDIRECT_LIMIT;
use crate::http::headers::parse_headers;
use crate::types::Cli;

/// Build HTTP client from CLI configuration
pub fn build_http_client(cli: &Cli) -> Result<Client> {
    let redirect_policy = if cli.follow_redirect {
        Policy::limited(DEFAULT_REDIRECT_LIMIT)
    } else {
        Policy::none()
    };

    let default_headers = parse_headers(&cli.headers);

    let mut client_builder = Client::builder()
        .timeout(Duration::from_secs(cli.timeout))
        .redirect(redirect_policy)
        .default_headers(default_headers);

    // Disable SSL verification by default
    if !cli.verify_ssl {
        client_builder = client_builder.danger_accept_invalid_certs(true);
    }

    if let Some(proxy_url) = &cli.proxy {
        let proxy = reqwest::Proxy::all(proxy_url)?;
        client_builder = client_builder.proxy(proxy);
    }

    if !cli.http2 {
        client_builder = client_builder.http1_only();
    }

    Ok(client_builder.build()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_build_http_client_default() {
        let cli = Cli::parse_from(&["reqs"]);
        let client = build_http_client(&cli);
        assert!(client.is_ok(), "Should build a client with default settings");
    }

    #[test]
    fn test_build_http_client_with_custom_headers() {
        let cli = Cli::parse_from(&["reqs", "-H", "User-Agent: test-agent"]);
        let client = build_http_client(&cli);
        assert!(client.is_ok(), "Should build a client with custom headers");
    }

    #[test]
    fn test_build_http_client_with_proxy() {
        let cli = Cli::parse_from(&["reqs", "--proxy", "http://127.0.0.1:8080"]);
        let client = build_http_client(&cli);
        assert!(client.is_ok(), "Should build a client with a proxy");
    }

    #[test]
    fn test_build_http_client_with_invalid_proxy() {
        let cli = Cli::parse_from(&["reqs", "--proxy", "htt\0p://127.0.0.1:8080"]);
        let client = build_http_client(&cli);
        assert!(client.is_err(), "Should fail to build a client with an invalid proxy");
    }

    #[test]
    fn test_build_http_client_ssl_verification() {
        let cli = Cli::parse_from(&["reqs", "--verify-ssl"]);
        let client = build_http_client(&cli);
        assert!(client.is_ok(), "Should build a client with SSL verification enabled");
    }

    #[test]
    fn test_build_http_client_http2() {
        let cli = Cli::parse_from(&["reqs", "--http2"]);
        let client = build_http_client(&cli);
        assert!(client.is_ok(), "Should build a client with HTTP2 enabled");
    }
}
