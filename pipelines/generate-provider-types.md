# Provider type generation pipeline

Lingua generates provider wire types from checked-in API specifications. Generated files must only be changed by this pipeline.

## Supported providers

- OpenAI: Stainless OpenAPI specification
- Anthropic: hosted community OpenAPI specification

## Run the pipeline

```bash
./pipelines/generate-provider-types.sh openai
./pipelines/generate-provider-types.sh anthropic
```

Use `--headless` in automation:

```bash
./pipelines/generate-provider-types.sh openai --headless
```

The script downloads the current specification, runs `generate-types`, formats the generated Rust, and verifies that the workspace builds.

Run `pnpm install` once after cloning. Lingua pins quicktype in `tools/quicktype/package.json`; no global quicktype installation is used.

## OpenAI generation

OpenAI types are generated with quicktype from `specs/openai/openapi.yml`.

The pinned quicktype release processes Stainless schemas that contain intersections nested inside unions. The generator then applies narrow compatibility transforms for stable Rust names used by Lingua adapters. Compatibility aliases are emitted only when they do not collide with newly generated names.

Generation errors return a non-zero exit code and leave the existing generated Rust file unchanged.

## Output files

- `crates/lingua/src/providers/openai/generated.rs`
- `crates/lingua/src/providers/anthropic/generated.rs`
- `specs/openai/openapi.yml`
- `specs/anthropic/openapi.json`

Do not edit `generated.rs` files directly. Change `crates/generate-types/src/main.rs`, regenerate, and commit the resulting specification and generated output together.

## Validation

Run focused generator tests first:

```bash
cargo test -p generate-types
```

Then run the provider pipeline and Lingua checks:

```bash
./pipelines/generate-provider-types.sh openai --headless
cargo test -p lingua
make typed-boundary-check
```
