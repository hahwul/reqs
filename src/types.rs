use clap::Parser;

/// Output format options
#[derive(clap::ValueEnum, Debug, Clone, Default)]
pub enum OutputFormat {
    #[default]
    Plain,
    Jsonl,
    Csv,
}

/// CLI arguments structure
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    // NETWORK
    /// Timeout for each request in seconds.
    #[arg(long, default_value_t = 10, help_heading = "NETWORK")]
    pub timeout: u64,

    /// Number of retries for failed requests.
    #[arg(long, default_value_t = 0, help_heading = "NETWORK")]
    pub retry: u32,

    /// Delay between retries in milliseconds.
    #[arg(long, default_value_t = 0, help_heading = "NETWORK")]
    pub delay: u64,

    /// Maximum number of concurrent requests (0 for unlimited).
    #[arg(long, default_value_t = 0, help_heading = "NETWORK")]
    pub concurrency: usize,

    /// Use a proxy for requests (e.g., "http://127.0.0.1:8080").
    #[arg(long, help_heading = "NETWORK")]
    pub proxy: Option<String>,

    /// Verify SSL certificates (default: false, insecure).
    #[arg(long, default_value_t = false, help_heading = "NETWORK")]
    pub verify_ssl: bool,

    /// Limit requests per second. E.g., --rate-limit 100.
    #[arg(long, help_heading = "NETWORK")]
    pub rate_limit: Option<u64>,

    /// Random delay between requests in milliseconds. E.g., --random-delay 100:500.
    #[arg(long, help_heading = "NETWORK")]
    pub random_delay: Option<String>,

    // HTTP
    /// Whether to follow HTTP redirects.
    #[arg(long, default_value_t = true, help_heading = "HTTP")]
    pub follow_redirect: bool,

    /// Use HTTP/2 for requests.
    #[arg(long, help_heading = "HTTP")]
    pub http2: bool,

    /// Custom headers to add to the request (e.g., "User-Agent: my-app").
    #[arg(short = 'H', long, help_heading = "HTTP")]
    pub headers: Vec<String>,

    // OUTPUT
    /// Output file to save results (instead of stdout).
    #[arg(short, long, help_heading = "OUTPUT")]
    pub output: Option<String>,

    /// Output format.
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Plain, help_heading = "OUTPUT")]
    pub format: OutputFormat,

    #[arg(
        short = 'S',
        long,
        help_heading = "OUTPUT",
        long_help = "Custom format string for plain output (e.g. \"%method %url -> %code\").\nPlaceholders: %method, %url, %status, %code, %size, %time, %ip, %title"
    )]
    pub strf: Option<String>,

    /// Include request details in the output.
    #[arg(long, help_heading = "OUTPUT")]
    pub include_req: bool,

    /// Include response body in the output.
    #[arg(long, help_heading = "OUTPUT")]
    pub include_res: bool,

    /// Include title from response body in the output.
    #[arg(long, help_heading = "OUTPUT")]
    pub include_title: bool,

    /// Disable color output.
    #[arg(long, help_heading = "OUTPUT")]
    pub no_color: bool,

    // FILTER
    /// Filter by specific HTTP status codes (e.g., "200,404").
    #[arg(long, value_delimiter = ',', help_heading = "FILTER")]
    pub filter_status: Vec<u16>,

    /// Filter by string in response body.
    #[arg(long, help_heading = "FILTER")]
    pub filter_string: Option<String>,

    /// Filter by regex in response body.
    #[arg(long, help_heading = "FILTER")]
    pub filter_regex: Option<String>,

    // MCP
    /// Run in MCP (Model Context Protocol) server mode.
    #[arg(long, help_heading = "MCP")]
    pub mcp: bool,
}
