//! Main entry point for the `mcp-web-agent` server.
//!
//! This binary reads line-delimited JSON-RPC 2.0 requests from Standard Input (`stdin`),
//! dispatches them to the MCP handler module, and writes non-empty responses back to
//! Standard Output (`stdout`).

mod engine;
mod mcp;
mod utils;

use mcp::handler::handle_rpc_request;
use std::io::{self, BufRead, Write};

/// Runs the main STDIO loop for processing JSON-RPC messages.
///
/// Continuously locks `stdin`, reads incoming lines, passes non-empty lines to `handle_rpc_request`,
/// serializes the resulting response into JSON, and flushes it to `stdout`.
fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let Ok(req_text) = line else { break };
        if req_text.trim().is_empty() {
            continue;
        }

        if let Some(response) = handle_rpc_request(&req_text) {
            let mut out = serde_json::to_string(&response).unwrap();
            out.push('\n');
            let _ = stdout.write_all(out.as_bytes());
            let _ = stdout.flush();
        }
    }
}
