use crate::engine::fetch::fetch_markdown;
use crate::engine::search::search_web;
use serde_json::{json, Value};

pub fn handle_rpc_request(req_text: &str) -> Option<Value> {
    let req: Value = serde_json::from_str(req_text).ok()?;
    let id = req.get("id").cloned();
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");

    // Abaikan semua JSON-RPC notification (tidak memiliki ID atau berawalan notifications/)
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
                        "description": "Mencari informasi di DuckDuckGo dengan filter rentang waktu dan wilayah untuk mendapatkan informasi yang relevan dan terkini.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": {
                                    "type": "string",
                                    "description": "Kata kunci pencarian spesifik"
                                },
                                "max_results": {
                                    "type": "integer",
                                    "description": "Jumlah maksimal tautan yang dikembalikan (default: 15)"
                                },
                                "time_range": {
                                    "type": "string",
                                    "enum": ["day", "week", "month", "year"],
                                    "description": "Filter publikasi: 'day' (24 jam terakhir), 'week' (1 minggu), 'month' (1 bulan), 'year' (1 tahun)"
                                },
                                "region": {
                                    "type": "string",
                                    "description": "Kode wilayah bahasa/negara (contoh: 'id-id' untuk Indonesia, 'us-en' untuk US/Global, 'wt-wt' tanpa filter wilayah)"
                                }
                            },
                            "required": ["query"]
                        }
                    },
                    {
                      "name": "fetch_web_content",
                      "description": "Membaca konten halaman web ke format Markdown bersih. Berikan parameter 'query' untuk menyaring bagian teks yang relevan.",
                      "inputSchema": {
                        "type": "object",
                        "properties": {
                          "url": {
                            "type": "string",
                            "description": "URL lengkap halaman web (http/https)"
                          },
                          "query": {
                            "type": "string",
                            "description": "Topik spesifik untuk menyaring teks artikel"
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

                    // Parsing fleksibel (mendukung integer u64 maupun i64)
                    let limit = args
                        .get("max_results")
                        .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|i| i.max(1) as u64)))
                        .unwrap_or(15) as usize;

                    let time_range = args.get("time_range").and_then(|v| v.as_str());
                    let region = args.get("region").and_then(|v| v.as_str());

                    if q.trim().is_empty() {
                        "Error: Parameter 'query' tidak boleh kosong.".to_string()
                    } else {
                        search_web(q, limit, time_range, region)
                    }
                }
                "fetch_web_content" => {
                    let u = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
                    let q = args.get("query").and_then(|v| v.as_str());

                    if u.trim().is_empty() {
                        "Error: Parameter 'url' tidak boleh kosong.".to_string()
                    } else {
                        fetch_markdown(u, q)
                    }
                }
                _ => format!("Error: Tool '{}' tidak ditemukan.", tool_name),
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
