use anyhow::Result;
use clap::Parser;
use futures::stream::{self, StreamExt};
use reqwest::{Client, redirect::Policy};
use std::io::{self, BufRead};
use std::time::Duration;
use tokio::task;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::fs::File;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A simple and fast command-line tool to test URLs from a pipeline.
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Timeout for each request in seconds.
    #[arg(long, default_value_t = 10)]
    timeout: u64,

    /// Number of retries for failed requests.
    #[arg(long, default_value_t = 0)]
    retry: u32,

    /// Delay between retries in milliseconds.
    #[arg(long, default_value_t = 0)]
    delay: u64,

    /// Maximum number of concurrent requests (0 for unlimited).
    #[arg(long, default_value_t = 0)]
    concurrency: usize,

    /// Whether to follow HTTP redirects.
    #[arg(long, default_value_t = true)]
    follow_redirect: bool,

    /// Output file to save results (instead of stdout).
    #[arg(short, long)]
    output: Option<String>,

    /// Include request details in the output.
    #[arg(long)]
    include_req: bool,

    /// Include response body in the output.
    #[arg(long)]
    include_res: bool,

    /// Use HTTP/2 for requests.
    #[arg(long)]
    http2: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let redirect_policy = if cli.follow_redirect {
        Policy::limited(10) // Default reqwest behavior for following redirects
    } else {
        Policy::none()
    };

    let mut client_builder = Client::builder()
        .timeout(Duration::from_secs(cli.timeout))
        .redirect(redirect_policy);

    if !cli.http2 {
        client_builder = client_builder.http1_only();
    }

    let client = client_builder.build()?;

    let output_writer: Option<Arc<Mutex<BufWriter<File>>>> = if let Some(output_path) = &cli.output {
        let file = File::create(output_path).await?;
        Some(Arc::new(Mutex::new(BufWriter::new(file))))
    } else {
        None
    };

    let stdin = io::stdin();
    let handles = stdin
        .lock()
        .lines()
        .filter_map(Result::ok)
        .map(|url| {
            let client = client.clone();
            let cli = cli.clone(); // Clone cli for each task
            let output_writer = output_writer.clone(); // Clone output_writer for each task
            task::spawn(async move {
                if url.trim().is_empty() {
                    return;
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
                                for (name, value) in req.headers() {
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

                    match request_builder.send().await {
                        Ok(resp) => {
                            let status = resp.status();
                            let size = resp.content_length().unwrap_or(0);
                            let mut output_str = String::new();
                            
                            output_str.push_str(&format!("[{}] - Status: {}, Size: {}\n", url_str, status, size));

                            if let Some(raw_req) = req_for_display {
                                output_str.push_str(&format!("[Raw Request]\n{}\n", raw_req));
                            }

                            if cli.include_res {
                                if let Ok(body) = resp.text().await {
                                    output_str.push_str(&format!("[Response Body]\n{}\n", body));
                                }
                            }

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