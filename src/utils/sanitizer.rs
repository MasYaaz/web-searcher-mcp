pub fn decode_html_entities(input: &str) -> String {
    if !input.contains('&') {
        return input.to_string();
    }

    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '&' {
            let mut entity = String::new();
            let mut matched = false;

            while let Some(&next_c) = chars.peek() {
                if next_c == ';' {
                    chars.next();
                    let decoded = match entity.as_str() {
                        "nbsp" => " ",
                        "quot" => "\"",
                        "amp" => "&",
                        "lt" => "<",
                        "gt" => ">",
                        "#39" | "apos" => "'",
                        "mdash" => "—",
                        "ndash" => "–",
                        "bull" => "•",
                        "copy" => "©",
                        _ => {
                            output.push('&');
                            output.push_str(&entity);
                            output.push(';');
                            matched = true;
                            break;
                        }
                    };
                    output.push_str(decoded);
                    matched = true;
                    break;
                } else if next_c.is_alphanumeric() || next_c == '#' {
                    entity.push(chars.next().unwrap());
                    if entity.len() > 8 {
                        break;
                    }
                } else {
                    break;
                }
            }

            if !matched {
                output.push('&');
                output.push_str(&entity);
            }
        } else {
            output.push(c);
        }
    }
    output
}

pub fn strip_html_tags_safely(input: &str) -> String {
    if !input.contains('<') {
        return input.to_string();
    }

    let mut clean = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut inside_inline_code = false;

    while let Some(c) = chars.next() {
        if c == '`' {
            inside_inline_code = !inside_inline_code;
            clean.push(c);
            continue;
        }

        if c == '<' && !inside_inline_code {
            if let Some(&next) = chars.peek() {
                if next.is_alphabetic() || next == '/' || next == '!' {
                    let mut in_quote = None;

                    for end_c in chars.by_ref() {
                        if let Some(q) = in_quote {
                            if end_c == q {
                                in_quote = None;
                            }
                        } else if end_c == '"' || end_c == '\'' {
                            in_quote = Some(end_c);
                        } else if end_c == '>' {
                            break;
                        }
                    }
                    continue;
                }
            }
            clean.push(c);
        } else {
            clean.push(c);
        }
    }
    clean
}

pub fn remove_wiki_citations(input: &str) -> String {
    if !input.contains('[') {
        return input.to_string();
    }

    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '[' {
            let mut temp = String::new();
            let mut is_citation = false;

            while let Some(&next_c) = chars.peek() {
                if next_c == ']' {
                    chars.next();

                    if chars.peek() == Some(&'(') {
                        temp.push(']');
                        break;
                    }

                    let inner = temp.trim();
                    let inner_no_caret = inner.strip_prefix('^').unwrap_or(inner);

                    if inner_no_caret.chars().all(|ch| ch.is_ascii_digit() || ch == ',' || ch == ' ' || ch == '-')
                        || inner == "rujukan?"
                        || inner.starts_with("catatan")
                        || inner.starts_with("note")
                    {
                        is_citation = true;
                    } else {
                        temp.push(']');
                    }
                    break;
                } else if next_c == '\n' || temp.len() > 25 {
                    break;
                } else {
                    temp.push(chars.next().unwrap());
                }
            }

            if !is_citation {
                output.push('[');
                output.push_str(&temp);
            }
        } else {
            output.push(c);
        }
    }
    output
}

pub fn clean_markdown_content(raw_md: &str) -> String {
    let mut cleaned_lines = Vec::new();
    let mut in_code_block = false;

    for line in raw_md.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            cleaned_lines.push(trimmed.to_string());
            continue;
        }

        if in_code_block {
            cleaned_lines.push(line.to_string());
            continue;
        }

        // Filter noise, script/style leak, dan metadata
        if trimmed.contains("adsbygoogle")
            || trimmed.contains("googletag")
            || trimmed.contains("window.")
            || trimmed.contains("({});")
            || trimmed.starts_with("{ \"@context\"")
            || trimmed.starts_with("{\\\"@context\\\"")
            || trimmed.starts_with("@keyframes")
            || trimmed.starts_with("@media")
            || trimmed.contains("veaction=edit")
            || trimmed.contains("sunting sumber")
            || trimmed.contains("[sunting]")
            || trimmed.contains("[edit]")
            || trimmed.starts_with("Share:")
            || trimmed.starts_with("Bagikan:")
            || trimmed.starts_with("Follow us")
            || trimmed.starts_with("Cookie Policy")
            || trimmed.starts_with("Kebijakan Privasi")
            || trimmed.starts_with("Halaman ini terakhir diubah")
            || trimmed.starts_with("Daftar isi")
        {
            continue;
        }

        let tag_stripped = strip_html_tags_safely(trimmed);
        let decoded = decode_html_entities(&tag_stripped);
        let citation_cleaned = remove_wiki_citations(&decoded);
        let line_clean = citation_cleaned.trim().to_string();

        if line_clean.starts_with("[ ](")
            || line_clean.starts_with("[](")
            || (line_clean.starts_with("![") && line_clean.ends_with(')'))
            || (line_clean.starts_with('/') && (line_clean.ends_with(".jpg)") || line_clean.ends_with(".png)")))
        {
            continue;
        }

        if (line_clean.starts_with("* [") || line_clean.starts_with("- ["))
            && line_clean.ends_with(')')
            && line_clean.len() < 50
        {
            continue;
        }

        if !line_clean.is_empty() {
            cleaned_lines.push(line_clean);
        }
    }

    cleaned_lines.join("\n\n")
}
