//! Narrow JSON string escape repair for malformed unicode escapes.
//!
//! This is intentionally not a generic "JSON repair" utility. It only handles
//! two cases that otherwise fail before Lingua can deserialize the document:
//! - truncated `\uXXXX` escape sequences inside JSON strings
//! - lone UTF-16 surrogate escapes inside JSON strings
//!
//! The repair strategy is conservative:
//! - scan the raw JSON bytes without changing document structure
//! - only touch `\u` escapes inside quoted strings
//! - replace unrecoverable escapes with `\uFFFD`
//! - return `None` if no repair was needed
//!
//! Valid surrogate pairs such as `\uD83D\uDE00` are preserved as-is.

/// Repair malformed JSON unicode escapes in-place at the byte level.
///
/// The returned bytes remain JSON text. Callers should retry normal JSON
/// deserialization on the repaired buffer and keep the original error path if
/// the payload is still invalid for some other reason.
pub fn repair_json_unicode_escapes(input: &[u8]) -> Option<Vec<u8>> {
    let mut repaired = Vec::new();
    let mut copy_from = 0usize;
    let mut in_string = false;
    let mut idx = 0usize;

    while idx < input.len() {
        match input[idx] {
            b'"' if !in_string => {
                in_string = true;
                idx += 1;
            }
            b'"' => {
                in_string = false;
                idx += 1;
            }
            b'\\' if in_string => {
                if idx + 1 >= input.len() || input[idx + 1] != b'u' {
                    idx += 2.min(input.len().saturating_sub(idx));
                    continue;
                }

                let escape_start = idx + 2;
                let mut escape_end = escape_start;
                while escape_end < input.len()
                    && escape_end < escape_start + 4
                    && input[escape_end].is_ascii_hexdigit()
                {
                    escape_end += 1;
                }

                if escape_end - escape_start < 4 {
                    repaired.extend_from_slice(&input[copy_from..idx]);
                    repaired.extend_from_slice(br#"\uFFFD"#);
                    copy_from = escape_end;
                    idx = escape_end;
                    continue;
                }

                let code_unit = parse_hex_code_unit(&input[escape_start..escape_start + 4]);
                let escape_end = idx + 6;

                if is_leading_surrogate(code_unit) {
                    if has_trailing_surrogate_escape(input, escape_end) {
                        idx += 12;
                        continue;
                    }

                    repaired.extend_from_slice(&input[copy_from..idx]);
                    repaired.extend_from_slice(br#"\uFFFD"#);
                    copy_from = escape_end;
                    idx = escape_end;
                    continue;
                }

                if is_trailing_surrogate(code_unit) {
                    repaired.extend_from_slice(&input[copy_from..idx]);
                    repaired.extend_from_slice(br#"\uFFFD"#);
                    copy_from = escape_end;
                    idx = escape_end;
                    continue;
                }

                idx = escape_end;
            }
            _ => {
                idx += 1;
            }
        }
    }

    if repaired.is_empty() {
        None
    } else {
        repaired.extend_from_slice(&input[copy_from..]);
        Some(repaired)
    }
}

fn parse_hex_code_unit(hex: &[u8]) -> u16 {
    hex.iter().fold(0u16, |acc, byte| {
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => 0,
        };
        (acc << 4) | u16::from(digit)
    })
}

fn has_trailing_surrogate_escape(input: &[u8], idx: usize) -> bool {
    if idx + 6 > input.len() || input[idx] != b'\\' || input[idx + 1] != b'u' {
        return false;
    }

    let hex = &input[idx + 2..idx + 6];
    if !hex.iter().all(u8::is_ascii_hexdigit) {
        return false;
    }

    is_trailing_surrogate(parse_hex_code_unit(hex))
}

fn is_leading_surrogate(code_unit: u16) -> bool {
    (0xD800..=0xDBFF).contains(&code_unit)
}

fn is_trailing_surrogate(code_unit: u16) -> bool {
    (0xDC00..=0xDFFF).contains(&code_unit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repairs_truncated_unicode_escape() {
        let repaired = repair_json_unicode_escapes(br#"{"x":"bad \uD83"}"#).unwrap();
        assert_eq!(repaired, br#"{"x":"bad \uFFFD"}"#);
    }

    #[test]
    fn repairs_lone_leading_surrogate() {
        let repaired = repair_json_unicode_escapes(br#"{"x":"bad \uD83D text"}"#).unwrap();
        assert_eq!(repaired, br#"{"x":"bad \uFFFD text"}"#);
    }

    #[test]
    fn repairs_lone_trailing_surrogate() {
        let repaired = repair_json_unicode_escapes(br#"{"x":"bad \uDE00 text"}"#).unwrap();
        assert_eq!(repaired, br#"{"x":"bad \uFFFD text"}"#);
    }

    #[test]
    fn preserves_valid_surrogate_pair() {
        assert!(repair_json_unicode_escapes(br#"{"x":"ok \uD83D\uDE00"}"#).is_none());
    }

    #[test]
    fn ignores_non_unicode_escapes_and_structure() {
        let input = br#"{"x":"quote: \" slash: \\ newline: \n"}"#;
        assert!(repair_json_unicode_escapes(input).is_none());
    }
}
