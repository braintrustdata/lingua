//! Proto file patching for Google AI types.
//!
//! Patches the .proto file before compilation to add fields/enums that
//! Google's API uses but aren't in official protos yet.
//!
//! DELETE when googleapis adds these upstream.
//! Check: https://github.com/googleapis/googleapis/blob/master/google/ai/generativelanguage/v1beta/generative_service.proto

use regex::Regex;
use std::sync::LazyLock;

/// Proto patches for Google AI types
pub static GOOGLE_TYPE_PATCHES: LazyLock<ProtoPatches> = LazyLock::new(|| ProtoPatches {
    enums: vec![EnumPatch {
        name: "ThinkingLevel",
        insert_before_message: "ThinkingConfig",
        variants: &[
            ("THINKING_LEVEL_UNSPECIFIED", 0),
            ("LOW", 1),
            ("MEDIUM", 2),
            ("HIGH", 3),
            ("MINIMAL", 4),
        ],
    }],
    fields: vec![
        FieldPatch {
            message: "ThinkingConfig",
            field_name: "thinking_level",
            field_type: "ThinkingLevel",
            tag: 3,
        },
        FieldPatch {
            message: "ImageConfig",
            field_name: "image_size",
            field_type: "string",
            tag: 2,
        },
    ],
});

/// Declarative patches for proto files.
/// Adding a new field is just adding a FieldPatch entry.
pub struct ProtoPatches {
    pub enums: Vec<EnumPatch>,
    pub fields: Vec<FieldPatch>,
}

pub struct EnumPatch {
    pub name: &'static str,
    pub insert_before_message: &'static str,
    pub variants: &'static [(&'static str, i32)],
}

pub struct FieldPatch {
    pub message: &'static str,
    pub field_name: &'static str,
    pub field_type: &'static str, // "string", "int32", or enum name
    pub tag: i32,
}

impl ProtoPatches {
    pub fn apply(&self, proto_content: &str) -> Result<String, String> {
        let mut content = proto_content.to_string();

        // Insert enums before their target messages
        for e in &self.enums {
            content = self.insert_enum(&content, e)?;
        }

        // Insert fields into messages
        for f in &self.fields {
            content = self.insert_field(&content, f)?;
        }

        // Validate
        self.validate(&content)?;

        Ok(content)
    }

    fn insert_enum(&self, content: &str, e: &EnumPatch) -> Result<String, String> {
        // Find "message ThinkingConfig {" and insert enum before it
        let pattern = format!(
            r"(message\s+{}\s*\{{)",
            regex::escape(e.insert_before_message)
        );
        let re = Regex::new(&pattern).map_err(|e| format!("Invalid regex: {}", e))?;

        if !re.is_match(content) {
            return Err(format!(
                "Could not find message '{}' to insert enum '{}' before",
                e.insert_before_message, e.name
            ));
        }

        let variants = e
            .variants
            .iter()
            .map(|(name, val)| format!("  {} = {};", name, val))
            .collect::<Vec<_>>()
            .join("\n");

        let enum_code = format!(
            "// LINGUA PATCH - DELETE when googleapis adds upstream\nenum {} {{\n{}\n}}\n\n",
            e.name, variants
        );

        Ok(re
            .replace(content, |caps: &regex::Captures| {
                format!("{}{}", enum_code, &caps[1])
            })
            .to_string())
    }

    fn insert_field(&self, content: &str, f: &FieldPatch) -> Result<String, String> {
        // Find closing brace of message and insert field before it
        // Pattern needs to handle nested messages, so we find the message start
        // and then look for the field insertion point
        let pattern = format!(
            r"(?s)(message\s+{}\s*\{{[^{{}}]*)(}})",
            regex::escape(f.message)
        );
        let re = Regex::new(&pattern).map_err(|e| format!("Invalid regex: {}", e))?;

        if !re.is_match(content) {
            return Err(format!(
                "Could not find message '{}' to insert field '{}'",
                f.message, f.field_name
            ));
        }

        let field_code = format!(
            "\n  // LINGUA PATCH - DELETE when googleapis adds upstream\n  optional {} {} = {};\n",
            f.field_type, f.field_name, f.tag
        );

        Ok(re
            .replace(content, |caps: &regex::Captures| {
                format!("{}{}{}", &caps[1], field_code, &caps[2])
            })
            .to_string())
    }

    fn validate(&self, content: &str) -> Result<(), String> {
        // Validate enums exist
        for e in &self.enums {
            let pattern = format!(r"enum\s+{}\s*\{{", regex::escape(e.name));
            let re = Regex::new(&pattern).unwrap();
            if !re.is_match(content) {
                return Err(format!(
                    "Validation failed: enum '{}' not found in proto",
                    e.name
                ));
            }
        }

        // Validate fields exist
        for f in &self.fields {
            if !content.contains(&format!("{} = {};", f.field_name, f.tag)) {
                return Err(format!(
                    "Validation failed: field '{}' not found in proto",
                    f.field_name
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_enum() {
        let input = r#"syntax = "proto3";

message ThinkingConfig {
  optional bool include_thoughts = 1;
  optional int32 thinking_budget = 2;
}
"#;
        let patches = ProtoPatches {
            enums: vec![EnumPatch {
                name: "ThinkingLevel",
                insert_before_message: "ThinkingConfig",
                variants: &[("THINKING_LEVEL_UNSPECIFIED", 0), ("LOW", 1), ("HIGH", 3)],
            }],
            fields: vec![],
        };

        let result = patches.apply(input).unwrap();

        assert!(result.contains("enum ThinkingLevel {"));
        assert!(result.contains("THINKING_LEVEL_UNSPECIFIED = 0;"));
        assert!(result.contains("LOW = 1;"));
        assert!(result.contains("HIGH = 3;"));
        assert!(result.contains("message ThinkingConfig {")); // still there
                                                              // Enum should come before message
        let enum_pos = result.find("enum ThinkingLevel").unwrap();
        let msg_pos = result.find("message ThinkingConfig").unwrap();
        assert!(enum_pos < msg_pos);
    }

    #[test]
    fn test_insert_field() {
        let input = r#"message ImageConfig {
  optional string aspect_ratio = 1;
}
"#;
        let patches = ProtoPatches {
            enums: vec![],
            fields: vec![FieldPatch {
                message: "ImageConfig",
                field_name: "image_size",
                field_type: "string",
                tag: 2,
            }],
        };

        let result = patches.apply(input).unwrap();

        assert!(result.contains("optional string image_size = 2;"));
        assert!(result.contains("LINGUA PATCH"));
    }

    #[test]
    fn test_insert_enum_field() {
        let input = r#"message ThinkingConfig {
  optional int32 thinking_budget = 2;
}
"#;
        let patches = ProtoPatches {
            enums: vec![EnumPatch {
                name: "ThinkingLevel",
                insert_before_message: "ThinkingConfig",
                variants: &[("THINKING_LEVEL_UNSPECIFIED", 0), ("HIGH", 3)],
            }],
            fields: vec![FieldPatch {
                message: "ThinkingConfig",
                field_name: "thinking_level",
                field_type: "ThinkingLevel",
                tag: 3,
            }],
        };

        let result = patches.apply(input).unwrap();

        assert!(result.contains("enum ThinkingLevel {"));
        assert!(result.contains("optional ThinkingLevel thinking_level = 3;"));
    }

    #[test]
    fn test_validation_fails_for_missing_message() {
        let input = "message SomeOtherConfig {}";
        let patches = ProtoPatches {
            enums: vec![EnumPatch {
                name: "ThinkingLevel",
                insert_before_message: "ThinkingConfig",
                variants: &[("UNSPECIFIED", 0)],
            }],
            fields: vec![],
        };

        let result = patches.apply(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ThinkingConfig"));
    }
}
