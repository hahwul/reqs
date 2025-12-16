use anyhow::Result;
use async_trait::async_trait;
use regex::Regex;
use reqwest::{Client, redirect::Policy};
use rust_mcp_sdk::mcp_server::{ServerHandler, ServerRuntime, server_runtime};
use rust_mcp_sdk::schema::schema_utils::CallToolError;
use rust_mcp_sdk::schema::{
    CallToolRequest, CallToolResult, Implementation, InitializeResult, LATEST_PROTOCOL_VERSION,
    ListToolsRequest, ListToolsResult, RpcError, ServerCapabilities, ServerCapabilitiesTools,
    TextContent, Tool,
};
use rust_mcp_sdk::{McpServer, StdioTransport, TransportOptions};
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::constants::DEFAULT_REDIRECT_LIMIT;
use crate::filter::should_filter_response;
use crate::http::{build_request, format_raw_request, parse_headers, parse_request_line};
use crate::types::Cli;
use crate::utils::normalize_url_scheme;

/// Run the MCP (Model Context Protocol) server
pub async fn run_mcp_server(cli: Cli) -> Result<()> {
    // Define server details and capabilities
    let server_details = InitializeResult {
        server_info: Implementation {
            name: "reqs".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            title: Some("HTTP Request Testing Tool".to_string()),
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            ..Default::default()
        },
        meta: None,
        instructions: Some("Send HTTP requests and return response metadata.".to_string()),
        protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
    };

    // Create stdio transport
    let transport = StdioTransport::new(TransportOptions::default())
        .map_err(|e| anyhow::anyhow!("Failed to create stdio transport: {}", e))?;

    // Create handler
    let handler = ReqsServerHandler { cli: cli.clone() };

    // Create and start server
    let server: Arc<ServerRuntime> =
        server_runtime::create_server(server_details, transport, handler);
    server
        .start()
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    Ok(())
}

/// Custom handler for the MCP server
struct ReqsServerHandler {
    cli: Cli,
}

#[async_trait]
impl ServerHandler for ReqsServerHandler {
    async fn handle_list_tools_request(
        &self,
        _request: ListToolsRequest,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<ListToolsResult, RpcError> {
        let input_schema = create_tool_input_schema();

        Ok(ListToolsResult {
            tools: vec![Tool {
                name: "send_requests".to_string(),
                description: Some("Send HTTP requests and return response metadata. Accepts a list of requests with optional filters (filter_status, filter_string, filter_regex), HTTP options (follow_redirect, http2, headers), and output options (include_req, include_res) for LLM analysis.".to_string()),
                input_schema,
                annotations: None,
                meta: None,
                output_schema: None,
                title: Some("Send HTTP Requests".to_string()),
            }],
            meta: None,
            next_cursor: None,
        })
    }

    async fn handle_call_tool_request(
        &self,
        request: CallToolRequest,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        if request.tool_name() != "send_requests" {
            return Err(CallToolError::unknown_tool(format!(
                "Unknown tool: {}",
                request.tool_name()
            )));
        }

        let args = request.params.arguments.as_ref().ok_or_else(|| {
            CallToolError::new(
                RpcError::invalid_params().with_message("Missing arguments".to_string()),
            )
        })?;

        let requests = args
            .get("requests")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                CallToolError::new(
                    RpcError::invalid_params()
                        .with_message("requests parameter must be an array".to_string()),
                )
            })?;

        // Extract parameters
        let params = extract_tool_parameters(args, &self.cli)?;

        // Create HTTP client
        let client = build_mcp_client(&self.cli, &params)?;

        // Process requests
        let results = process_requests(requests, &client, &params).await;

        // Return results as tool response
        let result_text = results
            .iter()
            .map(|r| serde_json::to_string(r).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(CallToolResult::text_content(vec![TextContent::from(
            result_text,
        )]))
    }
}

/// Tool parameters extracted from request arguments
struct ToolParameters {
    filter_status: Vec<u16>,
    filter_string: Option<String>,
    filter_regex: Option<Regex>,
    include_req: bool,
    include_res: bool,
    follow_redirect: bool,
    http2: bool,
    custom_headers: Vec<String>,
}

/// Extract tool parameters from arguments
fn extract_tool_parameters(
    args: &serde_json::Map<String, serde_json::Value>,
    cli: &Cli,
) -> std::result::Result<ToolParameters, CallToolError> {
    let filter_status: Vec<u16> = args
        .get("filter_status")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_u64().map(|n| n as u16))
                .collect()
        })
        .unwrap_or_default();

    let filter_string = args
        .get("filter_string")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let filter_regex_str = args
        .get("filter_regex")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let include_req = args
        .get("include_req")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let include_res = args
        .get("include_res")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let follow_redirect = args
        .get("follow_redirect")
        .and_then(|v| v.as_bool())
        .unwrap_or(cli.follow_redirect);

    let http2 = args
        .get("http2")
        .and_then(|v| v.as_bool())
        .unwrap_or(cli.http2);

    let custom_headers: Vec<String> = args
        .get("headers")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    // Compile regex if provided
    let filter_regex = if let Some(regex_str) = &filter_regex_str {
        match Regex::new(regex_str) {
            Ok(re) => Some(re),
            Err(e) => {
                return Err(CallToolError::new(RpcError::invalid_params().with_message(
                    format!("Invalid regex provided for filter_regex: {}", e),
                )));
            }
        }
    } else {
        None
    };

    Ok(ToolParameters {
        filter_status,
        filter_string,
        filter_regex,
        include_req,
        include_res,
        follow_redirect,
        http2,
        custom_headers,
    })
}

/// Build HTTP client for MCP requests
fn build_mcp_client(
    cli: &Cli,
    params: &ToolParameters,
) -> std::result::Result<Client, CallToolError> {
    let redirect_policy = if params.follow_redirect {
        Policy::limited(DEFAULT_REDIRECT_LIMIT)
    } else {
        Policy::none()
    };

    // First, apply headers from CLI (global default), then custom headers from tool call (overrides)
    let mut default_headers = parse_headers(&cli.headers);
    default_headers.extend(parse_headers(&params.custom_headers));

    let mut client_builder = Client::builder()
        .timeout(Duration::from_secs(cli.timeout))
        .redirect(redirect_policy)
        .default_headers(default_headers);

    if !cli.verify_ssl {
        client_builder = client_builder.danger_accept_invalid_certs(true);
    }

    if let Some(proxy_url) = &cli.proxy {
        let proxy = reqwest::Proxy::all(proxy_url).map_err(|e| {
            CallToolError::new(
                RpcError::internal_error().with_message(format!("Failed to create proxy: {}", e)),
            )
        })?;
        client_builder = client_builder.proxy(proxy);
    }

    if !params.http2 {
        client_builder = client_builder.http1_only();
    }

    client_builder.build().map_err(|e| {
        CallToolError::new(
            RpcError::internal_error().with_message(format!("Failed to build HTTP client: {}", e)),
        )
    })
}

/// Process all requests and return results
async fn process_requests(
    requests: &[serde_json::Value],
    client: &Client,
    params: &ToolParameters,
) -> Vec<serde_json::Value> {
    let mut results = Vec::new();

    for req in requests {
        let req_str = match req.as_str() {
            Some(s) => s.trim(),
            None => continue,
        };

        if req_str.is_empty() {
            continue;
        }

        let (method, url_str, body) = parse_request_line(req_str);

        if url_str.is_empty() {
            continue;
        }

        let url_str = normalize_url_scheme(&url_str);

        let request_builder = build_request(client, &method, &url_str, &body);

        // Capture raw request if needed
        let raw_request = if params.include_req {
            request_builder
                .try_clone()
                .unwrap()
                .build()
                .ok()
                .map(|req| format_raw_request(&req, params.http2, None))
        } else {
            None
        };

        let start_time = Instant::now();
        match request_builder.send().await {
            Ok(resp) => {
                let elapsed = start_time.elapsed();
                let status = resp.status();
                let size = resp.content_length().unwrap_or(0);
                let ip_addr = resp
                    .remote_addr()
                    .map(|s| s.ip().to_string())
                    .unwrap_or_default();

                // Fetch response body if needed for filtering or output
                let body_text = if params.include_res
                    || params.filter_string.is_some()
                    || params.filter_regex.is_some()
                {
                    Some(resp.text().await.unwrap_or_default())
                } else {
                    None
                };

                if should_filter_response(
                    status.as_u16(),
                    &body_text,
                    &params.filter_status,
                    &params.filter_string,
                    &params.filter_regex,
                ) {
                    continue; // Skip this result
                }

                let mut result = json!({
                    "method": method,
                    "url": url_str,
                    "status_code": status.as_u16(),
                    "content_length": size,
                    "response_time_ms": elapsed.as_millis(),
                });

                if !ip_addr.is_empty() {
                    result["ip_address"] = ip_addr.into();
                }

                if let Some(raw_req) = raw_request {
                    result["raw_request"] = raw_req.into();
                }

                if params.include_res
                    && let Some(body) = body_text
                {
                    result["response_body"] = body.into();
                }

                results.push(result);
            }
            Err(err) => {
                results.push(json!({
                    "method": method,
                    "url": url_str,
                    "error": err.to_string(),
                }));
            }
        }
    }

    results
}

/// Create input schema for the send_requests tool
fn create_tool_input_schema() -> rust_mcp_sdk::schema::ToolInputSchema {
    use std::collections::HashMap;

    let mut properties = HashMap::new();

    // requests parameter
    let mut requests_prop = serde_json::Map::new();
    requests_prop.insert("type".to_string(), json!("array"));
    requests_prop.insert("description".to_string(), json!("List of HTTP requests. Each request can be a simple URL or a string with METHOD URL BODY format (e.g., 'POST https://example.com data=value')"));
    let mut items = serde_json::Map::new();
    items.insert("type".to_string(), json!("string"));
    requests_prop.insert("items".to_string(), json!(items));
    properties.insert("requests".to_string(), requests_prop);

    // filter_status parameter
    let mut filter_status_prop = serde_json::Map::new();
    filter_status_prop.insert("type".to_string(), json!("array"));
    filter_status_prop.insert("description".to_string(), json!("Filter results by HTTP status codes (e.g., [200, 404]). Only responses with these status codes will be returned."));
    let mut status_items = serde_json::Map::new();
    status_items.insert("type".to_string(), json!("number"));
    filter_status_prop.insert("items".to_string(), json!(status_items));
    properties.insert("filter_status".to_string(), filter_status_prop);

    // filter_string parameter
    let mut filter_string_prop = serde_json::Map::new();
    filter_string_prop.insert("type".to_string(), json!("string"));
    filter_string_prop.insert("description".to_string(), json!("Filter results by string match in response body. Only responses containing this string will be returned."));
    properties.insert("filter_string".to_string(), filter_string_prop);

    // filter_regex parameter
    let mut filter_regex_prop = serde_json::Map::new();
    filter_regex_prop.insert("type".to_string(), json!("string"));
    filter_regex_prop.insert("description".to_string(), json!("Filter results by regex pattern in response body. Only responses matching this pattern will be returned."));
    properties.insert("filter_regex".to_string(), filter_regex_prop);

    // include_req parameter
    let mut include_req_prop = serde_json::Map::new();
    include_req_prop.insert("type".to_string(), json!("boolean"));
    include_req_prop.insert(
        "description".to_string(),
        json!("Include raw HTTP request details in the output."),
    );
    properties.insert("include_req".to_string(), include_req_prop);

    // include_res parameter
    let mut include_res_prop = serde_json::Map::new();
    include_res_prop.insert("type".to_string(), json!("boolean"));
    include_res_prop.insert(
        "description".to_string(),
        json!("Include response body in the output."),
    );
    properties.insert("include_res".to_string(), include_res_prop);

    // follow_redirect parameter
    let mut follow_redirect_prop = serde_json::Map::new();
    follow_redirect_prop.insert("type".to_string(), json!("boolean"));
    follow_redirect_prop.insert(
        "description".to_string(),
        json!("Whether to follow HTTP redirects. Defaults to true."),
    );
    properties.insert("follow_redirect".to_string(), follow_redirect_prop);

    // http2 parameter
    let mut http2_prop = serde_json::Map::new();
    http2_prop.insert("type".to_string(), json!("boolean"));
    http2_prop.insert(
        "description".to_string(),
        json!("Use HTTP/2 for requests. Defaults to false (HTTP/1.1)."),
    );
    properties.insert("http2".to_string(), http2_prop);

    // headers parameter
    let mut headers_prop = serde_json::Map::new();
    headers_prop.insert("type".to_string(), json!("array"));
    headers_prop.insert("description".to_string(), json!("Custom headers to add to the request (e.g., [\"User-Agent: my-app\", \"Authorization: Bearer token\"])"));
    let mut headers_items = serde_json::Map::new();
    headers_items.insert("type".to_string(), json!("string"));
    headers_prop.insert("items".to_string(), json!(headers_items));
    properties.insert("headers".to_string(), headers_prop);

    const REQUIRED_FIELDS: &[&str] = &["requests"];
    rust_mcp_sdk::schema::ToolInputSchema::new(
        REQUIRED_FIELDS.iter().map(|s| s.to_string()).collect(),
        Some(properties),
    )
}
