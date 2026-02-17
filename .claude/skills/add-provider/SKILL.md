---
name: add-provider
description: Add support for a new LLM provider format to Lingua. Follow the test-first workflow with payload snapshots.
---

# Add Provider Skill

Add support for a new LLM provider to Lingua using a test-first approach.

**Full documentation**: `crates/lingua/docs/ADDING_PROVIDER_FORMAT.md`

## Workflow Overview

```
1. Add payload snapshots → 2. Add ProviderFormat → 3. Create module → 4. Implement adapter → 5. Validate with coverage-report
```

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

## Step 4: Implement ProviderAdapter

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
cargo run --bin coverage-report -- -p myprovider > .claude/myprovider_bugs.md
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
- [ ] Coverage report shows transformations working
- [ ] Roundtrip tests passing (Provider → Universal → Provider)
