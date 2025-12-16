use reqwest::Client;

use crate::constants::{HTTP_METHODS, HTTP_VERSION_1_1, HTTP_VERSION_2};
use crate::http::headers::parse_headers;

/// Parse request line to extract method, URL, and optional body
pub fn parse_request_line(line: &str) -> (String, String, Option<String>) {
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.is_empty() {
        return ("GET".to_string(), String::new(), None);
    }

    if parts.len() > 1 && HTTP_METHODS.contains(&parts[0].to_uppercase().as_str()) {
        let method = parts[0].to_uppercase();
        let url = parts[1].to_string();
        let body = if parts.len() > 2 {
            Some(parts[2..].join(" "))
        } else {
            None
        };
        (method, url, body)
    } else {
        ("GET".to_string(), line.to_string(), None)
    }
}

/// Build HTTP request from method, URL, and optional body
pub fn build_request(
    client: &Client,
    method: &str,
    url: &str,
    body: &Option<String>,
) -> reqwest::RequestBuilder {
    let mut request_builder = match method {
        "POST" => client.post(url),
        "PUT" => client.put(url),
        "DELETE" => client.delete(url),
        "HEAD" => client.head(url),
        "PATCH" => client.patch(url),
        "OPTIONS" => client.request(reqwest::Method::OPTIONS, url),
        _ => client.get(url),
    };

    if let Some(body_content) = body {
        request_builder = request_builder.body(body_content.clone());
    }

    request_builder
}

/// Format raw HTTP request for display
pub fn format_raw_request(
    req: &reqwest::Request,
    http2: bool,
    custom_headers: Option<&[String]>,
) -> String {
    let method = req.method();
    let url = req.url();
    let path_and_query = if let Some(query) = url.query() {
        format!("{}?{}", url.path(), query)
    } else {
        url.path().to_string()
    };
    let version = if http2 {
        HTTP_VERSION_2
    } else {
        HTTP_VERSION_1_1
    };
    let mut raw_req = format!("{} {} {}\n", method, path_and_query, version);
    raw_req.push_str(&format!("Host: {}\n", url.host_str().unwrap_or("")));

    // Create a temporary HeaderMap for display to handle overrides correctly
    let mut display_headers = req.headers().clone();

    // Add/overwrite with custom headers if provided
    if let Some(headers) = custom_headers {
        display_headers.extend(parse_headers(headers));
    }

    // Print the combined headers
    for (name, value) in &display_headers {
        raw_req.push_str(&format!(
            "{}: {}\n",
            name,
            value.to_str().unwrap_or("[unprintable]")
        ));
    }

    if let Some(body) = req.body().and_then(|b| b.as_bytes())
        && !body.is_empty()
    {
        raw_req.push_str(&format!("\n{}", String::from_utf8_lossy(body)));
    }

    raw_req
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_request_line_get() {
        let (method, url, body) = parse_request_line("https://example.com");
        assert_eq!(method, "GET");
        assert_eq!(url, "https://example.com");
        assert_eq!(body, None);
    }

    #[test]
    fn test_parse_request_line_post() {
        let (method, url, body) = parse_request_line("POST https://example.com data=value");
        assert_eq!(method, "POST");
        assert_eq!(url, "https://example.com");
        assert_eq!(body, Some("data=value".to_string()));
    }

    #[test]
    fn test_parse_request_line_empty() {
        let (method, url, body) = parse_request_line("");
        assert_eq!(method, "GET");
        assert_eq!(url, "");
        assert_eq!(body, None);
    }
}
