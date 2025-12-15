use anyhow::Result;
use futures::stream::{self, StreamExt};
use regex::Regex;
use reqwest::Client;
use serde_json::json;
use std::io::{self, BufRead};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::Mutex;
use tokio::task;

use crate::filter::should_filter_response;
use crate::http::{build_request, format_raw_request, parse_request_line};
use crate::output::{ResponseInfo, format_plain_output};
use crate::types::{Cli, OutputFormat};
use crate::utils::{apply_random_delay, apply_rate_limit, extract_title, normalize_url_scheme};

/// Context for request processing
struct ProcessingContext {
    output_writer: Option<Arc<Mutex<BufWriter<File>>>>,
    parsed_filter_regex: Arc<Option<Regex>>,
    csv_header_written: Arc<Mutex<bool>>,
}

/// Process URLs from stdin and send HTTP requests
pub async fn process_urls_from_stdin(cli: Cli, client: Client) -> Result<()> {
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

    let last_request_time = Arc::new(Mutex::new(Instant::now()));

    let output_writer: Option<Arc<Mutex<BufWriter<File>>>> = if let Some(output_path) = &cli.output
    {
        let file = File::create(output_path).await?;
        Some(Arc::new(Mutex::new(BufWriter::new(file))))
    } else {
        None
    };

    let context = Arc::new(ProcessingContext {
        output_writer: output_writer.clone(),
        parsed_filter_regex,
        csv_header_written: Arc::new(Mutex::new(false)),
    });

    let stdin = io::stdin();
    let handles = stdin
        .lock()
        .lines()
        .map_while(Result::ok)
        .map(|url| {
            let client = client.clone();
            let cli = cli.clone();
            let last_request_time = last_request_time.clone();
            let context = context.clone();
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

                process_single_request(&client, &cli, &method, &url_str, &body, &context).await;
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
    if let Some(writer) = &context.output_writer {
        let mut writer = writer.lock().await;
        writer.flush().await?;
    }

    Ok(())
}

/// Process a single HTTP request with retries
async fn process_single_request(
    client: &Client,
    cli: &Cli,
    method: &str,
    url_str: &str,
    body: &Option<String>,
    context: &ProcessingContext,
) {
    let mut attempts = 0;
    let mut last_error = None;

    while attempts <= cli.retry {
        if attempts > 0 && cli.delay > 0 {
            tokio::time::sleep(Duration::from_millis(cli.delay)).await;
        }

        let request_builder = build_request(client, method, url_str, body);

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
                let ip_addr = resp
                    .remote_addr()
                    .map(|s| s.ip().to_string())
                    .unwrap_or_default();

                let body_text = if cli.include_res
                    || cli.filter_string.is_some()
                    || cli.filter_regex.is_some()
                    || cli.include_title
                {
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
                    context.parsed_filter_regex.as_ref(),
                ) {
                    return; // Skip output if it doesn't pass filters
                }

                // Write CSV header if needed
                if let OutputFormat::Csv = cli.format {
                    write_csv_header(cli, &context.output_writer, &context.csv_header_written)
                        .await;
                }

                let response_data = ResponseData {
                    method,
                    url_str,
                    ip_addr: &ip_addr,
                    status,
                    size,
                    elapsed,
                    title: &title,
                    req_for_display: &req_for_display,
                    body_text: &body_text,
                };
                let output_str = format_response_output(cli, &response_data);

                write_output(output_str, &context.output_writer).await;
                return; // Success, exit retry loop
            }
            Err(err) => {
                last_error = Some(err);
                attempts += 1;
                if attempts <= cli.retry {
                    eprintln!(
                        "[{}] - Attempt {} failed: {}. Retrying...",
                        url_str,
                        attempts,
                        last_error.as_ref().unwrap()
                    );
                }
            }
        }
    }

    if let Some(err) = last_error {
        eprintln!(
            "[{}] - Error after {} attempts: {}",
            url_str,
            cli.retry + 1,
            err
        );
    }
}

/// Write CSV header if not yet written
async fn write_csv_header(
    cli: &Cli,
    output_writer: &Option<Arc<Mutex<BufWriter<File>>>>,
    csv_header_written: &Arc<Mutex<bool>>,
) {
    let mut header_written = csv_header_written.lock().await;
    if !*header_written {
        let mut csv_header =
            "method,url,ip_address,status_code,content_length,response_time_ms".to_string();
        if cli.include_title {
            csv_header.push_str(",title");
        }
        csv_header.push('\n');

        if let Some(writer) = output_writer {
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

/// Response data for formatting
struct ResponseData<'a> {
    method: &'a str,
    url_str: &'a str,
    ip_addr: &'a str,
    status: reqwest::StatusCode,
    size: u64,
    elapsed: Duration,
    title: &'a Option<String>,
    req_for_display: &'a Option<String>,
    body_text: &'a Option<String>,
}

/// Format response output
fn format_response_output(cli: &Cli, data: &ResponseData) -> String {
    match cli.format {
        OutputFormat::Plain => {
            let response_info = ResponseInfo {
                method: data.method,
                url: data.url_str,
                ip_addr: data.ip_addr,
                status: data.status,
                size: data.size,
                elapsed: data.elapsed,
                title: data.title,
            };
            let mut s = format_plain_output(
                &response_info,
                &cli.strf,
                cli.output.is_none() && !cli.no_color,
            );
            if let Some(raw_req) = data.req_for_display {
                s.push_str(&format!("[Raw Request]\n{}\n", raw_req));
            }
            if cli.include_res
                && let Some(body) = data.body_text
            {
                s.push_str(&format!("[Response Body]\n{}\n", body));
            }
            s
        }
        OutputFormat::Jsonl => {
            let mut json_output = json!({
                "method": data.method,
                "url": data.url_str,
                "ip_address": data.ip_addr,
                "status_code": data.status.as_u16(),
                "content_length": data.size,
                "response_time_ms": data.elapsed.as_millis(),
            });
            if let Some(t) = data.title {
                json_output["title"] = t.clone().into();
            }
            if let Some(req) = data.req_for_display {
                json_output["raw_request"] = req.clone().into();
            }
            if cli.include_res
                && let Some(body) = data.body_text
            {
                json_output["response_body"] = body.clone().into();
            }
            serde_json::to_string(&json_output).unwrap_or_default() + "\n"
        }
        OutputFormat::Csv => {
            let time_str = format!("{:?}", data.elapsed);
            let mut csv_line = format!(
                "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"",
                data.method,
                data.url_str,
                data.ip_addr,
                data.status.as_u16(),
                data.size,
                time_str
            );
            if cli.include_title {
                csv_line.push_str(&format!(",\"{}\"", data.title.clone().unwrap_or_default()));
            }
            csv_line.push('\n');
            csv_line
        }
    }
}

/// Write output to file or stdout
async fn write_output(output_str: String, output_writer: &Option<Arc<Mutex<BufWriter<File>>>>) {
    if let Some(writer) = output_writer {
        let mut writer = writer.lock().await;
        if let Err(e) = writer.write_all(output_str.as_bytes()).await {
            eprintln!("Error writing to output file: {}", e);
        }
    } else {
        print!("{}", output_str);
    }
}
