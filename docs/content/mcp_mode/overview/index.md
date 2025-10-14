---
title: "MCP Server Overview"
weight: 1
---

The Model Context Protocol (MCP) is a standard protocol for communication between AI assistants and external tools. Reqs can run as an MCP server, allowing AI assistants like Claude to send HTTP requests through it.

## What is MCP Mode?

MCP mode transforms Reqs into a server that listens for requests from AI assistants. Instead of reading URLs from stdin, Reqs waits for the AI assistant to send requests via the MCP protocol.

## Starting the MCP Server

To start Reqs in MCP mode, use the `--mcp` flag:

```bash
reqs --mcp
```

The server will start and wait for connections from MCP clients.

## Server Options

You can configure the MCP server with various options:

### Custom Timeout

```bash
reqs --mcp --timeout 30
```

### Custom Headers

```bash
reqs --mcp --headers "Authorization: Bearer token123"
```

### Concurrency Control

```bash
reqs --mcp --concurrency 20
```

### HTTP/2 Support

```bash
reqs --mcp --http2
```

### Combining Options

```bash
reqs --mcp \
  --timeout 30 \
  --concurrency 20 \
  --headers "User-Agent: AI-Assistant/1.0" \
  --http2
```

## Available Tools

When running as an MCP server, Reqs provides the following tool:

### send_requests

The `send_requests` tool accepts a list of HTTP requests and returns their results.

**Input Parameters:**

- `requests` (required): Array of URLs or request strings in format `METHOD URL BODY`
- `filter_status` (optional): Filter results by HTTP status codes (e.g., `[200, 404]`)
- `filter_string` (optional): Filter results containing specific text in response body
- `filter_regex` (optional): Filter results matching regex pattern in response body
- `follow_redirect` (optional): Whether to follow HTTP redirects. Defaults to true
- `http2` (optional): Use HTTP/2 for requests. Defaults to false (HTTP/1.1)
- `headers` (optional): Custom headers to add to the request (e.g., `["User-Agent: my-app", "Authorization: Bearer token"]`)
- `include_req` (optional): Include raw HTTP request details in output
- `include_res` (optional): Include response body in output

**Example Input:**

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

**Output Format:**

Basic output includes:

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

## Use Cases

MCP mode is ideal for:

- **AI-Powered Security Testing**: Let AI assistants discover and test endpoints
- **Automated Reconnaissance**: AI can analyze responses and make intelligent follow-up requests
- **Interactive Analysis**: Chat with an AI while it tests web applications in real-time
- **Dynamic Testing Workflows**: AI can adapt testing strategies based on responses

## Benefits

- **Natural Language Interface**: Describe what you want to test in plain language
- **Context-Aware**: AI assistants can analyze responses and make intelligent decisions
- **No Scripting Required**: Complex testing workflows without writing code
- **Interactive**: Real-time feedback and conversation during testing
