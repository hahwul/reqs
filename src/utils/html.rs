use scraper::{Html, Selector};

use crate::constants::TITLE_SELECTOR;

/// Extract title from HTML content
pub fn extract_title(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(TITLE_SELECTOR).ok()?;
    document.select(&selector).next().map(|t| t.inner_html())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_title() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head><title>Test Title</title></head>
            <body><h1>Hello</h1></body>
            </html>
        "#;
        assert_eq!(extract_title(html), Some("Test Title".to_string()));
    }

    #[test]
    fn test_extract_title_no_title() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head></head>
            <body><h1>Hello</h1></body>
            </html>
        "#;
        assert_eq!(extract_title(html), None);
    }
}
