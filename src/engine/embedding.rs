use crate::utils::http::build_http_client;
use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::env;

const DEFAULT_LLAMA_EMBED_URL: &str = "http://127.0.0.1:8081";

fn get_embed_base_url() -> String {
    env::var("LLAMA_EMBED_URL").unwrap_or_else(|_| DEFAULT_LLAMA_EMBED_URL.to_string())
}

pub fn get_llama_embedding(client: &Client, text: &str) -> Result<Vec<f32>, String> {
    let base_url = get_embed_base_url();
    let base_trimmed = base_url.trim_end_matches('/');

    let native_url = format!("{}/embedding", base_trimmed);
    let native_body = json!({ "content": text });

    if let Ok(res) = client.post(&native_url).json(&native_body).send() {
        if let Ok(val) = res.json::<Value>() {
            if let Some(arr) = val.get("embedding").and_then(|v| v.as_array()) {
                let vec: Vec<f32> = arr.iter().filter_map(|x| x.as_f64().map(|n| n as f32)).collect();
                if !vec.is_empty() {
                    return Ok(vec);
                }
            }
        }
    }

    let openai_url = format!("{}/v1/embeddings", base_trimmed);
    let openai_body = json!({ "input": text });

    if let Ok(res) = client.post(&openai_url).json(&openai_body).send() {
        if let Ok(val) = res.json::<Value>() {
            if let Some(arr) = val.pointer("/data/0/embedding").and_then(|v| v.as_array()) {
                let vec: Vec<f32> = arr.iter().filter_map(|x| x.as_f64().map(|n| n as f32)).collect();
                if !vec.is_empty() {
                    return Ok(vec);
                }
            }
        }
    }

    Err("llama-server embedding offline".to_string())
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for i in 0..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

pub fn chunk_markdown(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    let mut current = String::new();

    for p in paragraphs {
        let trimmed = p.trim();
        if trimmed.is_empty() {
            continue;
        }

        if current.len() + trimmed.len() > chunk_size && !current.is_empty() {
            chunks.push(current.clone());
            if current.len() > overlap {
                let start = current.len() - overlap;
                current = current[start..].to_string();
            } else {
                current.clear();
            }
        }

        if !current.is_empty() {
            current.push_str("\n\n");
        }
        current.push_str(trimmed);
    }

    if !current.trim().is_empty() {
        chunks.push(current);
    }

    chunks
}

pub fn semantic_filter_content(query: &str, raw_md: &str, top_k: usize) -> String {
    let client = match build_http_client(4) {
        Ok(c) => c,
        Err(_) => return raw_md.to_string(),
    };

    let query_vec = match get_llama_embedding(&client, query) {
        Ok(v) => v,
        Err(_) => {
            return if raw_md.len() > 12000 {
                format!(
                    "{}\n\n*(llama-server embedding offline: menampilkan teks awal)*",
                    &raw_md[..12000]
                )
            } else {
                raw_md.to_string()
            };
        }
    };

    let chunks = chunk_markdown(raw_md, 700, 150);
    if chunks.is_empty() {
        return raw_md.to_string();
    }

    let mut scored: Vec<(f32, String)> = Vec::new();
    for chunk in chunks {
        if let Ok(c_vec) = get_llama_embedding(&client, &chunk) {
            let score = cosine_similarity(&query_vec, &c_vec);
            scored.push((score, chunk));
        }
    }

    if scored.is_empty() {
        return raw_md.to_string();
    }

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut output = format!(
        "*Hasil ekstraksi semantik (Llama.cpp Embedding) untuk: \"{}\"*\n\n",
        query
    );
    for (score, chunk) in scored.into_iter().take(top_k) {
        output.push_str(&format!(
            "> **Relevansi: {:.2}%**\n{}\n\n---\n\n",
            score * 100.0,
            chunk
        ));
    }

    output
}
