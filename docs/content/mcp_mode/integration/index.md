---
title: "Integration with AI Tools"
weight: 2
---

Reqs can be integrated with various AI assistants and tools that support the Model Context Protocol (MCP).

## Claude Desktop Integration

Claude Desktop natively supports MCP servers. To integrate Reqs with Claude Desktop:

### 1. Install Reqs

First, ensure Reqs is installed and available in your PATH:

```bash
cargo install reqs
```

### 2. Configure Claude Desktop

Edit your Claude Desktop configuration file:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`

**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

Add the Reqs MCP server configuration:

```json
{
  "mcpServers": {
    "reqs": {
      "command": "reqs",
      "args": ["--mcp"]
    }
  }
}
```

### 3. Configure with Options

You can add custom options to the configuration:

```json
{
  "mcpServers": {
    "reqs": {
      "command": "reqs",
      "args": [
        "--mcp",
        "--timeout", "30",
        "--concurrency", "20",
        "--headers", "User-Agent: Claude-Desktop/1.0"
      ]
    }
  }
}
```

### 4. Restart Claude Desktop

After updating the configuration, restart Claude Desktop for the changes to take effect.

### 5. Using Reqs in Claude

Once configured, you can ask Claude to send HTTP requests:

**Example conversations:**

> "Can you check the status of https://example.com?"

> "Send GET requests to these URLs and tell me which ones return 200 status codes: [list of URLs]"

> "Test POST requests to https://api.example.com/endpoint with JSON body {\"test\": \"data\"}"

## Other MCP Clients

Reqs can work with any MCP-compatible client. The key requirements are:

1. The client must support the stdio transport
2. The client must implement the MCP protocol

## Example: Using with MCP Client Library

If you're building your own MCP client:

```javascript
const { Client } = require('@modelcontextprotocol/sdk');

const client = new Client({
  name: 'my-client',
  version: '1.0.0'
});

// Connect to Reqs MCP server
await client.connect({
  command: 'reqs',
  args: ['--mcp']
});

// List available tools
const tools = await client.listTools();
console.log(tools);

// Send requests
const result = await client.callTool('send_requests', {
  requests: [
    'https://example.com',
    'https://github.com'
  ],
  filter_status: [200]
});

console.log(result);
```

## Troubleshooting

### Server Not Starting

If the MCP server doesn't start:

1. Verify Reqs is installed: `reqs --version`
2. Check if the binary is in your PATH: `which reqs`
3. Try starting manually: `reqs --mcp`

### Connection Issues

If the client can't connect:

1. Check the configuration file syntax
2. Ensure the command path is correct
3. Verify file permissions on the Reqs binary
4. Check client logs for error messages

### Request Failures

If requests are failing:

1. Test the URLs manually: `echo "URL" | reqs`
2. Check network connectivity
3. Verify timeout settings aren't too low
4. Check for proxy/firewall issues

## Advanced Configuration

### Using with Proxy

```json
{
  "mcpServers": {
    "reqs": {
      "command": "reqs",
      "args": [
        "--mcp",
        "--proxy", "http://proxy.example.com:8080"
      ]
    }
  }
}
```

### Custom User Agent

```json
{
  "mcpServers": {
    "reqs": {
      "command": "reqs",
      "args": [
        "--mcp",
        "--headers", "User-Agent: MyApp/1.0"
      ]
    }
  }
}
```

### Rate Limiting

```json
{
  "mcpServers": {
    "reqs": {
      "command": "reqs",
      "args": [
        "--mcp",
        "--delay", "100",
        "--concurrency", "5"
      ]
    }
  }
}
```

## Security Considerations

When running Reqs as an MCP server:

1. **Network Access**: The AI assistant will be able to send HTTP requests through Reqs. Be mindful of what networks the server has access to.

2. **Credentials**: If you configure headers with credentials, they will be used for all requests from the AI assistant.

3. **Rate Limiting**: Consider setting concurrency and delay limits to prevent overwhelming target servers.

4. **Logging**: Review the requests being made to ensure they align with your expectations.

## Next Steps

- Explore the [send_requests tool documentation](/mcp_mode/overview#send_requests)
- Learn about [filtering and output options](/get_started/usage#filtering-results)
- Check out [example use cases](/mcp_mode/overview#use-cases)
