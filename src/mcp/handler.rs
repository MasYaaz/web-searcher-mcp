//! JSON-RPC 2.0 Request Dispatcher and MCP Protocol Handler.
//!
//! This module parses incoming JSON-RPC requests, routes Model Context Protocol (MCP) methods
//! (`initialize`, `ping`, `tools/list`, `tools/call`), executes tool invocations against the web engine,
//! and returns formatted JSON-RPC responses.

use crate::engine::fetch::fetch_markdown;
use crate::engine::search::search_web;
use serde_json::{json, Value};

/// Handles incoming JSON-RPC 2.0 requests for the MCP Web Agent.
///
/// Filters out notifications (requests missing an `id` or starting with `notifications/`),
/// dispatches supported protocol methods (`initialize`, `ping`, `tools/list`, `tools/call`),
/// and returns a JSON `Value` response.
///
/// # Arguments
/// * `req_text` - A string slice containing the raw JSON-RPC request payload.
///
/// # Returns
/// * `Some(Value)` - A structured JSON-RPC 2.0 response object.
/// * `None` - If the request text is invalid JSON or is a JSON-RPC notification.
pub fn handle_rpc_request(req_text: &str) -> Option<Value> {
    let req: Value = serde_json::from_str(req_text).ok()?;
    let id = req.get("id").cloned();
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");

    // Ignore all JSON-RPC notifications (missing an ID or starting with notifications/)
    if id.is_none() || method.starts_with("notifications/") {
        return None;
    }

    let response = match method {
        "initialize" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": "mcp-web-agent",
                    "version": "1.5.0"
                }
            }
        }),
        "ping" => json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
        "tools/list" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [
                    {
                        "name": "web_search",
                        "description": "Searches for information on DuckDuckGo with time-range and region filters to obtain relevant and up-to-date search results.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": {
                                    "type": "string",
                                    "description": "Specific search keywords or terms"
                                },
                                "max_results": {
                                    "type": "integer",
                                    "description": "Maximum number of search results to return (default: 15)"
                                },
                                "time_range": {
                                    "type": "string",
                                    "enum": ["day", "week", "month", "year"],
                                    "description": "Publication time filter: 'day' (past 24h), 'week' (past week), 'month' (past month), 'year' (past year)"
                                },
                                "region": {
                                    "type": "string",
                                    "description": "Language/country region code (e.g., 'us-en' for US/Global, 'id-id' for Indonesia, 'wt-wt' for no region filter)"
                                }
                            },
                            "required": ["query"]
                        }
                    },
                    {
                      "name": "fetch_web_content",
                      "description": "Fetches and parses webpage content, converting it into clean, sanitized Markdown format.",
                      "inputSchema": {
                        "type": "object",
                        "properties": {
                          "url": {
                            "type": "string",
                            "description": "Full webpage URL (http/https)"
                          }
                        },
                        "required": ["url"]
                      }
                    }
                ]
            }
        }),
        "tools/call" => {
            let tool_name = req.pointer("/params/name").and_then(|n| n.as_str()).unwrap_or("");
            let empty_map = serde_json::Map::new();
            let args = req.pointer("/params/arguments").and_then(|v| v.as_object()).unwrap_or(&empty_map);

            let result_text = match tool_name {
                "web_search" => {
                    let q = args.get("query").and_then(|v| v.as_str()).unwrap_or("");

                    // Flexible limit parsing (supports u64 and positive i64)
                    let limit = args
                        .get("max_results")
                        .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|i| i.max(1) as u64)))
                        .unwrap_or(15) as usize;

                    let time_range = args.get("time_range").and_then(|v| v.as_str());
                    let region = args.get("region").and_then(|v| v.as_str());

                    if q.trim().is_empty() {
                        "Error: Parameter 'query' cannot be empty.".to_string()
                    } else {
                        search_web(q, limit, time_range, region)
                    }
                }
                "fetch_web_content" => {
                    let u = args.get("url").and_then(|v| v.as_str()).unwrap_or("");

                    if u.trim().is_empty() {
                        "Error: Parameter 'url' cannot be empty.".to_string()
                    } else {
                        fetch_markdown(u)
                    }
                }
                _ => format!("Error: Tool '{}' not found.", tool_name),
            };

            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [
                        {
                            "type": "text",
                            "text": result_text
                        }
                    ]
                }
            })
        }
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": "Method not found"
            }
        }),
    };

    Some(response)
}
