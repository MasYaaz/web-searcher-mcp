use crate::utils::http::build_http_client;
use crate::utils::sanitizer::clean_markdown_content;
use crate::utils::security::validate_public_url;
use readability::extractor;
use scraper::{Html, Selector};
use std::io::Cursor;
use url::Url;

fn fallback_dom_cleaner(raw_html: &str) -> String {
    let document = Html::parse_document(raw_html);
    let remove_selector = Selector::parse(
        "script, style, svg, noscript, iframe, header, footer, nav, aside, .ads, .advertisement"
    ).unwrap();

    let mut clean_html = raw_html.to_string();
    for el in document.select(&remove_selector) {
        clean_html = clean_html.replace(&el.html(), "");
    }
    clean_html
}

pub fn fetch_markdown(url: &str) -> String {
    // 1. SSRF Guard
    if let Err(err_msg) = validate_public_url(url) {
        return format!("Error Keamanan: {}", err_msg);
    }

    // 2. HTTP Client
    let client = match build_http_client(15) {
        Ok(c) => c,
        Err(e) => return e,
    };

    // 3. Request Jaringan
    let resp = match client.get(url).send() {
        Ok(res) => res,
        Err(e) => return format!("Gagal memuat URL: {}", e),
    };

    let status = resp.status();
    if !status.is_success() {
        return format!("Server web merespons dengan status error: {}", status);
    }

    let bytes = match resp.bytes() {
        Ok(b) => b,
        Err(e) => return format!("Gagal membaca stream body: {}", e),
    };

    let html = String::from_utf8_lossy(&bytes);

    // 4. Deteksi Bot / Cloudflare
    if html.contains("cf-mitigated")
        || html.contains("Just a moment...")
        || html.contains("Enable JavaScript and cookies to continue")
        || html.contains("Checking your browser before accessing")
    {
        return "Halaman web dilindungi oleh Cloudflare / Bot Challenge dan tidak dapat dibaca secara langsung.".to_string();
    }

    // 5. Ekstraksi dengan Readability (Fallback ke DOM stripping jika gagal)
    let md = if let Ok(parsed_url) = Url::parse(url) {
        let mut cursor = Cursor::new(html.as_bytes());
        match extractor::extract(&mut cursor, &parsed_url) {
            Ok(product) if !product.content.trim().is_empty() => {
                let raw_body_md = html2md::parse_html(&product.content);
                let cleaned_body = clean_markdown_content(&raw_body_md);
                if product.title.trim().is_empty() {
                    cleaned_body
                } else {
                    format!("# {}\n\n{}", product.title.trim(), cleaned_body)
                }
            }
            _ => {
                let fallback_html = fallback_dom_cleaner(&html);
                let raw_md = html2md::parse_html(&fallback_html);
                clean_markdown_content(&raw_md)
            }
        }
    } else {
        let fallback_html = fallback_dom_cleaner(&html);
        let raw_md = html2md::parse_html(&fallback_html);
        clean_markdown_content(&raw_md)
    };

    // 6. Token Char Limiting (UTF-8 Safe)
    let char_limit = 12000;
    let mut chars = md.chars();
    let truncated: String = chars.by_ref().take(char_limit).collect();

    if chars.next().is_some() {
        format!("{}\n\n*(Teks dipotong karena melebihi batas token)*", truncated)
    } else {
        truncated
    }
}
