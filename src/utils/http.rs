use reqwest::blocking::Client;
use reqwest::header::{
    HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, COOKIE, REFERER, USER_AGENT,
};
use std::time::Duration;

pub const BROWSER_UA: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0";

pub fn build_http_client(timeout_secs: u64) -> Result<Client, String> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(BROWSER_UA));
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"),
    );
    headers.insert(
        ACCEPT_LANGUAGE,
        HeaderValue::from_static("id-ID,id;q=0.9,en-US;q=0.8,en;q=0.7"),
    );
    headers.insert(REFERER, HeaderValue::from_static("https://html.duckduckgo.com/"));
    headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("document"));
    headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("navigate"));
    headers.insert("Sec-Fetch-Site", HeaderValue::from_static("same-origin"));
    headers.insert("Sec-Fetch-User", HeaderValue::from_static("?1"));
    headers.insert(COOKIE, HeaderValue::from_static("kl=id-id"));

    Client::builder()
        .gzip(true)
        .default_headers(headers)
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("Gagal inisialisasi HTTP client: {}", e))
}

pub fn open_browser_for_captcha() {
    let ddg_html_url = "https://html.duckduckgo.com/html";

    if let Err(e) = open::that(ddg_html_url) {
        eprintln!("Gagal membuka browser secara otomatis: {}", e);
    }
}
