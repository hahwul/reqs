use colored::*;
use reqwest::StatusCode;
use std::time::Duration;

/// Response information for formatting
pub struct ResponseInfo<'a> {
    pub method: &'a str,
    pub url: &'a str,
    pub ip_addr: &'a str,
    pub status: StatusCode,
    pub size: u64,
    pub elapsed: Duration,
    pub title: &'a Option<String>,
}

/// Format response as plain text output
pub fn format_plain_output(
    response: &ResponseInfo,
    template: &Option<String>,
    colored: bool,
) -> String {
    if let Some(template_str) = template {
        let time_str = format!("{:?}", response.elapsed);
        let mut output = template_str
            .replace("%method", response.method)
            .replace("%url", response.url)
            .replace("%status", &response.status.to_string())
            .replace("%code", &response.status.as_u16().to_string())
            .replace("%size", &response.size.to_string())
            .replace("%time", &time_str)
            .replace("%ip", response.ip_addr)
            .replace("%title", &response.title.clone().unwrap_or_default());
        output.push('\n');
        output
    } else {
        let title_str = if let Some(t) = response.title {
            if colored {
                format!(" | Title: {}", t.blue())
            } else {
                format!(" | Title: {}", t)
            }
        } else {
            String::new()
        };

        if colored {
            let status_str = response.status.to_string();
            let colored_status = if response.status.is_success() {
                status_str.green()
            } else if response.status.is_redirection() {
                status_str.yellow()
            } else {
                status_str.red()
            };
            format!(
                "[{}] [{}] [{}] -> {} | Size: {} {}| Time: {:?}\n",
                response.method.yellow(),
                response.url.cyan(),
                response.ip_addr.magenta(),
                colored_status,
                response.size.to_string().blue(),
                title_str,
                response.elapsed
            )
        } else {
            format!(
                "[{}] [{}] [{}] -> {} | Size: {} {}| Time: {:?}\n",
                response.method,
                response.url,
                response.ip_addr,
                response.status,
                response.size,
                title_str,
                response.elapsed
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_plain_output_no_template() {
        let response = ResponseInfo {
            method: "GET",
            url: "https://example.com",
            ip_addr: "1.2.3.4",
            status: StatusCode::OK,
            size: 1234,
            elapsed: Duration::from_secs(1),
            title: &None,
        };
        let output = format_plain_output(&response, &None, false);
        assert!(output.contains("GET"));
        assert!(output.contains("https://example.com"));
        assert!(output.contains("200 OK"));
    }

    #[test]
    fn test_format_plain_output_with_template() {
        let response = ResponseInfo {
            method: "GET",
            url: "https://example.com",
            ip_addr: "1.2.3.4",
            status: StatusCode::OK,
            size: 1234,
            elapsed: Duration::from_secs(1),
            title: &None,
        };
        let template = Some("%method %url -> %code".to_string());
        let output = format_plain_output(&response, &template, false);
        assert_eq!(output, "GET https://example.com -> 200\n");
    }
}
