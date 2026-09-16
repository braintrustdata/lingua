---
name: add-provider
description: Add support for a new LLM provider format to Lingua. Classify provider-specific features and reject unsupported cross-provider mappings explicitly.
---

# Add Provider Skill

Add support for a new LLM provider to Lingua using a test-first approach.

**Full documentation**: `crates/lingua/docs/ADDING_PROVIDER_FORMAT.md`

## Workflow Overview

```
1. Add payload snapshots → 2. Add ProviderFormat → 3. Create module → 4. Classify and implement conversions → 5. Validate with coverage-report
```

## Transformation boundary

Adding a provider requires faithful native types, detection, validation, and same-format handling. It does not require every provider-specific capability to transform into every other format.

- Implement cross-provider conversion only for features with a clear provider-neutral meaning and a semantically equivalent target representation.
- Do not emulate provider-managed services or translate their opaque execution state, handles, replay data, or lifecycle events into superficially similar target features.
- Return a clear unsupported-mapping error when preserving meaning is impossible. Do not drop the feature, stringify it, flatten it into text, simulate it, or partially convert the request.
- A coverage gap for correctly rejected provider-specific behavior is an intentional limitation, not unfinished adapter work.

## Step 1: Add Payload Snapshots

Create test fixtures from real API calls in `payloads/snapshots/`:

```
payloads/snapshots/simpleRequest/myprovider/
├── request.json
├── response.json
├── response-streaming.json
├── followup-request.json
├── followup-response.json
└── followup-response-streaming.json
```

**Quick manual capture**:
```bash
mkdir -p payloads/snapshots/simpleRequest/myprovider
# Save actual API request/response JSON to these files
```

**Using capture system** (recommended):
```bash
cd payloads && pnpm capture --providers myprovider
```

## Step 2: Add to ProviderFormat Enum

**File**: `crates/lingua/src/capabilities/format.rs`

```rust
pub enum ProviderFormat {
    // ... existing
    MyProvider,  // Add here
    Unknown,
}
```

Update `Display` and `FromStr` implementations.

## Step 3: Create Provider Module

**Directory**: `crates/lingua/src/providers/myprovider/`

```
myprovider/
├── mod.rs
├── adapter.rs    # ProviderAdapter implementation
├── convert.rs    # TryFromLLM conversions
├── detect.rs     # Request/response types
└── params.rs     # Typed params with #[serde(flatten)]
```

**Add to** `crates/lingua/src/providers/mod.rs`:
```rust
#[cfg(feature = "myprovider")]
pub mod myprovider;
```

**Add feature flag** to `Cargo.toml`:
```toml
[features]
default = ["openai", "anthropic", "google", "bedrock", "myprovider"]
myprovider = []
```

## Step 4: Classify and implement ProviderAdapter conversions

**File**: `crates/lingua/src/providers/myprovider/adapter.rs`

Required methods (9 total):

| Method | Purpose |
|--------|---------|
| `format()` | Return `ProviderFormat::MyProvider` |
| `directory_name()` | Return `"myprovider"` (matches snapshot dir) |
| `display_name()` | Return `"MyProvider"` (for reports) |
| `detect_request()` | Return `true` if payload is this format |
| `request_to_universal()` | Provider request → UniversalRequest |
| `request_from_universal()` | UniversalRequest → Provider request |
| `detect_response()` | Return `true` if response is this format |
| `response_to_universal()` | Provider response → UniversalResponse |
| `response_from_universal()` | UniversalResponse → Provider response |

Before implementing each conversion path, classify provider-specific blocks and fields:

- **Transformable**: Stable meaning, representable in the universal model, and semantically supported by the target.
- **Native-only**: Valid provider data that must be preserved for native validation or same-format passthrough but rejected during cross-provider conversion.
- **Ambiguous**: Stop and clarify the intended provider-neutral contract before adding a universal type or fallback.

Test native-only cases for a meaningful unsupported error. Do not force them through `UniversalRequest` or `UniversalResponse` to satisfy method completeness.

**Register in** `crates/lingua/src/processing/adapters.rs`:
```rust
#[cfg(feature = "myprovider")]
list.push(Box::new(crate::providers::myprovider::MyProviderAdapter));
```

## Step 5: Validate with Coverage Report

**Quick iteration** (use compact mode):
```bash
cargo run --bin coverage-report -- -f compact -p myprovider
```

**Full details for debugging**:
```bash
cargo run --bin coverage-report -- -p myprovider
```

**Document issues**:
```bash
cargo run --bin coverage-report -- -p myprovider > .Codex/myprovider_bugs.md
```

## Key Patterns

### params.rs - Typed with Automatic Extras

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MyProviderParams {
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i64>,

    /// Unknown fields captured automatically
    #[serde(flatten)]
    pub extras: BTreeMap<String, Value>,
}
```

### Provider Isolation

Store provider-specific extras keyed by `ProviderFormat` to prevent cross-contamination:

```rust
// In request_to_universal:
provider_extras.insert(ProviderFormat::MyProvider, typed_params.extras);

// In request_from_universal:
if let Some(extras) = req.provider_extras.get(&ProviderFormat::MyProvider) {
    // Only merge back same-provider extras
}
```

## Reference Implementations

| Pattern | Example |
|---------|---------|
| Simple adapter | `providers/anthropic/adapter.rs` |
| Complex with streaming | `providers/openai/adapter.rs` |
| Bedrock (nested config) | `providers/bedrock/adapter.rs` |

## Checklist

- [ ] Payload snapshots captured in `payloads/snapshots/`
- [ ] `ProviderFormat` enum updated
- [ ] Provider module created with all files
- [ ] `ProviderAdapter` trait implemented
- [ ] Adapter registered in `adapters.rs`
- [ ] Feature flag added to `Cargo.toml`
- [ ] Provider-managed and opaque features classified as transformable, native-only, or ambiguous
- [ ] Native-only cross-provider paths return tested unsupported-mapping errors
- [ ] Coverage report shows transformations working
- [ ] Roundtrip tests passing (Provider → Universal → Provider)
