//! Splitting macro arguments at top-level commas, and reading field keys from them.

/// Splits the inside of `name!(...)` (or a bare argument list) at top-level commas, keeping
/// string literals intact. Empty segments are dropped.
pub fn split_top_level(args: &str) -> Vec<String> {
    let mut segments = vec![String::new()];
    let mut depth = 0_i32;
    let mut chars = args.chars();
    while let Some(ch) = chars.next() {
        let Some(current) = segments.last_mut() else {
            break;
        };
        match ch {
            '"' => {
                current.push(ch);
                push_string_literal(&mut chars, current);
            }
            '(' | '[' | '{' => {
                depth += 1;
                current.push(ch);
            }
            ')' | ']' | '}' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => segments.push(String::new()),
            _ => current.push(ch),
        }
    }
    segments
        .into_iter()
        .map(|segment| segment.trim().to_owned())
        .filter(|segment| !segment.is_empty())
        .collect()
}

/// Copies the rest of a string literal (after its opening quote) into `out`.
fn push_string_literal(chars: &mut std::str::Chars<'_>, out: &mut String) {
    while let Some(next) = chars.next() {
        out.push(next);
        if next == '\\' {
            out.extend(chars.next());
        } else if next == '"' {
            break;
        }
    }
}

/// The text between the outermost brackets of a macro call snippet: `info!(a, b)` gives `a, b`.
pub fn inner(snippet: &str) -> &str {
    let Some(open) = snippet.find(['(', '[', '{']) else {
        return "";
    };
    snippet
        .get(open + 1..snippet.len().saturating_sub(1))
        .unwrap_or_default()
}

pub fn is_literal(segment: &str) -> bool {
    segment.starts_with('"') && segment.ends_with('"') && segment.len() >= 2
}

/// The field key of one argument (`key = value`, `?key`, `%key`, `key`), if it is a field.
pub fn field_key(segment: &str) -> Option<String> {
    let segment = segment.trim().trim_start_matches(['?', '%']).trim();
    if segment.starts_with('"') {
        return None;
    }
    let key = match segment.find('=') {
        Some(at) if !segment[at..].starts_with("==") => &segment[..at],
        _ => segment,
    };
    let key: String = key.split_whitespace().collect();
    let plain = key
        .chars()
        .all(|ch| ch.is_alphanumeric() || ch == '_' || ch == '.');
    (plain && !key.is_empty() && !key.starts_with(|ch: char| ch.is_ascii_digit())).then_some(key)
}

/// `snake_case`, or lowercase dotted segments (`http.request.method`).
pub fn is_valid_key(key: &str) -> bool {
    key.split('.').all(|part| {
        let mut chars = part.chars();
        chars
            .next()
            .is_some_and(|first| first.is_ascii_lowercase() || first == '_')
            && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_and_reads_keys() {
        let segments = split_top_level(r#"item_id = %id, ?name, "msg {}", f(a, b)"#);
        assert_eq!(segments.len(), 4, "four arguments");
        assert_eq!(
            field_key(&segments[0]).as_deref(),
            Some("item_id"),
            "assignment"
        );
        assert_eq!(
            field_key(&segments[1]).as_deref(),
            Some("name"),
            "shorthand"
        );
        assert_eq!(field_key("Level::INFO"), None, "level is not a field");
        assert_eq!(field_key("target: \"x\""), None, "target is not a field");
        assert!(
            is_valid_key("http.request.method") && is_valid_key("item_id"),
            "valid styles"
        );
        assert!(
            !is_valid_key("itemId") && !is_valid_key("Item.id"),
            "invalid styles"
        );
    }
}
