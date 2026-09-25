//! Module for scraping DuckDuckGo HTML search results and Instant Answers.
//!
//! This module handles DuckDuckGo URL sanitization, bang quoting, CAPTCHA/challenge checks,
//! zero-click abstract extraction, multi-page result pagination, and Markdown formatting.

use crate::utils::http::build_http_client;
use crate::utils::sanitizer::{decode_html_entities, strip_html_tags_safely};
use scraper::{Html, Selector};
use std::thread;
use std::time::Duration;

/// Base URL for DuckDuckGo HTML search endpoint.
const DDG_HTML_URL: &str = "https://html.duckduckgo.com/html/";

/// Cleans and extracts the actual target URL from a raw DuckDuckGo redirect link.
///
/// DuckDuckGo wraps search result links with redirect paths (e.g., `//duckduckgo.com/l/?uddg=...`).
/// This function parses `uddg` parameters and performs form-urlencoded decoding.
///
/// # Arguments
/// * `raw_link` - The raw href string extracted from the search result element.
///
/// # Returns
/// A cleaned, fully-qualified URL string.
pub fn clean_ddg_url(raw_link: &str) -> String {
    if let Some(pos) = raw_link.find("uddg=") {
        let encoded_part = &raw_link[pos + 5..];
        let clean_encoded = encoded_part.split('&').next().unwrap_or(encoded_part);

        // `url::form_urlencoded::parse` automatically performs URL decoding
        let decoded = url::form_urlencoded::parse(clean_encoded.as_bytes())
            .map(|(key, _)| key)
            .collect::<String>();

        if !decoded.is_empty() {
            return decoded;
        }
    }

    if raw_link.starts_with("//") {
        format!("https:{}", raw_link)
    } else {
        raw_link.to_string()
    }
}

/// Escapes DuckDuckGo search bangs (e.g., `!wiki`) by wrapping them in single quotes
/// to prevent DuckDuckGo from executing accidental redirects instead of text searches.
///
/// # Arguments
/// * `query` - The input search query string.
///
/// # Returns
/// A query string with bang terms quoted.
pub fn quote_ddg_bangs(query: &str) -> String {
    let mut parts = Vec::new();
    for word in query.split_whitespace() {
        if word.starts_with('!') && word.len() > 1 {
            parts.push(format!("'{}'", word));
        } else {
            parts.push(word.to_string());
        }
    }
    parts.join(" ")
}

/// Checks if the response HTML contains a DuckDuckGo CAPTCHA or bot challenge modal.
fn check_is_captcha(document: &Html) -> bool {
    let challenge_selector = Selector::parse("form#challenge-form, .captcha-modal").unwrap();
    document.select(&challenge_selector).next().is_some()
}

/// Extracts Instant Answer snippets (zero-click abstract) from DuckDuckGo search results if available.
fn extract_zero_click(document: &Html) -> Option<String> {
    let zero_click_selector = Selector::parse("#zero_click_abstract").unwrap();
    let zc_el = document.select(&zero_click_selector).next()?;
    let zc_text = decode_html_entities(&strip_html_tags_safely(&zc_el.text().collect::<String>()))
        .trim()
        .to_string();

    if !zc_text.is_empty()
        && !zc_text.contains("Your IP address is")
        && !zc_text.contains("Your user agent:")
    {
        Some(format!("> **Instant Answer:**\n> {}\n\n---\n\n", zc_text))
    } else {
        None
    }
}

/// Extracts hidden form parameters required to request subsequent result pages.
fn extract_next_page_payload(document: &Html) -> Option<Vec<(String, String)>> {
    let form_selector = Selector::parse(".nav-link form, form").unwrap();
    let input_selector = Selector::parse("input").unwrap();

    for form in document.select(&form_selector) {
        let inputs: Vec<_> = form.select(&input_selector).collect();
        let has_next = inputs.iter().any(|i| i.value().attr("name") == Some("nextParams"));
        let has_s = inputs.iter().any(|i| i.value().attr("name") == Some("s"));

        if has_next && has_s {
            let mut form_data = Vec::new();
            for input in inputs {
                if let Some(name) = input.value().attr("name") {
                    if input.value().attr("type") != Some("submit") {
                        let val = input.value().attr("value").unwrap_or("");
                        form_data.push((name.to_string(), val.to_string()));
                    }
                }
            }
            if !form_data.is_empty() {
                return Some(form_data);
            }
        }
    }
    None
}

/// Parses search result items from a single page HTML document and formats them into Markdown headers.
fn parse_search_results(
    document: &Html,
    limit: usize,
    current_count: &mut usize,
) -> (String, usize) {
    let result_selector = Selector::parse("#links .web-result, .web-result, .result").unwrap();
    let title_selector = Selector::parse("h2 a, a.result__a").unwrap();
    let snippet_selector = Selector::parse("a.result__snippet, .result__snippet").unwrap();

    let mut output = String::new();
    let mut page_items = 0;

    for element in document.select(&result_selector) {
        let class_attr = element.value().attr("class").unwrap_or("");
        if class_attr.contains("result--ad") {
            continue;
        }

        if let Some(link_el) = element.select(&title_selector).next() {
            let raw_title = link_el.text().collect::<String>();
            let title = decode_html_entities(&strip_html_tags_safely(&raw_title));
            let raw_href = link_el.value().attr("href").unwrap_or("");

            let raw_snippet = element
                .select(&snippet_selector)
                .next()
                .map(|s| s.text().collect::<String>())
                .unwrap_or_default();
            let snippet = decode_html_entities(&strip_html_tags_safely(&raw_snippet));

            if !title.trim().is_empty() && !raw_href.is_empty() {
                let full_url = clean_ddg_url(raw_href);
                let clean_snippet = if snippet.trim().is_empty() {
                    "*(No text snippet available)*".to_string()
                } else {
                    snippet.trim().to_string()
                };

                *current_count += 1;
                page_items += 1;

                output.push_str(&format!(
                    "### [{}] [{}]({})\n{}\n\n",
                    *current_count,
                    title.trim(),
                    full_url,
                    clean_snippet
                ));

                if *current_count >= limit {
                    break;
                }
            }
        }
    }

    (output, page_items)
}

/// Maps input time range strings to DuckDuckGo date filter parameters (`df`).
fn map_time_range(range: &str) -> &'static str {
    match range.trim().to_lowercase().as_str() {
        "d" | "day" | "hari" => "d",
        "w" | "week" | "minggu" => "w",
        "m" | "month" | "bulan" => "m",
        "y" | "year" | "tahun" => "y",
        _ => "",
    }
}

/// Performs a web search on DuckDuckGo and formats the result as Markdown.
///
/// Handles search pagination, CAPTCHA protection checks, time/region filtering,
/// and query length validation.
///
/// # Arguments
/// * `raw_query` - The search query string.
/// * `limit` - Maximum number of search results to retrieve.
/// * `time_range` - Optional time filter (`"d"`, `"w"`, `"m"`, `"y"`).
/// * `region` - Optional regional location code (defaults to `"id-id"`).
///
/// # Returns
/// A `String` containing the Markdown-formatted search results or an error message.
pub fn search_web(
    raw_query: &str,
    limit: usize,
    time_range: Option<&str>,
    region: Option<&str>,
) -> String {
    let query = quote_ddg_bangs(raw_query.trim());
    if query.len() >= 500 {
        return "Error: Search query exceeds DuckDuckGo's 499 character limit.".to_string();
    }

    let client = match build_http_client(12) {
        Ok(c) => c,
        Err(e) => return e,
    };

    let df_param = time_range.map(map_time_range).unwrap_or("");
    let region_param = region.unwrap_or("id-id");

    let mut output = String::new();
    let mut count = 0;
    let mut is_first_page = true;
    let mut next_payload: Option<Vec<(String, String)>> = None;

    while count < limit {
        let payload = if is_first_page {
            let mut p = vec![
                ("q".to_string(), query.clone()),
                ("b".to_string(), "".to_string()),
                ("kl".to_string(), region_param.to_string()),
            ];
            if !df_param.is_empty() {
                p.push(("df".to_string(), df_param.to_string()));
            }
            p
        } else {
            match next_payload.take() {
                Some(p) => p,
                None => break,
            }
        };

        let resp = client.post(DDG_HTML_URL).form(&payload).send();
        let bytes = match resp {
            Ok(res) => res.bytes().unwrap_or_default(),
            Err(e) => {
                if count == 0 {
                    return format!("Failed to connect to DuckDuckGo: {}", e);
                }
                break;
            }
        };

        let html_content = String::from_utf8_lossy(&bytes).to_string();
        let document = Html::parse_document(&html_content);

        // Check for CAPTCHA or Bot Challenge
        if check_is_captcha(&document) {
            if count == 0 {
                return "Search request was blocked by DuckDuckGo security protection (CAPTCHA/Bot Challenge).".to_string();
            }
            break;
        }

        if is_first_page {
            if let Some(zc_snippet) = extract_zero_click(&document) {
                output.push_str(&zc_snippet);
            }
            is_first_page = false;
        }

        let (page_output, page_items) = parse_search_results(&document, limit, &mut count);
        output.push_str(&page_output);

        if page_items == 0 || count >= limit {
            break;
        }

        next_payload = extract_next_page_payload(&document);
        if next_payload.is_none() {
            break;
        }

        thread::sleep(Duration::from_millis(350));
    }

    if output.is_empty() {
        "No search results found.".to_string()
    } else {
        output
    }
}
