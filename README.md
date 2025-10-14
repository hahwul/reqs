# reqs

A simple and fast command-line tool to test URLs from a pipeline.

## Features

- Send HTTP requests to multiple URLs concurrently
- Support for various HTTP methods (GET, POST, PUT, DELETE, etc.)
- Custom headers, timeouts, and retries
- Output in plain text or JSON Lines format
- Filter by status code, response body content, or regex
- **MCP (Model Context Protocol) server mode** for integration with AI tools

## MCP Server Mode

Run `reqs` as an MCP server to enable AI assistants to send HTTP requests:

```bash
reqs --mcp
```

### MCP Tool: send_requests

The server provides a `send_requests` tool that accepts a list of HTTP requests and returns their results. The tool supports filtering and output customization for LLM analysis.

**Input format:**
```json
{
  "requests": [
    "https://www.hahwul.com",
    "https://www.hahwul.com/about/",
    "https://github.com",
    "POST https://www.hahwul.com a=d"
  ],
  "filter_status": [200, 201],
  "filter_string": "example text",
  "filter_regex": "pattern.*match",
  "follow_redirect": true,
  "http2": false,
  "headers": ["User-Agent: my-app", "Authorization: Bearer token"],
  "include_req": true,
  "include_res": true
}
```

**Parameters:**
- `requests` (required): Array of URLs or request strings in format `METHOD URL BODY`
- `filter_status` (optional): Filter results by HTTP status codes (e.g., `[200, 404]`)
- `filter_string` (optional): Filter results containing specific text in response body
- `filter_regex` (optional): Filter results matching regex pattern in response body
- `follow_redirect` (optional): Whether to follow HTTP redirects. Defaults to true
- `http2` (optional): Use HTTP/2 for requests. Defaults to false (HTTP/1.1)
- `headers` (optional): Custom headers to add to the request (e.g., `["User-Agent: my-app", "Authorization: Bearer token"]`)
- `include_req` (optional): Include raw HTTP request details in output
- `include_res` (optional): Include response body in output

**Output format:**
```json
{"content_length":131,"method":"POST","response_time_ms":42,"status_code":405,"url":"https://www.hahwul.com"}
{"content_length":32498,"method":"GET","response_time_ms":43,"status_code":200,"url":"https://www.hahwul.com"}
{"content_length":30063,"method":"GET","response_time_ms":44,"status_code":200,"url":"https://www.hahwul.com/about/"}
{"content_length":0,"method":"GET","response_time_ms":49,"status_code":200,"url":"https://github.com"}
```

With `include_req` and `include_res`, the output includes additional fields:
```json
{"content_length":149,"ip_address":"127.0.0.1","method":"GET","raw_request":"GET /path HTTP/1.1\nHost: example.com\n","response_body":"<html>...</html>","response_time_ms":42,"status_code":200,"url":"https://example.com"}
```

## Usage

### Normal Mode

```bash
# From pipeline
echo "https://example.com" | reqs

# With options
cat urls.txt | reqs --timeout 5 --concurrency 10 --format jsonl
```

### MCP Mode

```bash
# Start MCP server with default settings
reqs --mcp

# Start with custom timeout
reqs --mcp --timeout 30

# Start with custom headers
reqs --mcp --headers "Authorization: Bearer token123"
```

## Installation

```bash
cargo install reqs
```

Or build from source:

```bash
git clone https://github.com/hahwul/reqs
cd reqs
cargo build --release
```

## Documentation

For detailed documentation, please visit [reqs.hahwul.com](https://reqs.hahwul.com) or check the `docs/` directory.

