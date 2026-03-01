use scraper::{Html, Selector};
use std::sync::LazyLock;

use crate::constants::TITLE_SELECTOR;

static TITLE_SEL: LazyLock<Selector> = LazyLock::new(|| Selector::parse(TITLE_SELECTOR).unwrap());

/// Extract title from HTML content
pub fn extract_title(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    document.select(&TITLE_SEL).next().map(|t| t.inner_html())
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
