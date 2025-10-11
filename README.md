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

The server provides a `send_requests` tool that accepts a list of HTTP requests and returns their results.

**Input format:**
```json
{
  "requests": [
    "https://www.hahwul.com",
    "https://www.hahwul.com/about/",
    "https://github.com",
    "POST https://www.hahwul.com a=d"
  ]
}
```

**Output format:**
```json
{"content_length":131,"method":"POST","response_time_ms":42,"status_code":405,"url":"https://www.hahwul.com"}
{"content_length":32498,"method":"GET","response_time_ms":43,"status_code":200,"url":"https://www.hahwul.com"}
{"content_length":30063,"method":"GET","response_time_ms":44,"status_code":200,"url":"https://www.hahwul.com/about/"}
{"content_length":0,"method":"GET","response_time_ms":49,"status_code":200,"url":"https://github.com"}
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
