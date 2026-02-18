//! Generate random `serde_json::Value` from OpenAPI / JSON Schema definitions.
//!
//! This module walks a JSON Schema tree and produces a `proptest::Strategy` that
//! generates conforming JSON values. It handles `$ref`, `const`, `enum`,
//! `anyOf`/`oneOf`, objects (required + optional properties), arrays, and
//! primitive types.
//!
//! Used by `fuzz_roundtrip.rs` to auto-generate provider request payloads
//! directly from the OpenAPI specs in `specs/`.

use lingua::serde_json::{self, json, Map, Value};
use proptest::prelude::*;
use std::sync::Arc;

/// Context for schema resolution (definitions map + depth tracking).
#[derive(Clone)]
pub struct SchemaCtx {
    definitions: Arc<Map<String, Value>>,
}

impl SchemaCtx {
    pub fn new(definitions: Map<String, Value>) -> Self {
        Self {
            definitions: Arc::new(definitions),
        }
    }

    /// Look up a schema name from a `$ref` string like `#/components/schemas/Foo`.
    fn resolve_ref(&self, ref_str: &str) -> Option<&Value> {
        let name = ref_str.rsplit('/').next()?;
        self.definitions.get(name)
    }
}

const MAX_DEPTH: usize = 12;

/// Build a proptest `Strategy` that generates random JSON conforming to `schema`.
pub fn strategy_from_schema(schema: &Value, ctx: &SchemaCtx, depth: usize) -> BoxedStrategy<Value> {
    if depth > MAX_DEPTH {
        return Just(Value::Null).boxed();
    }

    // Handle $ref
    if let Some(ref_str) = schema.get("$ref").and_then(|v| v.as_str()) {
        if let Some(resolved) = ctx.resolve_ref(ref_str) {
            let resolved = resolved.clone();
            let ctx = ctx.clone();
            return strategy_from_schema(&resolved, &ctx, depth + 1);
        }
        return Just(Value::Null).boxed();
    }

    // Handle const
    if let Some(const_val) = schema.get("const") {
        return Just(const_val.clone()).boxed();
    }

    // Handle enum
    if let Some(enum_vals) = schema.get("enum").and_then(|v| v.as_array()) {
        if !enum_vals.is_empty() {
            let vals: Vec<Value> = enum_vals.clone();
            return (0..vals.len()).prop_map(move |i| vals[i].clone()).boxed();
        }
    }

    // Handle anyOf / oneOf (pick one variant)
    if let Some(variants) = schema
        .get("anyOf")
        .or_else(|| schema.get("oneOf"))
        .and_then(|v| v.as_array())
    {
        if !variants.is_empty() {
            let strategies: Vec<BoxedStrategy<Value>> = variants
                .iter()
                .map(|v| strategy_from_schema(v, ctx, depth + 1))
                .collect();
            return proptest::strategy::Union::new(strategies).boxed();
        }
    }

    // Handle allOf (merge all schemas - simplified: just use the first object-like one)
    if let Some(all_of) = schema.get("allOf").and_then(|v| v.as_array()) {
        // Merge all properties into a single object schema
        let mut merged_props = Map::new();
        let mut merged_required = Vec::new();
        for sub in all_of {
            if let Some(props) = sub.get("properties").and_then(|p| p.as_object()) {
                for (k, v) in props {
                    merged_props.insert(k.clone(), v.clone());
                }
            }
            if let Some(req) = sub.get("required").and_then(|r| r.as_array()) {
                for r in req {
                    if let Some(s) = r.as_str() {
                        merged_required.push(Value::String(s.to_string()));
                    }
                }
            }
            // Also resolve $ref within allOf
            if let Some(ref_str) = sub.get("$ref").and_then(|v| v.as_str()) {
                if let Some(resolved) = ctx.resolve_ref(ref_str) {
                    if let Some(props) = resolved.get("properties").and_then(|p| p.as_object()) {
                        for (k, v) in props {
                            merged_props.insert(k.clone(), v.clone());
                        }
                    }
                    if let Some(req) = resolved.get("required").and_then(|r| r.as_array()) {
                        for r in req {
                            if let Some(s) = r.as_str() {
                                merged_required.push(Value::String(s.to_string()));
                            }
                        }
                    }
                }
            }
        }
        if !merged_props.is_empty() {
            let merged = json!({
                "type": "object",
                "properties": Value::Object(merged_props),
                "required": Value::Array(merged_required),
            });
            return strategy_from_schema(&merged, ctx, depth + 1);
        }
    }

    // Handle by type
    let type_str = schema.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match type_str {
        "string" => arb_string(schema).boxed(),
        "integer" => (-1000i64..1000).prop_map(|i| json!(i)).boxed(),
        "number" => (-100.0f64..100.0).prop_map(|f| json!(f)).boxed(),
        "boolean" => any::<bool>().prop_map(Value::Bool).boxed(),
        "null" => Just(Value::Null).boxed(),
        "object" => strategy_for_object(schema, ctx, depth),
        "array" => strategy_for_array(schema, ctx, depth),
        _ => {
            // No type specified - could be a ref-only or untyped schema
            // Try to infer from other properties
            if schema.get("properties").is_some() {
                strategy_for_object(schema, ctx, depth)
            } else if schema.get("items").is_some() {
                strategy_for_array(schema, ctx, depth)
            } else {
                // Fallback: random primitive
                prop_oneof![
                    "[a-zA-Z0-9 ]{1,30}".prop_map(Value::String),
                    Just(Value::Null),
                ]
                .boxed()
            }
        }
    }
}

fn arb_string(schema: &Value) -> impl Strategy<Value = Value> {
    // If there are enum values, use those
    if let Some(enum_vals) = schema.get("enum").and_then(|v| v.as_array()) {
        let vals: Vec<String> = enum_vals
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        if !vals.is_empty() {
            return (0..vals.len())
                .prop_map(move |i| Value::String(vals[i].clone()))
                .boxed();
        }
    }
    "[a-zA-Z0-9 .!?,]{1,50}".prop_map(Value::String).boxed()
}

fn strategy_for_object(schema: &Value, ctx: &SchemaCtx, depth: usize) -> BoxedStrategy<Value> {
    let properties = schema
        .get("properties")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let required: Vec<String> = schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    if properties.is_empty() {
        return Just(json!({})).boxed();
    }

    // Separate required and optional properties
    let mut required_entries: Vec<(String, Value)> = Vec::new();
    let mut optional_entries: Vec<(String, Value)> = Vec::new();

    for (key, prop_schema) in &properties {
        if required.contains(key) {
            required_entries.push((key.clone(), prop_schema.clone()));
        } else {
            optional_entries.push((key.clone(), prop_schema.clone()));
        }
    }

    let ctx_clone = ctx.clone();

    // Build strategies for required properties
    let required_strategies: Vec<(String, BoxedStrategy<Value>)> = required_entries
        .into_iter()
        .map(|(key, prop_schema)| {
            let strat = strategy_from_schema(&prop_schema, &ctx_clone, depth + 1);
            (key, strat)
        })
        .collect();

    // Build strategies for optional properties (each may or may not be included)
    let optional_strategies: Vec<(String, BoxedStrategy<Option<Value>>)> = optional_entries
        .into_iter()
        .map(|(key, prop_schema)| {
            let strat =
                prop::option::of(strategy_from_schema(&prop_schema, &ctx_clone, depth + 1)).boxed();
            (key, strat)
        })
        .collect();

    // We need to combine all strategies. Use a tuple approach with vectors.
    // Generate required values as a vec, optional as a vec of options.
    let req_strat = required_strategies
        .into_iter()
        .map(|(k, s)| s.prop_map(move |v| (k.clone(), v)))
        .collect::<Vec<_>>();

    let opt_strat = optional_strategies
        .into_iter()
        .map(|(k, s)| s.prop_map(move |v| (k.clone(), v)))
        .collect::<Vec<_>>();

    // Use prop_flat_map to combine dynamically-sized strategy lists
    let req_values = if req_strat.is_empty() {
        Just(Vec::<(String, Value)>::new()).boxed()
    } else {
        req_strat.into_iter().fold(
            Just(Vec::<(String, Value)>::new()).boxed(),
            |acc, item_strat| {
                (acc, item_strat)
                    .prop_map(|(mut vec, item)| {
                        vec.push(item);
                        vec
                    })
                    .boxed()
            },
        )
    };

    let opt_values = if opt_strat.is_empty() {
        Just(Vec::<(String, Option<Value>)>::new()).boxed()
    } else {
        opt_strat.into_iter().fold(
            Just(Vec::<(String, Option<Value>)>::new()).boxed(),
            |acc, item_strat| {
                (acc, item_strat)
                    .prop_map(|(mut vec, item)| {
                        vec.push(item);
                        vec
                    })
                    .boxed()
            },
        )
    };

    (req_values, opt_values)
        .prop_map(|(required_vals, optional_vals)| {
            let mut map = Map::new();
            for (key, val) in required_vals {
                map.insert(key, val);
            }
            for (key, maybe_val) in optional_vals {
                if let Some(val) = maybe_val {
                    map.insert(key, val);
                }
            }
            Value::Object(map)
        })
        .boxed()
}

fn strategy_for_array(schema: &Value, ctx: &SchemaCtx, depth: usize) -> BoxedStrategy<Value> {
    let items_schema = schema.get("items").cloned().unwrap_or(json!({}));
    let min_items = schema.get("minItems").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
    let max_items = schema
        .get("maxItems")
        .and_then(|v| v.as_u64())
        .unwrap_or(3)
        .min(5) as usize;
    let max_items = max_items.max(min_items);

    let item_strat = strategy_from_schema(&items_schema, ctx, depth + 1);
    proptest::collection::vec(item_strat, min_items..=max_items)
        .prop_map(Value::Array)
        .boxed()
}

// ============================================================================
// Spec loading helpers
// ============================================================================

/// Load an OpenAPI spec and extract the `components.schemas` definitions map.
pub fn load_openapi_definitions(spec_path: &str) -> Map<String, Value> {
    let content = std::fs::read_to_string(spec_path)
        .unwrap_or_else(|e| panic!("Failed to read spec at {}: {}", spec_path, e));

    // Try JSON first (Anthropic spec is JSON despite .yml extension), then YAML
    let spec: Value = serde_json::from_str(&content)
        .or_else(|_| serde_yaml::from_str::<Value>(&content).map_err(|e| e.to_string()))
        .unwrap_or_else(|e| panic!("Failed to parse spec at {}: {}", spec_path, e));

    spec.get("components")
        .and_then(|c| c.get("schemas"))
        .and_then(|s| s.as_object())
        .cloned()
        .unwrap_or_else(|| panic!("No components.schemas in {}", spec_path))
}

/// Load a Google Discovery REST spec and extract the top-level `schemas` map.
pub fn load_discovery_definitions(spec_path: &str) -> Map<String, Value> {
    let content = std::fs::read_to_string(spec_path)
        .unwrap_or_else(|e| panic!("Failed to read spec at {}: {}", spec_path, e));

    let spec: Value = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse spec at {}: {}", spec_path, e));

    spec.get("schemas")
        .and_then(|s| s.as_object())
        .cloned()
        .unwrap_or_else(|| panic!("No schemas in {}", spec_path))
}

/// Build a strategy for a named schema from the definitions map.
pub fn strategy_for_schema_name(
    name: &str,
    definitions: &Map<String, Value>,
) -> BoxedStrategy<Value> {
    let schema = definitions
        .get(name)
        .unwrap_or_else(|| panic!("Schema '{}' not found in definitions", name));
    let ctx = SchemaCtx::new(definitions.clone());
    strategy_from_schema(schema, &ctx, 0)
}
