//! Module for building configured HTTP clients with custom headers and timeouts.
//!
//! This module provides utilities to instantiate a blocking `reqwest::blocking::Client`
//! populated with realistic browser headers (User-Agent, Accept, Accept-Language, Sec-Fetch, Cookies)
//! to avoid basic bot-detection blocks during web scraping.

use reqwest::blocking::Client;
use reqwest::header::{
    HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, COOKIE, REFERER, USER_AGENT,
};
use std::time::Duration;

/// Default User-Agent string simulating a modern Firefox browser on Linux.
pub const BROWSER_UA: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0";

/// Builds a pre-configured blocking HTTP client with standard browser headers and gzip support.
///
/// Configures headers including `USER_AGENT`, `ACCEPT`, `ACCEPT_LANGUAGE`, `REFERER`, `COOKIE`,
/// and `Sec-Fetch-*` navigation attributes to mimic standard browser traffic.
///
/// # Arguments
/// * `timeout_secs` - Maximum timeout duration for network requests in seconds.
///
/// # Returns
/// * `Ok(Client)` - An initialized `reqwest::blocking::Client` ready for network calls.
/// * `Err(String)` - An error message if client initialization fails.
pub fn build_http_client(timeout_secs: u64) -> Result<Client, String> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(BROWSER_UA));
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"),
    );
    headers.insert(
        ACCEPT_LANGUAGE,
        HeaderValue::from_static("en-US,en;q=0.9,id-ID;q=0.8,id;q=0.7"),
    );
    headers.insert(REFERER, HeaderValue::from_static("https://html.duckduckgo.com/"));
    headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("document"));
    headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("navigate"));
    headers.insert("Sec-Fetch-Site", HeaderValue::from_static("same-origin"));
    headers.insert("Sec-Fetch-User", HeaderValue::from_static("?1"));
    headers.insert(COOKIE, HeaderValue::from_static("kl=us-en"));

    Client::builder()
        .gzip(true)
        .default_headers(headers)
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("Failed to initialize HTTP client: {}", e))
}
