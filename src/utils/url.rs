/// Normalize URL by adding the appropriate scheme if missing
pub fn normalize_url_scheme(url_str: &str) -> String {
    let trimmed_url = url_str.trim();
    if trimmed_url.starts_with("http://") || trimmed_url.starts_with("https://") {
        return trimmed_url.to_string();
    }

    if let Some(colon_pos) = trimmed_url.rfind(':')
        && let Some(port_str) = trimmed_url.get(colon_pos + 1..)
    {
        // Ensure what follows ':' is a valid port number and not part of the path
        if !port_str.is_empty() && port_str.chars().all(char::is_numeric) {
            if port_str == "80" {
                return format!("http://{}", trimmed_url);
            }
            // For 443 and all other ports, use https.
            return format!("https://{}", trimmed_url);
        }
    }

    // No port or invalid port format, default to https
    format!("https://{}", trimmed_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_url_with_scheme() {
        assert_eq!(
            normalize_url_scheme("http://example.com"),
            "http://example.com"
        );
        assert_eq!(
            normalize_url_scheme("https://example.com"),
            "https://example.com"
        );
    }

    #[test]
    fn test_normalize_url_with_port_80() {
        assert_eq!(
            normalize_url_scheme("example.com:80"),
            "http://example.com:80"
        );
    }

    #[test]
    fn test_normalize_url_with_port_443() {
        assert_eq!(
            normalize_url_scheme("example.com:443"),
            "https://example.com:443"
        );
    }

    #[test]
    fn test_normalize_url_with_custom_port() {
        assert_eq!(
            normalize_url_scheme("example.com:8080"),
            "https://example.com:8080"
        );
    }

    #[test]
    fn test_normalize_url_without_port() {
        assert_eq!(normalize_url_scheme("example.com"), "https://example.com");
    }
}
