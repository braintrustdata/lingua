//! Lint test: ensures no struct/enum derives Debug while containing secret fields.
//! If this test fails, either:
//!   1. Add a manual `impl Debug` that prints `"[REDACTED]"` for the sensitive field, or
//!   2. Wrap the field in a type whose Debug impl redacts (e.g. `secrecy::SecretString`).

use std::fs;
use std::path::{Path, PathBuf};

const SENSITIVE_FIELD_NAMES: &[&str] = &[
    "token",
    "auth_token",
    "api_key",
    "secret",
    "secret_key",
    "secret_access_key",
    "session_token",
    "service_token",
    "access_key",
    "access_key_id",
    "access_token",
    "bearer_token",
    "password",
];

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path
                .file_name()
                .is_some_and(|n| n == "target" || n == "examples")
            {
                continue;
            }
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            // Skip generated files — they contain logprob `token` fields (text tokens, not secrets)
            if path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().contains("generated"))
            {
                continue;
            }
            out.push(path);
        }
    }
}

fn has_manual_debug_impl(source: &str, type_name: &str) -> bool {
    let pattern = format!("impl std::fmt::Debug for {type_name}");
    let pattern_short = format!("impl Debug for {type_name}");
    source.contains(&pattern) || source.contains(&pattern_short)
}

fn is_sensitive_field(line: &str) -> Option<&'static str> {
    let trimmed = line.trim();
    if trimmed.starts_with("//") {
        return None;
    }
    for &name in SENSITIVE_FIELD_NAMES {
        let patterns = [format!("{name}:"), format!("{name} :")];
        for pat in &patterns {
            if let Some(before) = trimmed.find(pat.as_str()) {
                if before == 0 {
                    return Some(name);
                }
                let prev_char = trimmed.as_bytes()[before - 1];
                if prev_char == b' ' || prev_char == b'\t' {
                    return Some(name);
                }
            }
        }
    }
    None
}

fn check_file(path: &Path, violations: &mut Vec<String>) {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return,
    };

    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        if !(line.contains("#[derive(") && line.contains("Debug")) {
            i += 1;
            continue;
        }

        let mut j = i + 1;
        let mut type_name = None;
        while j < lines.len() && j <= i + 5 {
            let trimmed = lines[j].trim();
            for keyword in &["struct ", "enum "] {
                if let Some(pos) = trimmed.find(keyword) {
                    let after = &trimmed[pos + keyword.len()..];
                    let name: String = after
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    if !name.is_empty() {
                        type_name = Some(name);
                    }
                }
            }
            if type_name.is_some() {
                break;
            }
            j += 1;
        }

        let type_name = match type_name {
            Some(n) => n,
            None => {
                i += 1;
                continue;
            }
        };

        if has_manual_debug_impl(&source, &type_name) {
            i = j + 1;
            continue;
        }

        let mut brace_depth = 0i32;
        let mut found_open = false;
        let mut body_lines = Vec::new();
        let mut k = j;
        while k < lines.len() {
            for ch in lines[k].chars() {
                if ch == '{' {
                    brace_depth += 1;
                    found_open = true;
                } else if ch == '}' {
                    brace_depth -= 1;
                }
            }
            body_lines.push(lines[k]);
            if found_open && brace_depth <= 0 {
                break;
            }
            k += 1;
        }

        let mut sensitive_found = Vec::new();
        for body_line in &body_lines {
            if let Some(field_name) = is_sensitive_field(body_line) {
                if !sensitive_found.contains(&field_name) {
                    sensitive_found.push(field_name);
                }
            }
        }

        if !sensitive_found.is_empty() {
            violations.push(format!(
                "{}:{}: `{}` derives Debug with sensitive field(s): {}",
                path.display(),
                i + 1,
                type_name,
                sensitive_found.join(", "),
            ));
        }

        i = k + 1;
    }
}

#[test]
fn no_derive_debug_on_secret_fields() {
    // Scan the entire lingua workspace
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("could not find lingua workspace root");

    let mut rs_files = Vec::new();
    collect_rs_files(workspace_root, &mut rs_files);

    let mut violations = Vec::new();
    for file in &rs_files {
        check_file(file, &mut violations);
    }

    if !violations.is_empty() {
        let msg = violations.join("\n  ");
        panic!(
            "\n\nFound structs/enums deriving Debug with sensitive fields:\n\n  {msg}\n\n\
             Fix: add a manual `impl Debug` that prints \"[REDACTED]\" for the sensitive field.\n"
        );
    }
}
