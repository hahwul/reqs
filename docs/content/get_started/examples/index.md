---
title: "Examples"
weight: 3
---

This page provides practical examples and common use cases for Reqs.

## Basic Examples

### Test a Single URL

```bash
echo "https://example.com" | reqs
```

### Test Multiple URLs

```bash
cat << EOF | reqs
https://example.com
https://github.com
https://www.hahwul.com
EOF
```

### Test URLs from a File

```bash
cat urls.txt | reqs
```

## Output Format Examples

### Plain Text Output

```bash
echo "https://example.com" | reqs
```

Output:
```
[200] https://example.com (42ms)
```

### JSON Lines Output

```bash
echo "https://example.com" | reqs --format jsonl
```

Output:
```json
{"content_length":1256,"method":"GET","response_time_ms":42,"status_code":200,"url":"https://example.com"}
```

### CSV Output

```bash
echo "https://example.com" | reqs --format csv
```

Output:
```csv
url,method,status_code,content_length,response_time_ms
https://example.com,GET,200,1256,42
```

## HTTP Method Examples

### GET Request

```bash
echo "https://api.example.com/users" | reqs
```

### POST Request

```bash
echo "POST https://api.example.com/users {\"name\":\"John\",\"email\":\"john@example.com\"}" | reqs
```

### PUT Request

```bash
echo "PUT https://api.example.com/users/1 {\"name\":\"Jane\"}" | reqs
```

### DELETE Request

```bash
echo "DELETE https://api.example.com/users/1" | reqs
```

## Filtering Examples

### Filter by Status Code

Show only successful (200) responses:

```bash
cat urls.txt | reqs --filter-status 200
```

Show only client errors (4xx):

```bash
cat urls.txt | reqs --filter-status 400 --filter-status 404 --filter-status 403
```

### Filter by Response Content

Find pages containing "login":

```bash
cat urls.txt | reqs --filter-string "login"
```

### Filter with Regex

Find pages with email addresses:

```bash
cat urls.txt | reqs --filter-regex "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}"
```

## Advanced Examples

### API Testing with Authentication

```bash
cat endpoints.txt | reqs \
  --headers "Authorization: Bearer YOUR_TOKEN" \
  --headers "Content-Type: application/json" \
  --format jsonl
```

### Rate-Limited Scanning

```bash
cat urls.txt | reqs \
  --concurrency 5 \
  --delay 200 \
  --timeout 10
```

### Export Results for Analysis

```bash
cat urls.txt | reqs \
  --format jsonl \
  --filter-status 200 \
  --output results.jsonl
```

Then analyze with `jq`:

```bash
# Get average response time
cat results.jsonl | jq -s 'map(.response_time_ms) | add / length'

# Get slowest endpoints
cat results.jsonl | jq -s 'sort_by(.response_time_ms) | reverse | .[0:5]'

# Count by status code
cat results.jsonl | jq -s 'group_by(.status_code) | map({status: .[0].status_code, count: length})'
```

### Security Testing Workflow

```bash
# Test common endpoints
cat << EOF | reqs --format jsonl --output api-test.jsonl
https://api.example.com/v1/users
https://api.example.com/v1/admin
https://api.example.com/v1/health
https://api.example.com/v1/metrics
EOF

# Check results
cat api-test.jsonl | jq 'select(.status_code == 200)'
```

### Multiple POST Requests

```bash
cat << EOF | reqs --format jsonl
POST https://api.example.com/users {"name":"Alice","role":"user"}
POST https://api.example.com/users {"name":"Bob","role":"admin"}
POST https://api.example.com/users {"name":"Charlie","role":"user"}
EOF
```

## Pipeline Integration

### Combine with Other Tools

Using with `httpx`:

```bash
# Get URLs from httpx and test with reqs
echo "example.com" | httpx -silent | reqs
```

Using with `waybackurls`:

```bash
# Get historical URLs and test them
echo "example.com" | waybackurls | reqs --filter-status 200
```

Using with `gau`:

```bash
# Get URLs from common crawl and test
echo "example.com" | gau | reqs --concurrency 10
```

### Extract and Test

```bash
# Extract URLs from a webpage and test them
curl -s https://example.com | \
  grep -oP 'https?://[^"]+' | \
  sort -u | \
  reqs --filter-status 200
```

## Docker Examples

### Basic Usage

```bash
echo "https://example.com" | docker run -i ghcr.io/hahwul/reqs:latest
```

### With Volume Mount

```bash
docker run -i -v $(pwd):/data ghcr.io/hahwul/reqs:latest \
  --format jsonl \
  --output /data/results.jsonl \
  < urls.txt
```

### MCP Mode

```bash
docker run -i ghcr.io/hahwul/reqs:latest --mcp
```

## Automation Examples

### Continuous Monitoring Script

```bash
#!/bin/bash
# monitor.sh - Monitor website health

URLS="urls.txt"
OUTPUT="health-$(date +%Y%m%d-%H%M%S).jsonl"

cat "$URLS" | reqs \
  --format jsonl \
  --timeout 10 \
  --retry 2 \
  --output "$OUTPUT"

# Alert on failures
cat "$OUTPUT" | jq -r 'select(.status_code >= 500) | .url' | while read url; do
  echo "ALERT: $url is down!"
done
```

### Compare Environments

```bash
#!/bin/bash
# compare-envs.sh - Compare staging vs production

cat endpoints.txt | sed 's|^|https://staging.example.com|' | \
  reqs --format jsonl --output staging.jsonl

cat endpoints.txt | sed 's|^|https://example.com|' | \
  reqs --format jsonl --output production.jsonl

# Compare response times
echo "Staging avg:" $(cat staging.jsonl | jq -s 'map(.response_time_ms) | add / length')
echo "Production avg:" $(cat production.jsonl | jq -s 'map(.response_time_ms) | add / length')
```

## Troubleshooting Examples

### Debug Mode

```bash
# Use verbose output formats to see what's happening
cat urls.txt | reqs --format jsonl | jq .
```

### Test Connectivity

```bash
# Test if you can reach the hosts
echo "https://example.com" | reqs --timeout 30
```

### Test with Proxy

```bash
# Route through a proxy
cat urls.txt | reqs --proxy http://localhost:8080
```

### Test HTTP vs HTTPS

```bash
cat << EOF | reqs
http://example.com
https://example.com
EOF
```

## Performance Benchmarking

### Compare Response Times

```bash
# Test the same URL multiple times
for i in {1..10}; do
  echo "https://example.com"
done | reqs --format jsonl | \
  jq -s 'map(.response_time_ms) | {min: min, max: max, avg: (add/length)}'
```

### Concurrent Load Testing

```bash
# Generate multiple concurrent requests
for i in {1..100}; do
  echo "https://example.com"
done | reqs --concurrency 50 --format jsonl
```

## Tips and Best Practices

1. **Use appropriate concurrency**: Start with low values (5-10) and increase based on your needs
2. **Set reasonable timeouts**: Default is usually good, but adjust for slow services
3. **Save output to files**: Makes it easier to analyze results later
4. **Use JSONL for structured data**: Easier to process with tools like `jq`
5. **Respect rate limits**: Use `--delay` to avoid overwhelming servers
6. **Filter early**: Use status code filters to reduce noise
7. **Combine with other tools**: Reqs works great in Unix pipelines
