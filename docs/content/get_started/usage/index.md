---
title: "Basic Usage"
weight: 2
---

Reqs reads URLs from stdin and sends HTTP requests to them. It's designed to work seamlessly in Unix pipelines.

## Quick Start

The simplest way to use Reqs is to pipe URLs to it:

```bash
echo "https://example.com" | reqs
```

## Reading from Files

You can read URLs from a file:

```bash
cat urls.txt | reqs
```

Example `urls.txt`:
```
https://example.com
https://github.com
https://www.hahwul.com
```

## HTTP Methods

By default, Reqs uses the GET method. You can specify other HTTP methods:

```bash
# POST request
echo "POST https://example.com body=data" | reqs

# PUT request
echo "PUT https://api.example.com/resource data" | reqs

# DELETE request
echo "DELETE https://api.example.com/resource" | reqs
```

## Output Formats

Reqs supports multiple output formats:

### Plain Text (Default)

```bash
cat urls.txt | reqs
```

Output:
```
[200] https://example.com (42ms)
[200] https://github.com (103ms)
[404] https://example.com/notfound (35ms)
```

### JSON Lines (JSONL)

```bash
cat urls.txt | reqs --format jsonl
```

Output:
```json
{"content_length":1256,"method":"GET","response_time_ms":42,"status_code":200,"url":"https://example.com"}
{"content_length":52341,"method":"GET","response_time_ms":103,"status_code":200,"url":"https://github.com"}
{"content_length":1024,"method":"GET","response_time_ms":35,"status_code":404,"url":"https://example.com/notfound"}
```

### CSV

```bash
cat urls.txt | reqs --format csv
```

Output:
```csv
url,method,status_code,content_length,response_time_ms
https://example.com,GET,200,1256,42
https://github.com,GET,200,52341,103
https://example.com/notfound,GET,404,1024,35
```

## Common Options

### Concurrency

Control the number of concurrent requests:

```bash
cat urls.txt | reqs --concurrency 10
```

### Timeout

Set request timeout in seconds:

```bash
cat urls.txt | reqs --timeout 5
```

### Retries

Specify the number of retry attempts:

```bash
cat urls.txt | reqs --retry 3
```

### Custom Headers

Add custom HTTP headers:

```bash
cat urls.txt | reqs --headers "Authorization: Bearer token123"
cat urls.txt | reqs --headers "User-Agent: MyBot/1.0" --headers "Accept: application/json"
```

### Follow Redirects

By default, Reqs follows redirects. To disable:

```bash
cat urls.txt | reqs --no-follow-redirect
```

### HTTP/2

Use HTTP/2 protocol:

```bash
cat urls.txt | reqs --http2
```

## Filtering Results

### Filter by Status Code

Show only responses with specific status codes:

```bash
cat urls.txt | reqs --filter-status 200
cat urls.txt | reqs --filter-status 200 --filter-status 201
```

### Filter by Response Content

Show only responses containing specific text:

```bash
cat urls.txt | reqs --filter-string "success"
```

### Filter by Regex Pattern

Show only responses matching a regex pattern:

```bash
cat urls.txt | reqs --filter-regex "error.*code"
```

## Output to File

Save results to a file instead of stdout:

```bash
cat urls.txt | reqs --output results.txt
cat urls.txt | reqs --format jsonl --output results.jsonl
```

## Complete Example

Combining multiple options:

```bash
cat urls.txt | reqs \
  --concurrency 20 \
  --timeout 10 \
  --retry 2 \
  --format jsonl \
  --filter-status 200 \
  --headers "User-Agent: Reqs/0.0.1" \
  --output results.jsonl
```

This command:
- Processes URLs with 20 concurrent connections
- Sets a 10-second timeout per request
- Retries failed requests up to 2 times
- Outputs in JSONL format
- Filters to show only 200 status codes
- Uses a custom User-Agent header
- Saves results to `results.jsonl`
