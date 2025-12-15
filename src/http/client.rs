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
