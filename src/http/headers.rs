use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

/// Parse header strings into HeaderMap
pub fn parse_headers(headers: &[String]) -> HeaderMap {
    let mut header_map = HeaderMap::new();
    for header_str in headers {
        if let Some((key, value)) = header_str.split_once(": ") {
            if let Ok(header_name) = HeaderName::from_bytes(key.as_bytes()) {
                if let Ok(header_value) = HeaderValue::from_str(value.trim()) {
                    header_map.insert(header_name, header_value);
                } else {
                    eprintln!("[Warning] Invalid header value for key '{}'", key);
                }
            } else {
                eprintln!("[Warning] Invalid header name: {}", key);
            }
        } else {
            eprintln!(
                "[Warning] Invalid header format. Expected 'Key: Value'. Got: {}",
                header_str
            );
        }
    }
    header_map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_headers() {
        let headers = vec![
            "User-Agent: test-agent".to_string(),
            "Content-Type: application/json".to_string(),
        ];
        let header_map = parse_headers(&headers);
        assert_eq!(header_map.len(), 2);
        assert_eq!(header_map.get("User-Agent").unwrap(), "test-agent");
        assert_eq!(header_map.get("Content-Type").unwrap(), "application/json");
    }

    #[test]
    fn test_parse_headers_invalid() {
        let headers = vec!["Invalid Header".to_string()];
        let header_map = parse_headers(&headers);
        assert_eq!(header_map.len(), 0);
    }
}
