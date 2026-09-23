mod engine;
mod mcp;
mod utils;

use mcp::handler::handle_rpc_request;
use std::io::{self, BufRead, Write};

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
