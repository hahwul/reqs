use anyhow::Result;
use clap::Parser;
use colored::*;
use futures::stream::{self, StreamExt};
use rand::Rng;
use regex::Regex;
use reqwest::{
    Client,
    header::{HeaderMap, HeaderName, HeaderValue},
    redirect::Policy,
};
use scraper::{Html, Selector};
use serde_json::json;
use std::io::{self, BufRead};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::Mutex;
use tokio::task;

// Constants
const DEFAULT_REDIRECT_LIMIT: usize = 10;
const HTTP_VERSION_2: &str = "HTTP/2.0";
const HTTP_VERSION_1_1: &str = "HTTP/1.1";
const TITLE_SELECTOR: &str = "title";
const MICROSECONDS_PER_SECOND: u64 = 1_000_000;

// MCP imports (only used when --mcp flag is set)
use async_trait::async_trait;
use rust_mcp_sdk::McpServer;
use rust_mcp_sdk::mcp_server::{ServerHandler, ServerRuntime, server_runtime};
use rust_mcp_sdk::schema::schema_utils::CallToolError;
use rust_mcp_sdk::schema::{
    CallToolRequest, CallToolResult, Implementation, InitializeResult, LATEST_PROTOCOL_VERSION,
    ListToolsRequest, ListToolsResult, RpcError, ServerCapabilities, ServerCapabilitiesTools,
    TextContent, Tool,
};
use rust_mcp_sdk::{StdioTransport, TransportOptions};

#[derive(clap::ValueEnum, Debug, Clone, Default)]
enum OutputFormat {
    #[default]
    Plain,
    Jsonl,
    Csv,
}

/// A simple and fast command-line tool to test URLs from a pipeline.
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Cli {
    // NETWORK
    /// Timeout for each request in seconds.
    #[arg(long, default_value_t = 10, help_heading = "NETWORK")]
    timeout: u64,

    /// Number of retries for failed requests.
    #[arg(long, default_value_t = 0, help_heading = "NETWORK")]
    retry: u32,

    /// Delay between retries in milliseconds.
    #[arg(long, default_value_t = 0, help_heading = "NETWORK")]
    delay: u64,

    /// Maximum number of concurrent requests (0 for unlimited).
    #[arg(long, default_value_t = 0, help_heading = "NETWORK")]
    concurrency: usize,

    /// Use a proxy for requests (e.g., "http://127.0.0.1:8080").
    #[arg(long, help_heading = "NETWORK")]
    proxy: Option<String>,

    /// Verify SSL certificates (default: false, insecure).
    #[arg(long, default_value_t = false, help_heading = "NETWORK")]
    verify_ssl: bool,

    /// Limit requests per second. E.g., --rate-limit 100.
    #[arg(long, help_heading = "NETWORK")]
    rate_limit: Option<u64>,

    /// Random delay between requests in milliseconds. E.g., --random-delay 100:500.
    #[arg(long, help_heading = "NETWORK")]
    random_delay: Option<String>,

    // HTTP
    /// Whether to follow HTTP redirects.
    #[arg(long, default_value_t = true, help_heading = "HTTP")]
    follow_redirect: bool,

    /// Use HTTP/2 for requests.
    #[arg(long, help_heading = "HTTP")]
    http2: bool,

    /// Custom headers to add to the request (e.g., "User-Agent: my-app").
    #[arg(short = 'H', long, help_heading = "HTTP")]
    headers: Vec<String>,

    // OUTPUT
    /// Output file to save results (instead of stdout).
    #[arg(short, long, help_heading = "OUTPUT")]
    output: Option<String>,

    /// Output format.
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Plain, help_heading = "OUTPUT")]
    format: OutputFormat,

    #[arg(
        short = 'S',
        long,
        help_heading = "OUTPUT",
        long_help = "Custom format string for plain output (e.g. \"%method %url -> %code\").\nPlaceholders: %method, %url, %status, %code, %size, %time, %ip, %title"
    )]
    strf: Option<String>,

    /// Include request details in the output.
    #[arg(long, help_heading = "OUTPUT")]
    include_req: bool,

    /// Include response body in the output.
    #[arg(long, help_heading = "OUTPUT")]
    include_res: bool,

    /// Include title from response body in the output.
    #[arg(long, help_heading = "OUTPUT")]
    include_title: bool,

    /// Disable color output.
    #[arg(long, help_heading = "OUTPUT")]
    no_color: bool,

    // FILTER
    /// Filter by specific HTTP status codes (e.g., "200,404").
    #[arg(long, value_delimiter = ',', help_heading = "FILTER")]
    filter_status: Vec<u16>,

    /// Filter by string in response body.
    #[arg(long, help_heading = "FILTER")]
    filter_string: Option<String>,

    /// Filter by regex in response body.
    #[arg(long, help_heading = "FILTER")]
    filter_regex: Option<String>,

    // MCP
    /// Run in MCP (Model Context Protocol) server mode.
    #[arg(long, help_heading = "MCP")]
    mcp: bool,
}

fn normalize_url_scheme(url_str: &str) -> String {
    let trimmed_url = url_str.trim();
    if trimmed_url.starts_with("http://") || trimmed_url.starts_with("https://") {
        return trimmed_url.to_string();
    }

    if let Some(pos) = trimmed_url.rfind(':')
        && let Some(port_str) = trimmed_url.get(pos + 1..)
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

async fn apply_random_delay(random_delay_str: &Option<String>) {
    if let Some(delay_str) = random_delay_str {
        let parts: Vec<&str> = delay_str.split(':').collect();
        if parts.len() == 2 {
            if let (Ok(min_delay), Ok(max_delay)) =
                (parts[0].parse::<u64>(), parts[1].parse::<u64>())
            {
                if max_delay >= min_delay {
                    let delay = {
                        let mut rng = rand::thread_rng();
                        rng.gen_range(min_delay..=max_delay)
                    };
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                } else {
                    eprintln!(
                        "[Warning] Invalid --random-delay format: MAX must be greater than or equal to MIN. Got: {}",
                        delay_str
                    );
                }
            } else {
                eprintln!(
                    "[Warning] Invalid --random-delay format: Could not parse min/max values. Got: {}",
                    delay_str
                );
            }
        } else {
            eprintln!(
                "[Warning] Invalid --random-delay format. Expected MIN:MAX. Got: {}",
                delay_str
            );
        }
    }
}

async fn apply_rate_limit(rate_limit: Option<u64>, last_request_time: &Arc<Mutex<Instant>>) {
    if let Some(rate_limit) = rate_limit {
        let mut last_req_guard = last_request_time.lock().await;
        let elapsed = last_req_guard.elapsed();
        let min_delay_micros = MICROSECONDS_PER_SECOND / rate_limit;
        if elapsed.as_micros() < min_delay_micros as u128 {
            let sleep_duration =
                Duration::from_micros(min_delay_micros - elapsed.as_micros() as u64);
            tokio::time::sleep(sleep_duration).await;
        }
        *last_req_guard = Instant::now();
    }
}

fn extract_title(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(TITLE_SELECTOR).ok()?;
    document.select(&selector).next().map(|t| t.inner_html())
}

fn should_filter_response(
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
    if let Some(filter_str) = filter_string {
        if let Some(body_text) = body {
            if !body_text.contains(filter_str) {
                return true;
            }
        } else {
            return true;
        }
    }

    // Filter by regex in response body
    if let Some(re) = filter_regex {
        if let Some(body_text) = body {
            if !re.is_match(body_text) {
                return true;
            }
        } else {
            return true;
        }
    }

    false
}

fn parse_headers(headers: &[String]) -> HeaderMap {
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

fn parse_request_line(line: &str) -> (String, String, Option<String>) {
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.is_empty() {
        return ("GET".to_string(), String::new(), None);
    }

    if parts.len() > 1
        && ["GET", "POST", "PUT", "DELETE", "HEAD", "PATCH", "OPTIONS"]
            .contains(&parts[0].to_uppercase().as_str())
    {
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

fn build_request(
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

fn format_raw_request(
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
    let mut display_headers = HeaderMap::new();

    // Add headers from the request itself
    for (name, value) in req.headers() {
        display_headers.insert(name.clone(), value.clone());
    }

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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // If --mcp flag is set, run in MCP server mode
    if cli.mcp {
        return run_mcp_server(cli).await;
    }

    let parsed_filter_regex: Arc<Option<Regex>> = Arc::new(
        if let Some(regex_str) = &cli.filter_regex {
            match Regex::new(regex_str) {
                Ok(re) => Some(re),
                Err(e) => {
                    eprintln!(
                        "[Warning] Invalid regex provided for --filter-regex: {}. Disabling regex filtering.",
                        e
                    );
                    None
                }
            }
        } else {
            None
        },
    );

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

    let client = client_builder.build()?;

    let last_request_time = Arc::new(Mutex::new(Instant::now()));

    let output_writer: Option<Arc<Mutex<BufWriter<File>>>> = if let Some(output_path) = &cli.output
    {
        let file = File::create(output_path).await?;
        Some(Arc::new(Mutex::new(BufWriter::new(file))))
    } else {
        None
    };

    let csv_header_written = Arc::new(Mutex::new(false));

    let stdin = io::stdin();
    let handles = stdin
        .lock()
        .lines()
        .map_while(Result::ok)
        .map(|url| {
            let client = client.clone();
            let cli = cli.clone(); // Clone cli for each task
            let output_writer = output_writer.clone(); // Clone output_writer for each task
            let last_request_time = last_request_time.clone(); // Clone for rate limiting
            let parsed_filter_regex = parsed_filter_regex.clone(); // Clone for regex filtering
            let csv_header_written = csv_header_written.clone();
            task::spawn(async move {
                if url.trim().is_empty() {
                    return;
                }

                apply_random_delay(&cli.random_delay).await;

                apply_rate_limit(cli.rate_limit, &last_request_time).await;

                let (method, url_str, body) = parse_request_line(&url);

                if url_str.is_empty() {
                    return;
                }

                let url_str = normalize_url_scheme(&url_str);

                let mut attempts = 0;
                let mut last_error = None;

                while attempts <= cli.retry {
                    if attempts > 0 && cli.delay > 0 {
                        tokio::time::sleep(Duration::from_millis(cli.delay)).await;
                    }

                    let request_builder = build_request(&client, &method, &url_str, &body);

                    let req_for_display = if cli.include_req {
                        request_builder
                            .try_clone()
                            .unwrap()
                            .build()
                            .ok()
                            .map(|req| format_raw_request(&req, cli.http2, Some(&cli.headers)))
                    } else {
                        None
                    };

                    let start_time = Instant::now();
                    match request_builder.send().await {
                        Ok(resp) => {
                            let elapsed = start_time.elapsed();
                            let status = resp.status();
                            let size = resp.content_length().unwrap_or(0);
                            let ip_addr = resp.remote_addr().map(|s| s.ip().to_string()).unwrap_or_default();

                            let body_text = if cli.include_res || cli.filter_string.is_some() || cli.filter_regex.is_some() || cli.include_title {
                                Some(resp.text().await.unwrap_or_default())
                            } else {
                                None
                            };

                            let title = if cli.include_title {
                                body_text.as_ref().and_then(|body| extract_title(body))
                            } else {
                                None
                            };

                            if should_filter_response(
                                status.as_u16(),
                                &body_text,
                                &cli.filter_status,
                                &cli.filter_string,
                                parsed_filter_regex.as_ref(),
                            ) {
                                return; // Skip output if it doesn't pass filters
                            }

                            if let OutputFormat::Csv = cli.format {
                                let mut header_written = csv_header_written.lock().await;
                                if !*header_written {
                                    let mut csv_header = "method,url,ip_address,status_code,content_length,response_time_ms".to_string();
                                    if cli.include_title {
                                        csv_header.push_str(",title");
                                    }
                                    csv_header.push('\n');

                                    if let Some(writer) = &output_writer {
                                        let mut writer = writer.lock().await;
                                        if let Err(e) = writer.write_all(csv_header.as_bytes()).await {
                                            eprintln!("Error writing to output file: {}", e);
                                        }
                                    } else {
                                        print!("{}", csv_header);
                                    }
                                    *header_written = true;
                                }
                            }

                            let output_str = match cli.format {
                                OutputFormat::Plain => {
                                    let mut s = String::new();
                                    if let Some(template) = &cli.strf {
                                        let time_str = format!("{:?}", elapsed);
                                        s = template
                                            .replace("%method", &method)
                                            .replace("%url", &url_str)
                                            .replace("%status", &status.to_string())
                                            .replace("%code", &status.as_u16().to_string())
                                            .replace("%size", &size.to_string())
                                            .replace("%time", &time_str)
                                            .replace("%ip", &ip_addr)
                                            .replace("%title", &title.clone().unwrap_or_default());
                                        s.push('\n');
                                    } else {
                                        let title_str = if let Some(t) = &title {
                                            format!(" | Title: {}", t.blue())
                                        } else {
                                            "".to_string()
                                        };

                                        if cli.output.is_none() && !cli.no_color {
                                            let status_str = status.to_string();
                                            let colored_status = if status.is_success() {
                                                status_str.green()
                                            } else if status.is_redirection() {
                                                status_str.yellow()
                                            } else {
                                                status_str.red()
                                            };
                                            s.push_str(&format!("[{}] [{}] [{}] -> {} | Size: {}{}| Time: {:?}\n",
                                                method.yellow(),
                                                url_str.cyan(),
                                                ip_addr.magenta(),
                                                colored_status,
                                                size.to_string().blue(),
                                                title_str,
                                                elapsed
                                            ));
                                        } else {
                                            s.push_str(&format!("[{}] [{}] [{}] -> {} | Size: {}{}| Time: {:?}\n",
                                                method,
                                                url_str,
                                                ip_addr,
                                                status,
                                                size,
                                                title_str,
                                                elapsed
                                            ));
                                        }
                                    }
                                    if let Some(raw_req) = req_for_display {
                                        s.push_str(&format!("[Raw Request]\n{}\n", raw_req));
                                    }
                                    if cli.include_res
                                        && let Some(body) = body_text {
                                            s.push_str(&format!("[Response Body]\n{}\n", body));
                                        }
                                    s
                                },
                                OutputFormat::Jsonl => {
                                    let mut json_output = json!({
                                        "method": method,
                                        "url": url_str,
                                        "ip_address": ip_addr,
                                        "status_code": status.as_u16(),
                                        "content_length": size,
                                        "response_time_ms": elapsed.as_millis(),
                                    });
                                    if let Some(t) = title {
                                        json_output["title"] = t.into();
                                    }
                                    if let Some(req) = req_for_display {
                                        json_output["raw_request"] = req.into();
                                    }
                                    if cli.include_res
                                        && let Some(body) = body_text {
                                            json_output["response_body"] = body.into();
                                        }
                                    serde_json::to_string(&json_output).unwrap_or_default() + "\n"
                                },
                                OutputFormat::Csv => {
                                    let time_str = format!("{:?}", elapsed);
                                    let mut csv_line = format!("\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"",
                                        method,
                                        url_str,
                                        ip_addr,
                                        status.as_u16(),
                                        size,
                                        time_str
                                    );
                                    if cli.include_title {
                                        csv_line.push_str(&format!(",\"{}\"", title.unwrap_or_default()));
                                    }
                                    csv_line.push('\n');
                                    csv_line
                                }
                            };

                            if let Some(writer) = output_writer {
                                let mut writer = writer.lock().await;
                                if let Err(e) = writer.write_all(output_str.as_bytes()).await {
                                    eprintln!("Error writing to output file: {}", e);
                                }
                            } else {
                                print!("{}", output_str);
                            }
                            return; // Success, exit retry loop
                        }
                        Err(err) => {
                            last_error = Some(err);
                            attempts += 1;
                            if attempts <= cli.retry {
                                eprintln!("[{}] - Attempt {} failed: {}. Retrying...", url_str, attempts, last_error.as_ref().unwrap());
                            }
                        }
                    }
                }

                if let Some(err) = last_error {
                    eprintln!("[{}] - Error after {} attempts: {}", url_str, cli.retry + 1, err);
                }
            })
        })
        .collect::<Vec<_>>();

    let concurrency_limit = if cli.concurrency == 0 {
        None
    } else {
        Some(cli.concurrency)
    };

    stream::iter(handles)
        .for_each_concurrent(concurrency_limit, |h| async {
            h.await.unwrap();
        })
        .await;

    // Ensure all buffered output is written to file before exiting
    if let Some(writer) = output_writer {
        let mut writer = writer.lock().await;
        writer.flush().await?;
    }

    Ok(())
}

async fn run_mcp_server(cli: Cli) -> Result<()> {
    // Define server details and capabilities
    let server_details = InitializeResult {
        server_info: Implementation {
            name: "reqs".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            title: Some("HTTP Request Testing Tool".to_string()),
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            ..Default::default()
        },
        meta: None,
        instructions: Some("Send HTTP requests and return response metadata.".to_string()),
        protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
    };

    // Create stdio transport
    let transport = StdioTransport::new(TransportOptions::default())
        .map_err(|e| anyhow::anyhow!("Failed to create stdio transport: {}", e))?;

    // Create handler
    let handler = ReqsServerHandler { cli: cli.clone() };

    // Create and start server
    let server: Arc<ServerRuntime> =
        server_runtime::create_server(server_details, transport, handler);
    server
        .start()
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    Ok(())
}

// Custom handler for our MCP server
struct ReqsServerHandler {
    cli: Cli,
}

#[async_trait]
impl ServerHandler for ReqsServerHandler {
    async fn handle_list_tools_request(
        &self,
        _request: ListToolsRequest,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<ListToolsResult, RpcError> {
        use std::collections::HashMap;

        // Create input schema properties
        let mut properties = HashMap::new();

        // requests parameter
        let mut requests_prop = serde_json::Map::new();
        requests_prop.insert("type".to_string(), json!("array"));
        requests_prop.insert("description".to_string(), json!("List of HTTP requests. Each request can be a simple URL or a string with METHOD URL BODY format (e.g., 'POST https://example.com data=value')"));
        let mut items = serde_json::Map::new();
        items.insert("type".to_string(), json!("string"));
        requests_prop.insert("items".to_string(), json!(items));
        properties.insert("requests".to_string(), requests_prop);

        // filter_status parameter
        let mut filter_status_prop = serde_json::Map::new();
        filter_status_prop.insert("type".to_string(), json!("array"));
        filter_status_prop.insert("description".to_string(), json!("Filter results by HTTP status codes (e.g., [200, 404]). Only responses with these status codes will be returned."));
        let mut status_items = serde_json::Map::new();
        status_items.insert("type".to_string(), json!("number"));
        filter_status_prop.insert("items".to_string(), json!(status_items));
        properties.insert("filter_status".to_string(), filter_status_prop);

        // filter_string parameter
        let mut filter_string_prop = serde_json::Map::new();
        filter_string_prop.insert("type".to_string(), json!("string"));
        filter_string_prop.insert("description".to_string(), json!("Filter results by string match in response body. Only responses containing this string will be returned."));
        properties.insert("filter_string".to_string(), filter_string_prop);

        // filter_regex parameter
        let mut filter_regex_prop = serde_json::Map::new();
        filter_regex_prop.insert("type".to_string(), json!("string"));
        filter_regex_prop.insert("description".to_string(), json!("Filter results by regex pattern in response body. Only responses matching this pattern will be returned."));
        properties.insert("filter_regex".to_string(), filter_regex_prop);

        // include_req parameter
        let mut include_req_prop = serde_json::Map::new();
        include_req_prop.insert("type".to_string(), json!("boolean"));
        include_req_prop.insert(
            "description".to_string(),
            json!("Include raw HTTP request details in the output."),
        );
        properties.insert("include_req".to_string(), include_req_prop);

        // include_res parameter
        let mut include_res_prop = serde_json::Map::new();
        include_res_prop.insert("type".to_string(), json!("boolean"));
        include_res_prop.insert(
            "description".to_string(),
            json!("Include response body in the output."),
        );
        properties.insert("include_res".to_string(), include_res_prop);

        // follow_redirect parameter
        let mut follow_redirect_prop = serde_json::Map::new();
        follow_redirect_prop.insert("type".to_string(), json!("boolean"));
        follow_redirect_prop.insert(
            "description".to_string(),
            json!("Whether to follow HTTP redirects. Defaults to true."),
        );
        properties.insert("follow_redirect".to_string(), follow_redirect_prop);

        // http2 parameter
        let mut http2_prop = serde_json::Map::new();
        http2_prop.insert("type".to_string(), json!("boolean"));
        http2_prop.insert(
            "description".to_string(),
            json!("Use HTTP/2 for requests. Defaults to false (HTTP/1.1)."),
        );
        properties.insert("http2".to_string(), http2_prop);

        // headers parameter
        let mut headers_prop = serde_json::Map::new();
        headers_prop.insert("type".to_string(), json!("array"));
        headers_prop.insert("description".to_string(), json!("Custom headers to add to the request (e.g., [\"User-Agent: my-app\", \"Authorization: Bearer token\"])"));
        let mut headers_items = serde_json::Map::new();
        headers_items.insert("type".to_string(), json!("string"));
        headers_prop.insert("items".to_string(), json!(headers_items));
        properties.insert("headers".to_string(), headers_prop);

        let input_schema = rust_mcp_sdk::schema::ToolInputSchema::new(
            vec!["requests".to_string()],
            Some(properties),
        );

        Ok(ListToolsResult {
            tools: vec![Tool {
                name: "send_requests".to_string(),
                description: Some("Send HTTP requests and return response metadata. Accepts a list of requests with optional filters (filter_status, filter_string, filter_regex), HTTP options (follow_redirect, http2, headers), and output options (include_req, include_res) for LLM analysis.".to_string()),
                input_schema,
                annotations: None,
                meta: None,
                output_schema: None,
                title: Some("Send HTTP Requests".to_string()),
            }],
            meta: None,
            next_cursor: None,
        })
    }

    async fn handle_call_tool_request(
        &self,
        request: CallToolRequest,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        if request.tool_name() != "send_requests" {
            return Err(CallToolError::unknown_tool(format!(
                "Unknown tool: {}",
                request.tool_name()
            )));
        }

        let args = request.params.arguments.as_ref().ok_or_else(|| {
            CallToolError::new(
                RpcError::invalid_params().with_message("Missing arguments".to_string()),
            )
        })?;

        let requests = args
            .get("requests")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                CallToolError::new(
                    RpcError::invalid_params()
                        .with_message("requests parameter must be an array".to_string()),
                )
            })?;

        // Extract filter parameters
        let filter_status: Vec<u16> = args
            .get("filter_status")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_u64().map(|n| n as u16))
                    .collect()
            })
            .unwrap_or_default();

        let filter_string = args
            .get("filter_string")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let filter_regex_str = args
            .get("filter_regex")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let include_req = args
            .get("include_req")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let include_res = args
            .get("include_res")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Extract HTTP configuration parameters
        let follow_redirect = args
            .get("follow_redirect")
            .and_then(|v| v.as_bool())
            .unwrap_or(self.cli.follow_redirect);

        let http2 = args
            .get("http2")
            .and_then(|v| v.as_bool())
            .unwrap_or(self.cli.http2);

        let custom_headers: Vec<String> = args
            .get("headers")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Compile regex if provided
        let filter_regex = if let Some(regex_str) = &filter_regex_str {
            match Regex::new(regex_str) {
                Ok(re) => Some(re),
                Err(e) => {
                    return Err(CallToolError::new(RpcError::invalid_params().with_message(
                        format!("Invalid regex provided for filter_regex: {}", e),
                    )));
                }
            }
        } else {
            None
        };

        let mut results = Vec::new();

        // Create HTTP client using the parameters from the tool call (with CLI defaults as fallback)
        let redirect_policy = if follow_redirect {
            Policy::limited(DEFAULT_REDIRECT_LIMIT)
        } else {
            Policy::none()
        };

        // First, apply headers from CLI (global default), then custom headers from tool call (overrides)
        let mut default_headers = parse_headers(&self.cli.headers);
        default_headers.extend(parse_headers(&custom_headers));

        let mut client_builder = Client::builder()
            .timeout(Duration::from_secs(self.cli.timeout))
            .redirect(redirect_policy)
            .default_headers(default_headers);

        if !self.cli.verify_ssl {
            client_builder = client_builder.danger_accept_invalid_certs(true);
        }

        if let Some(proxy_url) = &self.cli.proxy {
            let proxy = reqwest::Proxy::all(proxy_url).map_err(|e| {
                CallToolError::new(
                    RpcError::internal_error()
                        .with_message(format!("Failed to create proxy: {}", e)),
                )
            })?;
            client_builder = client_builder.proxy(proxy);
        }

        if !http2 {
            client_builder = client_builder.http1_only();
        }

        let client = client_builder.build().map_err(|e| {
            CallToolError::new(
                RpcError::internal_error()
                    .with_message(format!("Failed to build HTTP client: {}", e)),
            )
        })?;

        // Process each request
        for req in requests {
            let req_str = req
                .as_str()
                .ok_or_else(|| {
                    CallToolError::new(
                        RpcError::invalid_params()
                            .with_message("Each request must be a string".to_string()),
                    )
                })?
                .trim();

            if req_str.is_empty() {
                continue;
            }

            let (method, url_str, body) = parse_request_line(req_str);

            if url_str.is_empty() {
                continue;
            }

            let url_str = normalize_url_scheme(&url_str);

            let request_builder = build_request(&client, &method, &url_str, &body);

            // Capture raw request if needed
            let raw_request = if include_req {
                request_builder
                    .try_clone()
                    .unwrap()
                    .build()
                    .ok()
                    .map(|req| format_raw_request(&req, http2, None))
            } else {
                None
            };

            let start_time = Instant::now();
            match request_builder.send().await {
                Ok(resp) => {
                    let elapsed = start_time.elapsed();
                    let status = resp.status();
                    let size = resp.content_length().unwrap_or(0);
                    let ip_addr = resp
                        .remote_addr()
                        .map(|s| s.ip().to_string())
                        .unwrap_or_default();

                    // Fetch response body if needed for filtering or output
                    let body_text =
                        if include_res || filter_string.is_some() || filter_regex.is_some() {
                            Some(resp.text().await.unwrap_or_default())
                        } else {
                            None
                        };

                    if should_filter_response(
                        status.as_u16(),
                        &body_text,
                        &filter_status,
                        &filter_string,
                        &filter_regex,
                    ) {
                        continue; // Skip this result
                    }

                    let mut result = json!({
                        "method": method,
                        "url": url_str,
                        "status_code": status.as_u16(),
                        "content_length": size,
                        "response_time_ms": elapsed.as_millis(),
                    });

                    if !ip_addr.is_empty() {
                        result["ip_address"] = ip_addr.into();
                    }

                    if let Some(raw_req) = raw_request {
                        result["raw_request"] = raw_req.into();
                    }

                    if include_res && let Some(body) = body_text {
                        result["response_body"] = body.into();
                    }

                    results.push(result);
                }
                Err(err) => {
                    results.push(json!({
                        "method": method,
                        "url": url_str,
                        "error": err.to_string(),
                    }));
                }
            }
        }

        // Return results as tool response
        let result_text = results
            .iter()
            .map(|r| serde_json::to_string(r).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(CallToolResult::text_content(vec![TextContent::from(
            result_text,
        )]))
    }
}
