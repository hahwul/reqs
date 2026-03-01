use regex::Regex;

/// Check if response should be filtered out based on criteria
pub fn should_filter_response(
    status: u16,
    body: &Option<String>,
    filter_status: &[u16],
    filter_string: &Option<String>,
    filter_regex: &Option<Regex>,
) -> bool {
    // Filter by status codes
    if !filter_status.is_empty() && !filter_status.contains(&status) {
        return true;
    }

    // Filter by string in response body
    if let (Some(filter_str), Some(body_text)) = (filter_string, body) {
        if !body_text.contains(filter_str) {
            return true;
        }
    } else if filter_string.is_some() {
        return true;
    }

    // Filter by regex in response body
    if let (Some(re), Some(body_text)) = (filter_regex, body) {
        if !re.is_match(body_text) {
            return true;
        }
    } else if filter_regex.is_some() {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_by_status() {
        let filter_status = vec![200, 404];
        assert!(!should_filter_response(
            200,
            &None,
            &filter_status,
            &None,
            &None
        ));
        assert!(should_filter_response(
            500,
            &None,
            &filter_status,
            &None,
            &None
        ));
    }

    #[test]
    fn test_filter_by_string() {
        let body = Some("test content".to_string());
        let filter_string = Some("test".to_string());
        assert!(!should_filter_response(
            200,
            &body,
            &[],
            &filter_string,
            &None
        ));

        let filter_string = Some("missing".to_string());
        assert!(should_filter_response(
            200,
            &body,
            &[],
            &filter_string,
            &None
        ));
    }

    #[test]
    fn test_filter_by_regex() {
        let body = Some("test content".to_string());
        let filter_regex = Some(Regex::new(r"content$").unwrap());

        // Regex matches the body, so it shouldn't filter
        assert!(!should_filter_response(
            200,
            &body,
            &[],
            &None,
            &filter_regex
        ));

        let filter_regex = Some(Regex::new(r"^missing").unwrap());

        // Regex does not match the body, so it should filter
        assert!(should_filter_response(
            200,
            &body,
            &[],
            &None,
            &filter_regex
        ));

        // Regex provided but no body, so it should filter
        assert!(should_filter_response(
            200,
            &None,
            &[],
            &None,
            &filter_regex
        ));
    }

    #[test]
    fn test_no_filter() {
        assert!(!should_filter_response(200, &None, &[], &None, &None));
    }
}
