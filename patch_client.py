with open("src/http/client.rs", "r") as f:
    content = f.read()

content = content.replace('Cli::parse_from(&["reqs"])', 'Cli::parse_from(["reqs"])')
content = content.replace('Cli::parse_from(&["reqs", "-H", "User-Agent: test-agent"])', 'Cli::parse_from(["reqs", "-H", "User-Agent: test-agent"])')
content = content.replace('Cli::parse_from(&["reqs", "--proxy", "http://127.0.0.1:8080"])', 'Cli::parse_from(["reqs", "--proxy", "http://127.0.0.1:8080"])')
content = content.replace('Cli::parse_from(&["reqs", "--proxy", "htt\\0p://127.0.0.1:8080"])', 'Cli::parse_from(["reqs", "--proxy", "htt\\0p://127.0.0.1:8080"])')
content = content.replace('Cli::parse_from(&["reqs", "--verify-ssl"])', 'Cli::parse_from(["reqs", "--verify-ssl"])')
content = content.replace('Cli::parse_from(&["reqs", "--http2"])', 'Cli::parse_from(["reqs", "--http2"])')

with open("src/http/client.rs", "w") as f:
    f.write(content)
