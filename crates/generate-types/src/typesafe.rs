//! TypeSafe System One type generation from `specs/typesafe/openapi.json`.
//!
//! quicktype merges the members of TypeSafe's question and answer unions into
//! one struct, which loses choice criteria, so these types are generated with
//! typify. Before generating, the schemas are simplified so typify produces
//! tagged enums and ordered maps:
//!
//! * question and answer variants are inlined into their unions, and `const`
//!   discriminators become single-value enums, so the unions become
//!   `#[serde(tag = "type")]` enums;
//! * every `string | object | array` schema (marked `x-accepts`) refers to the
//!   hand-written `TypeSafeEntry`, which can also embed JSON serialized in its
//!   own key order;
//! * maps are `IndexMap`s, so questions and choices keep the caller's order.

use serde_json::{Map, Value};

const SPEC_PATH: &str = "specs/typesafe/openapi.json";
const DEST_PATH: &str = "crates/lingua/src/providers/typesafe/generated.rs";
const ROOT_TYPES: [&str; 2] = ["SystemOneRequest", "SystemOneResponse"];
const UNIONS: [&str; 2] = ["Question", "Answer"];
const ENTRY_ACCEPTS: &str = "string | object | array";
const ENTRY: &str = "Entry";

/// Generate the System One types from the vendored spec, which
/// `pipelines/generate-provider-types.sh typesafe` downloads.
pub fn generate() -> Result<(), Box<dyn std::error::Error>> {
    println!("📦 Generating TypeSafe System One types from {SPEC_PATH}...");
    let spec: Value = serde_json::from_str(&std::fs::read_to_string(SPEC_PATH)?)?;
    let schema: schemars::schema::RootSchema = serde_json::from_value(root_schema(&spec)?)?;

    let mut settings = typify::TypeSpaceSettings::default();
    settings
        .with_derive("PartialEq".to_string())
        .with_struct_builder(false)
        .with_map_type("::indexmap::IndexMap")
        .with_replacement(ENTRY, "super::TypeSafeEntry", std::iter::empty());
    let mut space = typify::TypeSpace::new(&settings);
    space.add_root_schema(schema)?;
    let code = prettyplease::unparse(&syn::parse2(space.to_stream())?);

    std::fs::write(DEST_PATH, post_process(&code))?;
    let _ = std::process::Command::new("cargo")
        .args(["fmt", "--", DEST_PATH])
        .output();
    println!("📝 Generated TypeSafe System One types to: {DEST_PATH}");
    Ok(())
}

/// A JSON Schema holding the simplified schemas the request and response use.
fn root_schema(spec: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    let all_schemas = spec
        .pointer("/components/schemas")
        .and_then(Value::as_object)
        .ok_or("no components/schemas section found in TypeSafe OpenAPI spec")?;
    let mut definitions = Map::new();
    let mut pending: Vec<&str> = ROOT_TYPES.to_vec();
    while let Some(name) = pending.pop() {
        if definitions.contains_key(name) {
            continue;
        }
        let schema = all_schemas
            .get(name)
            .ok_or_else(|| format!("TypeSafe OpenAPI spec has no {name} schema"))?;
        collect_refs(schema, &mut pending);
        definitions.insert(name.to_string(), schema.clone());
    }

    for union in UNIONS {
        let variants = definitions
            .get(union)
            .and_then(|schema| schema.get("oneOf"))
            .and_then(Value::as_array)
            .ok_or_else(|| format!("TypeSafe {union} schema is not a oneOf"))?
            .iter()
            .map(|variant| {
                variant
                    .get("$ref")
                    .and_then(Value::as_str)
                    .and_then(|reference| reference.strip_prefix("#/components/schemas/"))
                    .map(str::to_string)
                    .ok_or_else(|| format!("TypeSafe {union} variant is not a schema reference"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let variants = variants
            .into_iter()
            .map(|name| {
                definitions
                    .remove(&name)
                    .ok_or_else(|| format!("TypeSafe OpenAPI spec has no {name} schema"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let schema = definitions
            .get_mut(union)
            .and_then(Value::as_object_mut)
            .ok_or_else(|| format!("TypeSafe {union} schema is not an object"))?;
        schema.remove("discriminator");
        schema.insert("oneOf".into(), Value::Array(variants));
    }

    let mut definitions = Value::Object(
        definitions
            .into_iter()
            .map(|(name, schema)| (name, simplify(schema)))
            .collect(),
    );
    if let Some(definitions) = definitions.as_object_mut() {
        definitions.insert(
            ENTRY.into(),
            serde_json::json!({"description": "Text, a JSON object, or a JSON array."}),
        );
    }
    Ok(serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "definitions": definitions,
    }))
}

fn collect_refs<'a>(schema: &'a Value, refs: &mut Vec<&'a str>) {
    match schema {
        Value::Object(object) => {
            if let Some(name) = object
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(|reference| reference.strip_prefix("#/components/schemas/"))
            {
                refs.push(name);
            }
            object.values().for_each(|value| collect_refs(value, refs));
        }
        Value::Array(items) => items.iter().for_each(|item| collect_refs(item, refs)),
        _ => {}
    }
}

/// Point references at `definitions`, turn `const` into single-value enums, and
/// replace entry schemas with references to `Entry`.
fn simplify(schema: Value) -> Value {
    match schema {
        Value::Object(mut object) => {
            if object.get("x-accepts").and_then(Value::as_str) == Some(ENTRY_ACCEPTS) {
                let nullable =
                    object
                        .get("anyOf")
                        .and_then(Value::as_array)
                        .is_some_and(|options| {
                            options.iter().any(|option| {
                                option.get("type").and_then(Value::as_str) == Some("null")
                            })
                        });
                let entry = serde_json::json!({"$ref": format!("#/definitions/{ENTRY}")});
                let mut simplified: Map<String, Value> = object
                    .into_iter()
                    .filter(|(key, _)| key == "title" || key == "description")
                    .collect();
                if nullable {
                    simplified.insert("anyOf".into(), serde_json::json!([entry, {"type": "null"}]));
                } else if let Value::Object(entry) = entry {
                    simplified.extend(entry);
                }
                return Value::Object(simplified);
            }
            if let Some(value) = object.remove("const") {
                object.insert("enum".into(), Value::Array(vec![value]));
            }
            if let Some(Value::String(reference)) = object.get_mut("$ref") {
                *reference = reference.replace("#/components/schemas/", "#/definitions/");
            }
            Value::Object(
                object
                    .into_iter()
                    .map(|(key, value)| (key, simplify(value)))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.into_iter().map(simplify).collect()),
        other => other,
    }
}

/// Add a header, use Lingua's `serde_json`, write block doc comments as line
/// comments so rustdoc does not read their indentation as code, and drop the
/// JSON Schema that typify copies into each type's documentation.
fn post_process(code: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut in_schema = false;
    let mut block_indent: Option<String> = None;
    for line in code.lines() {
        let doc = line.trim_start();
        if let Some(indent) = &block_indent {
            let text = doc.strip_suffix("*/");
            lines.push(format!("{indent}///{}", text.unwrap_or(doc)));
            if text.is_some() {
                block_indent = None;
            }
            continue;
        }
        if let Some(text) = doc.strip_prefix("/**") {
            let indent = line[..line.len() - doc.len()].to_string();
            match text.strip_suffix("*/") {
                Some(text) => lines.push(format!("{indent}///{text}")),
                None => {
                    lines.push(format!("{indent}///{text}"));
                    block_indent = Some(indent);
                }
            }
            continue;
        }
        if in_schema {
            in_schema = !doc.starts_with("/// </details>");
            continue;
        }
        if doc.starts_with("/// <details><summary>JSON schema</summary>") {
            in_schema = true;
            if lines.last().is_some_and(|last| last.trim() == "///") {
                lines.pop();
            }
            continue;
        }
        lines.push(line.replace("::serde_json::", "crate::serde_json::"));
    }
    format!(
        "// Generated TypeSafe System One types using typify\n\
         // Source: specs/typesafe/openapi.json\n\
         // Regenerate with `cargo run --bin generate-types -- typesafe`\n\
         #![allow(clippy::clone_on_copy, clippy::derivable_impls)]\n\n{}\n",
        lines.join("\n")
    )
}
