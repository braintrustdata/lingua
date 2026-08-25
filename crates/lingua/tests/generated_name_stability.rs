//! Guards the public names of the checked-in Anthropic generated types.
//!
//! Quicktype names anonymous schemas from a positional adjective sequence and names unified
//! schemas from the longest common suffix of their titles. Both are regeneration-unstable and
//! meaningless, and both previously leaked into the exported Rust and TypeScript surface (for
//! example `PurpleType` and `ErToolsetConfigs`). `crates/generate-types` normalizes them; this
//! test fails if an un-normalized name ever reaches the committed output.

const ANTHROPIC_GENERATED: &str = include_str!("../src/providers/anthropic/generated.rs");

/// The adjectives quicktype hands out to anonymous schemas, in order.
const PLACEHOLDER_ADJECTIVES: [&str; 13] = [
    "Purple",
    "Fluffy",
    "Tentacled",
    "Sticky",
    "Indigo",
    "Indecent",
    "Hilarious",
    "Ambitious",
    "Cunning",
    "Magenta",
    "Frisky",
    "Mischievous",
    "Braggadocious",
];

/// `BrowserFooConfig` unified with `ComputerFooConfig` yields the suffix `erFooConfig`.
const MIS_DERIVED_FRAGMENTS: [&str; 1] = ["Er"];

fn starts_with_fragment(name: &str, fragment: &str) -> bool {
    name.len() > fragment.len()
        && name.starts_with(fragment)
        && name[fragment.len()..].starts_with(|c: char| c.is_ascii_uppercase())
}

fn public_type_names(source: &str) -> Vec<&str> {
    source
        .lines()
        .filter_map(|line| {
            line.strip_prefix("pub struct ")
                .or_else(|| line.strip_prefix("pub enum "))
        })
        .map(|rest| rest.trim_end_matches(['{', ' ']))
        .collect()
}

fn pascal_case(wire_tag: &str) -> String {
    wire_tag
        .split('_')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

#[test]
fn anthropic_generated_types_have_no_placeholder_names() {
    for name in public_type_names(ANTHROPIC_GENERATED) {
        for adjective in PLACEHOLDER_ADJECTIVES {
            assert!(
                !starts_with_fragment(name, adjective),
                "generated type `{name}` is named from quicktype's positional `{adjective}` \
                 placeholder, which renumbers on the next specification bump"
            );
        }
        assert!(
            !name.ends_with("Class"),
            "generated type `{name}` still carries quicktype's `Class` collision suffix"
        );
    }
}

#[test]
fn anthropic_generated_types_have_no_mis_derived_word_fragment_names() {
    for name in public_type_names(ANTHROPIC_GENERATED) {
        for fragment in MIS_DERIVED_FRAGMENTS {
            assert!(
                !starts_with_fragment(name, fragment),
                "generated type `{name}` is named from the mis-derived word fragment \
                 `{fragment}`, which quicktype produces when it unifies two schemas"
            );
        }
    }
}

#[test]
fn anthropic_tool_variant_names_match_their_wire_tag() {
    let tool_enum = ANTHROPIC_GENERATED
        .split_once("pub enum Tool {")
        .expect("generated Anthropic types define the Tool enum")
        .1
        .split_once("\n}")
        .expect("the Tool enum is closed")
        .0;

    let mut checked = 0usize;
    let mut wire_tag: Option<&str> = None;
    for line in tool_enum.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("#[serde(rename = \"") {
            wire_tag = rest.split('"').next();
            continue;
        }
        let Some(tag) = wire_tag.take() else {
            continue;
        };
        let variant = trimmed
            .split_once('(')
            .expect("a tagged Tool variant wraps its tool struct")
            .0;

        assert_eq!(
            variant,
            pascal_case(tag),
            "Tool variant `{variant}` does not match its wire tag `{tag}`"
        );
        checked += 1;
    }

    assert!(checked > 0, "the Tool enum should have tagged variants");
}

#[test]
fn anthropic_generated_types_have_no_unreachable_toolset_config_structs() {
    let names = public_type_names(ANTHROPIC_GENERATED);

    // Quicktype merged `BrowserToolsetConfigs` and `ComputerToolsetConfigs` into one object
    // holding the union of both member sets, so no toolset-configs type may be emitted; the
    // toolsets carry `configs` as untyped provider JSON instead.
    let merged: Vec<&&str> = names
        .iter()
        .filter(|name| name.ends_with("ToolsetConfigs"))
        .collect();
    assert!(
        merged.is_empty(),
        "quicktype's merged toolset configs object is still generated: {merged:?}"
    );

    // Its per-member config structs must not survive as definitions nothing refers to.
    let orphaned_configs: Vec<&&str> = names
        .iter()
        .filter(|name| name.ends_with("Config") || name.ends_with("Configs"))
        .filter(|name| ANTHROPIC_GENERATED.matches(**name).count() == 1)
        .collect();
    assert!(
        orphaned_configs.is_empty(),
        "unreferenced config types are still generated: {orphaned_configs:?}"
    );

    assert!(
        ANTHROPIC_GENERATED.contains("pub struct BrowserToolset20260801 {"),
        "the browser toolset itself must still be generated"
    );
    assert!(
        ANTHROPIC_GENERATED.contains("pub configs: Option<serde_json::Value>,"),
        "the toolsets must keep carrying `configs` as untyped provider JSON"
    );
}
