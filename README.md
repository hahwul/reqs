# reqs

A simple and fast command-line tool to test URLs from a pipeline.

## Installation

### From Cargo
```bash
cargo install reqs
```

### From Homebrew Tap
```bash
brew tap hahwul/reqs
brew install reqs
```

## Basic Usage

```
Reqs is a command-line tool for massive sending requests

Usage: reqs [OPTIONS]

Options:
  -h, --help     Print help (see more with '--help')
  -V, --version  Print version

NETWORK:
      --timeout <TIMEOUT>            Timeout for each request in seconds [default: 10]
      --retry <RETRY>                Number of retries for failed requests [default: 0]
      --delay <DELAY>                Delay between retries in milliseconds [default: 0]
      --concurrency <CONCURRENCY>    Maximum number of concurrent requests (0 for unlimited) [default: 0]
      --proxy <PROXY>                Use a proxy for requests (e.g., "http://127.0.0.1:8080")
      --verify-ssl                   Verify SSL certificates (default: false, insecure)
      --rate-limit <RATE_LIMIT>      Limit requests per second. E.g., --rate-limit 100
      --random-delay <RANDOM_DELAY>  Random delay between requests in milliseconds. E.g., --random-delay 100:500

HTTP:
      --follow-redirect    Whether to follow HTTP redirects
      --http2              Use HTTP/2 for requests
  -H, --headers <HEADERS>  Custom headers to add to the request (e.g., "User-Agent: my-app")

OUTPUT:
  -o, --output <OUTPUT>  Output file to save results (instead of stdout)
  -f, --format <FORMAT>  Output format [default: plain] [possible values: plain, jsonl, csv]
  -S, --strf <STRF>      Custom format string for plain output (e.g. "%method %url -> %code").
                         Placeholders: %method, %url, %status, %code, %size, %time, %ip, %title
      --include-req      Include request details in the output
      --include-res      Include response body in the output
      --include-title    Include title from response body in the output
      --no-color         Disable color output

FILTER:
      --filter-status <FILTER_STATUS>  Filter by specific HTTP status codes (e.g., "200,404")
      --filter-string <FILTER_STRING>  Filter by string in response body
      --filter-regex <FILTER_REGEX>    Filter by regex in response body

MCP:
      --mcp  Run in MCP (Model Context Protocol) server mode
```

```bash
# From pipeline
echo "https://example.com" | reqs

# With options
cat urls.txt | reqs --timeout 5 --concurrency 10 --format jsonl

# With POST request
echo "POST https://example.com" "name=user&role=admin" | reqs
```

## MCP Mode
`reqs` can run as an MCP server, allowing AI assistants to send HTTP requests through it.

```bash
# Start MCP server with default settings
reqs --mcp

# Start with custom timeout
reqs --mcp --timeout 30
```

## Documentation

For detailed documentation, please visit [reqs.hahwul.com](https://reqs.hahwul.com).
