//! Post-processing patches for pbjson generated code.
//!
//! pbjson wraps floats in `NumberDeserialize<_>` to accept both numbers and
//! strings per proto3 JSON spec. We remove this wrapper since we want strict
//! JSON number parsing for floats.

use regex::Regex;
use std::sync::LazyLock;

/// Float field patches for strict JSON number parsing
pub static FLOAT_PATCHES: LazyLock<Vec<FloatFieldPatch>> = LazyLock::new(|| {
    vec![
        FloatFieldPatch {
            field_name: "temperature",
        },
        FloatFieldPatch {
            field_name: "top_p",
        },
        FloatFieldPatch {
            field_name: "presence_penalty",
        },
        FloatFieldPatch {
            field_name: "frequency_penalty",
        },
    ]
});

pub struct FloatFieldPatch {
    pub field_name: &'static str,
}

/// Fix float deserialization by removing NumberDeserialize wrapper.
///
/// pbjson generates:
/// ```ignore
/// field__ =
///     map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?
///     .map(|x| x.0)
///     ;
/// ```
///
/// We replace with:
/// ```ignore
/// field__ = map_.next_value()?;
/// ```
pub fn fix_float_fields(content: &str, patches: &[FloatFieldPatch]) -> Result<String, String> {
    let mut result = content.to_string();

    for f in patches {
        // pbjson format:
        // field__ =
        //     map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
        // ;
        let pattern = format!(
            r#"(?s)({}__\s*=\s*)\n\s*map_\.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>\(\)\?\.map\(\|x\| x\.0\)\s*;"#,
            regex::escape(f.field_name)
        );
        let re = Regex::new(&pattern).map_err(|e| format!("Invalid regex: {}", e))?;

        // This might not match if already fixed or format different - that's OK
        if re.is_match(&result) {
            result = re
                .replace_all(&result, |caps: &regex::Captures| {
                    format!("{} map_.next_value()?;", &caps[1])
                })
                .to_string();
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fix_float_field() {
        // Actual pbjson format has = on one line, value on next with indentation
        let input = r#"
                        GeneratedField::Temperature => {
                            if temperature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("temperature"));
                            }
                            temperature__ =
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
"#;
        let patches = vec![FloatFieldPatch {
            field_name: "temperature",
        }];

        let result = fix_float_fields(input, &patches).unwrap();

        assert!(result.contains("temperature__ = map_.next_value()?;"));
        assert!(!result.contains("NumberDeserialize"));
    }

    #[test]
    fn test_fix_multiple_float_fields() {
        // Actual pbjson format
        let input = r#"
                            temperature__ =
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                            top_p__ =
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
"#;
        let patches = vec![
            FloatFieldPatch {
                field_name: "temperature",
            },
            FloatFieldPatch {
                field_name: "top_p",
            },
        ];

        let result = fix_float_fields(input, &patches).unwrap();

        assert!(result.contains("temperature__ = map_.next_value()?;"));
        assert!(result.contains("top_p__ = map_.next_value()?;"));
        assert!(!result.contains("NumberDeserialize"));
    }

    #[test]
    fn test_no_match_is_ok() {
        let input = "some unrelated content";
        let patches = vec![FloatFieldPatch {
            field_name: "temperature",
        }];

        let result = fix_float_fields(input, &patches).unwrap();
        assert_eq!(result, input);
    }
}
