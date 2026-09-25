//! Module for fetching web content and parsing it into sanitized Markdown.
//!
//! This module handles SSRF validation, HTTP requests, Cloudflare bot-detection checks,
//! main content extraction using Readability with a DOM-stripping fallback,
//! HTML-to-Markdown parsing, and UTF-8 safe token limiting.

use crate::utils::http::build_http_client;
use crate::utils::sanitizer::clean_markdown_content;
use crate::utils::security::validate_public_url;
use readability::extractor;
use scraper::{Html, Selector};
use std::io::Cursor;
use url::Url;

/// Strips unwanted HTML tags (scripts, styles, ads, navigation, etc.) as a fallback
/// when Readability extraction fails or returns empty content.
///
/// # Arguments
/// * `raw_html` - The original raw HTML string.
///
/// # Returns
/// A cleaned HTML string with non-content elements removed.
fn fallback_dom_cleaner(raw_html: &str) -> String {
    let document = Html::parse_document(raw_html);
    let remove_selector = Selector::parse(
        "script, style, svg, noscript, iframe, header, footer, nav, aside, .ads, .advertisement",
    )
    .unwrap();

    let mut clean_html = raw_html.to_string();
    for el in document.select(&remove_selector) {
        clean_html = clean_html.replace(&el.html(), "");
    }
    clean_html
}

/// Fetches web content from a given URL and converts it into a sanitized Markdown string.
///
/// # Pipeline Steps:
/// 1. **SSRF Guard:** Validates that the URL is public and safe to request.
/// 2. **HTTP Client Creation:** Builds a reqwest HTTP client with a 15-second timeout.
/// 3. **Network Request:** Sends a GET request and verifies successful HTTP status.
/// 4. **Bot Challenge Detection:** Checks for Cloudflare / anti-bot challenge pages.
/// 5. **Extraction:** Attempts Mozilla Readability extraction; falls back to DOM stripping if extraction fails.
/// 6. **Markdown Conversion & Sanitization:** Converts HTML to Markdown and sanitizes layout noise.
/// 7. **Character Truncation:** Safely truncates the resulting Markdown at 12,000 UTF-8 characters.
///
/// # Arguments
/// * `url` - The target URL string to fetch.
///
/// # Returns
/// A `String` containing the extracted Markdown or an error message if any step fails.
pub fn fetch_markdown(url: &str) -> String {
    // 1. SSRF Guard
    if let Err(err_msg) = validate_public_url(url) {
        return format!("Security Error: {}", err_msg);
    }

    // 2. HTTP Client
    let client = match build_http_client(15) {
        Ok(c) => c,
        Err(e) => return e,
    };

    // 3. Network Request
    let resp = match client.get(url).send() {
        Ok(res) => res,
        Err(e) => return format!("Failed to fetch URL: {}", e),
    };

    let status = resp.status();
    if !status.is_success() {
        return format!("Web server responded with error status: {}", status);
    }

    let bytes = match resp.bytes() {
        Ok(b) => b,
        Err(e) => return format!("Failed to read stream body: {}", e),
    };

    let html = String::from_utf8_lossy(&bytes);

    // 4. Bot / Cloudflare Detection
    if html.contains("cf-mitigated")
        || html.contains("Just a moment...")
        || html.contains("Enable JavaScript and cookies to continue")
        || html.contains("Checking your browser before accessing")
    {
        return "Webpage is protected by Cloudflare / Bot Challenge and cannot be directly fetched.".to_string();
    }

    // 5. Extraction via Readability (Fallback to DOM stripping if it fails)
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

    // 6. Token / Character Limiting (UTF-8 Safe)
    let char_limit = 12000;
    let mut chars = md.chars();
    let truncated: String = chars.by_ref().take(char_limit).collect();

    if chars.next().is_some() {
        format!("{}\n\n*(Text truncated as it exceeded maximum token limit)*", truncated)
    } else {
        truncated
    }
}
