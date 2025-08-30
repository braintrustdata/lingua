# GitHub Actions

This directory contains GitHub Actions workflows for the LLMIR project.

## Workflows

### CI (`ci.yml`)

Runs automatically on every push and pull request to the `main` branch.

**What it does:**
- ✅ Checks code formatting with `cargo fmt --check`
- ✅ Runs clippy with strict warnings (`-D warnings`)
- ✅ Builds the project and all examples
- ✅ Runs all tests
- ✅ Installs protobuf compiler for Google types

### Update Provider Types (`update-provider-types.yml`)

A manually triggered workflow to update provider type definitions.

**How to trigger:**
1. Go to the **Actions** tab in GitHub
2. Select **Update Provider Types** workflow
3. Click **Run workflow**
4. Choose which providers to update:
   - `all` (default) - Updates all providers
   - `openai` - Updates only OpenAI types
   - `anthropic` - Updates only Anthropic types  
   - `google` - Updates only Google types
   - `openai,anthropic` - Updates multiple specific providers

**What it does:**
1. 📥 Downloads latest provider specifications (OpenAPI/protobuf)
2. 🏗️ Regenerates Rust types using the pipeline scripts
3. 🔧 Applies `cargo fmt` and `cargo clippy --fix`
4. 📝 Creates a pull request if there are changes
5. ❌ Does nothing if types are already up to date

**Example PR created by this workflow:**
```
Title: Update openai,anthropic provider types

Summary:
- Updated provider type definitions for: openai,anthropic
- Downloaded latest OpenAPI specs and protobuf files
- Regenerated Rust types using automated pipeline

Changes Made:
- 📥 Downloaded latest provider specifications
- 🏗️ Regenerated types using generate-provider-types.sh
- 🔧 Applied cargo fmt and clippy fixes
- ✅ All checks passing
```

## Permissions

The workflows use `GITHUB_TOKEN` which has the following permissions:
- Read repository contents
- Create pull requests
- Write to the Actions cache

No additional secrets are required.

## Local Development

To run the same checks locally before pushing:

```bash
# Check formatting
cargo fmt --all -- --check

# Run clippy with strict warnings
cargo clippy --all-targets --all-features -- -D warnings

# Build and test
cargo build
cargo test

# Update provider types manually
./pipelines/generate-provider-types.sh all
```

## Troubleshooting

### CI failures

- **Formatting issues**: Run `cargo fmt` locally and commit
- **Clippy warnings**: Run `cargo clippy --fix` locally and commit
- **Build failures**: Check that all dependencies are properly specified
- **Protobuf issues**: Ensure Google protobuf files are properly downloaded

### Provider update workflow

- **No changes detected**: Provider types are already up to date
- **Download failures**: Check if provider API specifications are available
- **Type generation failures**: Check the pipeline scripts work locally