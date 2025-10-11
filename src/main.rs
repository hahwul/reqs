use anyhow::Result;
use colored::*;
use clap::Parser;
use futures::stream::{self, StreamExt};
use reqwest::{Client, redirect::Policy, header::{HeaderMap, HeaderName, HeaderValue}};
use serde_json::json;
use std::io::{self, BufRead};
use std::time::{Duration, Instant};
use tokio::task;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::fs::File;
use rand::Rng;
use std::sync::Arc;
use tokio::sync::Mutex;
use regex::Regex;

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
        long_help = "Custom format string for plain output (e.g. \"%method %url -> %code\").\nPlaceholders: %method, %url, %status, %code, %size, %time"
    )]
    strf: Option<String>,

    /// Include request details in the output.
    #[arg(long, help_heading = "OUTPUT")]
    include_req: bool,

    /// Include response body in the output.
    #[arg(long, help_heading = "OUTPUT")]
    include_res: bool,

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
}

fn normalize_url_scheme(url_str: &str) -> String {
    let trimmed_url = url_str.trim();
    if trimmed_url.starts_with("http://") || trimmed_url.starts_with("https://") {
        return trimmed_url.to_string();
    }

    if let Some(pos) = trimmed_url.rfind(':') {
        if let Some(port_str) = trimmed_url.get(pos + 1..) {
            // Ensure what follows ':' is a valid port number and not part of the path
            if !port_str.is_empty() && port_str.chars().all(char::is_numeric) {
                if port_str == "80" {
                    return format!("http://{}", trimmed_url);
                }
                // For 443 and all other ports, use https.
                return format!("https://{}", trimmed_url);
            }
        }
    }

    // No port or invalid port format, default to https
    format!("https://{}", trimmed_url)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let parsed_filter_regex: Arc<Option<Regex>> = Arc::new(if let Some(regex_str) = &cli.filter_regex {
        match Regex::new(regex_str) {
            Ok(re) => Some(re),
            Err(e) => {
                eprintln!("[Warning] Invalid regex provided for --filter-regex: {}. Disabling regex filtering.", e);
                None
            }
        }
    } else {
        None
    });

    let redirect_policy = if cli.follow_redirect {
        Policy::limited(10) // Default reqwest behavior for following redirects
    } else {
        Policy::none()
    };

    let mut default_headers = HeaderMap::new();
    for header_str in &cli.headers {
        if let Some((key, value)) = header_str.split_once(": ") {
            if let Ok(header_name) = HeaderName::from_bytes(key.as_bytes()) {
                if let Ok(header_value) = HeaderValue::from_str(value.trim()) {
                    default_headers.insert(header_name, header_value);
                } else {
                    eprintln!("[Warning] Invalid header value for key '{}'", key);
                }
            } else {
                eprintln!("[Warning] Invalid header name: {}", key);
            }
        } else {
            eprintln!("[Warning] Invalid header format. Expected 'Key: Value'. Got: {}", header_str);
        }
    }

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

    let output_writer: Option<Arc<Mutex<BufWriter<File>>>> = if let Some(output_path) = &cli.output {
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
        .filter_map(Result::ok)
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

                if let Some(random_delay_str) = &cli.random_delay {
                    let parts: Vec<&str> = random_delay_str.split(':').collect();
                    if parts.len() == 2 {
                        if let (Ok(min_delay), Ok(max_delay)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>()) {
                            if max_delay >= min_delay {
                                let delay = {
                                    let mut rng = rand::thread_rng();
                                    rng.gen_range(min_delay..=max_delay)
                                };
                                tokio::time::sleep(Duration::from_millis(delay)).await;
                            } else {
                                eprintln!("[Warning] Invalid --random-delay format: MAX must be greater than or equal to MIN. Got: {}", random_delay_str);
                            }
                        } else {
                            eprintln!("[Warning] Invalid --random-delay format: Could not parse min/max values. Got: {}", random_delay_str);
                        }
                    } else {
                        eprintln!("[Warning] Invalid --random-delay format. Expected MIN:MAX. Got: {}", random_delay_str);
                    }
                }

                if let Some(rate_limit) = cli.rate_limit {
                    let mut last_req_guard = last_request_time.lock().await;
                    let elapsed = last_req_guard.elapsed();
                    let min_delay_micros = 1_000_000 / rate_limit; // microseconds per request
                    if elapsed.as_micros() < min_delay_micros as u128 {
                        let sleep_duration = Duration::from_micros(min_delay_micros - elapsed.as_micros() as u64);
                        tokio::time::sleep(sleep_duration).await;
                    }
                    *last_req_guard = Instant::now();
                }

                let parts: Vec<&str> = url.trim().split_whitespace().collect();
                
                let (method, url_str, body): (String, String, Option<String>) = if parts.is_empty() {
                    return;
                } else if parts.len() > 1 && ["GET", "POST", "PUT", "DELETE", "HEAD", "PATCH", "OPTIONS"].contains(&parts[0].to_uppercase().as_str()) {
                    let method = parts[0].to_uppercase();
                    let url = parts[1].to_string();
                    let body = if parts.len() > 2 {
                        Some(parts[2..].join(" ").to_string())
                    } else {
                        None
                    };
                    (method, url, body)
                } else {
                    ("GET".to_string(), url, None) // url is the original String
                };

                let url_str = normalize_url_scheme(&url_str);

                let mut attempts = 0;
                let mut last_error = None;

                while attempts <= cli.retry {
                    if attempts > 0 && cli.delay > 0 {
                        tokio::time::sleep(Duration::from_millis(cli.delay)).await;
                    }

                    let mut request_builder = match method.as_str() {
                        "POST" => client.post(&url_str),
                        "PUT" => client.put(&url_str),
                        "DELETE" => client.delete(&url_str),
                        "HEAD" => client.head(&url_str),
                        "PATCH" => client.patch(&url_str),
                        "OPTIONS" => client.request(reqwest::Method::OPTIONS, &url_str),
                        _ => client.get(&url_str),
                    };

                    if let Some(body_content) = &body {
                        request_builder = request_builder.body(body_content.clone());
                    }
                    
                    let req_for_display = if cli.include_req {
                        match request_builder.try_clone().unwrap().build() {
                            Ok(req) => {
                                let method = req.method();
                                let url = req.url();
                                let path_and_query = if let Some(query) = url.query() {
                                    format!("{}?{}", url.path(), query)
                                } else {
                                    url.path().to_string()
                                };
                                let version = if cli.http2 { "HTTP/2.0" } else { "HTTP/1.1" };
                                let mut raw_req = format!("{} {} {}\n", method, path_and_query, version);
                                raw_req.push_str(&format!("Host: {}\n", url.host_str().unwrap_or("")));

                                // Create a temporary HeaderMap for display to handle overrides correctly
                                let mut display_headers = HeaderMap::new();

                                // Add headers from the request itself (e.g. Accept, Content-Type set by reqwest)
                                for (name, value) in req.headers() {
                                    display_headers.insert(name.clone(), value.clone());
                                }

                                // Add/overwrite with custom headers from the CLI for display
                                for header_str in &cli.headers {
                                    if let Some((key, value)) = header_str.split_once(": ") {
                                        if let Ok(name) = HeaderName::from_bytes(key.as_bytes()) {
                                            if let Ok(val) = HeaderValue::from_str(value.trim()) {
                                                display_headers.insert(name, val);
                                            }
                                        }
                                    }
                                }

                                // Now print the combined headers
                                for (name, value) in &display_headers {
                                    raw_req.push_str(&format!("{}: {}\n", name, value.to_str().unwrap_or("[unprintable]")));
                                }

                                if let Some(body) = req.body().and_then(|b| b.as_bytes()) {
                                    if !body.is_empty() {
                                        raw_req.push_str(&format!("\n{}", String::from_utf8_lossy(body)));
                                    }
                                }
                                Some(raw_req)
                            },
                            Err(_) => None,
                        }
                    } else {
                        None
                    };

                    let start_time = Instant::now();
                    match request_builder.send().await {
                        Ok(resp) => {
                            let elapsed = start_time.elapsed();
                            let status = resp.status();
                            let size = resp.content_length().unwrap_or(0);
                            
                            let body_text = if cli.include_res || cli.filter_string.is_some() || cli.filter_regex.is_some() {
                                Some(resp.text().await.unwrap_or_default())
                            } else {
                                None
                            };

                            let mut should_output = true;

                            // Filter by status codes
                            if !cli.filter_status.is_empty() && !cli.filter_status.contains(&status.as_u16()) {
                                should_output = false;
                            }

                            // Filter by string in response body
                            if should_output { // Only check if still eligible
                                if let Some(filter_str) = &cli.filter_string {
                                    if let Some(body) = &body_text {
                                        if !body.contains(filter_str) {
                                            should_output = false;
                                        }
                                    } else {
                                        // If body is not included but filter_string is set, don't output
                                        should_output = false;
                                    }
                                }
                            }

                            // Filter by regex in response body
                            if should_output { // Only check if still eligible
                                if let Some(re) = parsed_filter_regex.as_ref() {
                                    if let Some(body) = &body_text {
                                        if !re.is_match(body) {
                                            should_output = false;
                                        }
                                    } else {
                                        // If body is not included but filter_regex is set, don't output
                                        should_output = false;
                                    }
                                }
                            }

                            if !should_output {
                                return; // Skip output if it doesn't pass filters
                            }

                            if let OutputFormat::Csv = cli.format {
                                let mut header_written = csv_header_written.lock().await;
                                if !*header_written {
                                    let csv_header = "method,url,status_code,content_length,response_time_ms\n".to_string();
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
                                            .replace("%time", &time_str);
                                        s.push('\n');
                                    } else {
                                        if cli.output.is_none() && !cli.no_color {
                                            let status_str = status.to_string();
                                            let colored_status = if status.is_success() {
                                                status_str.green()
                                            } else if status.is_redirection() {
                                                status_str.yellow()
                                            } else {
                                                status_str.red()
                                            };
                                            s.push_str(&format!("[{}] [{}] -> {} | Size: {} | Time: {:?}\n",
                                                method.yellow(),
                                                url_str.cyan(),
                                                colored_status,
                                                size.to_string().blue(),
                                                elapsed
                                            ));
                                        } else {
                                            s.push_str(&format!("[{}] [{}] -> {} | Size: {} | Time: {:?}\n",
                                                method,
                                                url_str,
                                                status,
                                                size,
                                                elapsed
                                            ));
                                        }
                                    }
                                    if let Some(raw_req) = req_for_display {
                                        s.push_str(&format!("[Raw Request]\n{}\n", raw_req));
                                    }
                                    if let Some(body) = body_text {
                                        s.push_str(&format!("[Response Body]\n{}\n", body));
                                    }
                                    s
                                },
                                OutputFormat::Jsonl => {
                                    let mut json_output = json!({
                                        "method": method,
                                        "url": url_str,
                                        "status_code": status.as_u16(),
                                        "content_length": size,
                                        "response_time_ms": elapsed.as_millis(),
                                    });
                                    if let Some(req) = req_for_display {
                                        json_output["raw_request"] = req.into();
                                    }
                                    if let Some(body) = body_text {
                                        json_output["response_body"] = body.into();
                                    }
                                    serde_json::to_string(&json_output).unwrap_or_default() + "\n"
                                },
                                OutputFormat::Csv => {
                                    let time_str = format!("{:?}", elapsed);
                                    format!("\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
                                        method,
                                        url_str,
                                        status.as_u16(),
                                        size,
                                        time_str
                                    )
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