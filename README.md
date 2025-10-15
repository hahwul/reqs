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
```bash
# From pipeline
echo "https://example.com" | reqs

# With options
cat urls.txt | reqs --timeout 5 --concurrency 10 --format jsonl
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
