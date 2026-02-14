.PHONY: all lingua-wasm typescript python test test-payloads capture capture-transforms clean help generate-types generate-all-providers install-hooks install-wasm-tools setup precommit

all: typescript python ## Build all bindings

help: ## Show this help message
	@echo "Lingua Build Targets:"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

generate-provider-types: ## Regenerate provider types from OpenAPI specs (usage: make generate-provider-types PROVIDER=openai)
	@if [ -z "$(PROVIDER)" ]; then \
		echo "Usage: make generate-provider-types PROVIDER=<provider>"; \
		echo "Available providers: openai, anthropic, google, all"; \
		echo "Example: make generate-provider-types PROVIDER=openai"; \
		exit 1; \
	fi
	@echo "Regenerating $(PROVIDER) types from OpenAPI spec..."
	@cargo run --bin generate-types -- $(PROVIDER)

generate-all-providers: ## Regenerate types for all providers (anthropic, openai, google)
	@echo "Regenerating all provider types..."
	./pipelines/generate-provider-types.sh anthropic
	./pipelines/generate-provider-types.sh openai
	./pipelines/generate-provider-types.sh google

generate-types: ## Generate TypeScript types from Rust (via ts-rs)
	@echo "Generating TypeScript types from Rust..."
	@cargo test export_bindings --lib --quiet

lingua-wasm: ## Build WASM package
	@echo "Building WASM package..."
	cd bindings/lingua-wasm && pnpm run build

typescript: generate-types lingua-wasm ## Build TypeScript bindings (WASM)
	@echo "Building TypeScript bindings..."
	cd bindings/typescript && pnpm install && pnpm run build

python: ## Build Python bindings (PyO3)
	@echo "Building Python bindings..."
	cd bindings/python && uv sync --all-extras --group dev

test: test-rust test-typescript test-python test-payloads ## Run all tests

test-rust: ## Run Rust tests
	@echo "Running Rust tests..."
	cargo test

test-typescript: typescript ## Run TypeScript tests
	@echo "Running TypeScript tests..."
	cd bindings/typescript && pnpm run test:run

test-typescript-integration: typescript ## Run TypeScript integration tests
	@echo "Running TypeScript integration tests..."
	cd bindings/typescript && pnpm run test:integration

test-payloads: lingua-wasm ## Run payload transform tests (REGENERATE=1 to auto-fix with API validation)
	@echo "Running payload tests..."
	@cd payloads && pnpm vitest run scripts/transforms $(if $(REGENERATE),|| pnpm tsx scripts/regenerate-failed.ts) \
		|| (echo "\n❌ Tests failed! Run 'make regenerate-failed-transforms' to regenerate with real API calls" && exit 1)

capture: lingua-wasm ## Capture payloads (snapshots + transforms + vitest snapshots)
	cd payloads && pnpm capture $(if $(FILTER),--filter $(FILTER)) $(if $(CASES),--cases $(CASES)) $(if $(FORCE),--force)

capture-transforms: lingua-wasm ## Re-capture only transforms (e.g. make capture-transforms FORCE=1)
	cd payloads && pnpm tsx scripts/transforms/capture-transforms.ts $(if $(FILTER),$(FILTER)) $(if $(FORCE),--force)

regenerate-failed-transforms: lingua-wasm ## Auto-regenerate failed transform payloads
	cd payloads && pnpm tsx scripts/regenerate-failed.ts

test-python: ## Run Python tests
	@echo "Running Python tests..."
	cd bindings/python && uv run maturin develop --features python
	cd bindings/python && uv run pytest tests/ -v

clean: ## Clean build artifacts
	@echo "Cleaning build artifacts..."
	cargo clean
	rm -rf bindings/lingua-wasm/nodejs bindings/lingua-wasm/web
	rm -rf bindings/typescript/dist bindings/typescript/node_modules
	rm -rf bindings/typescript/src/generated
	rm -rf bindings/python/.venv bindings/python/target
	rm -rf target/wheels

check: ## Check all code compiles
	@echo "Checking Rust code..."
	cargo check --all-features
	@echo "Checking TypeScript code..."
	cd bindings/typescript && pnpm run typecheck

fmt: ## Format all code
	@echo "Formatting Rust code..."
	cargo fmt
	@echo "Formatting TypeScript code..."
	cd bindings/typescript && pnpm run lint

install-hooks: ## Install git pre-commit hooks
	@echo "Installing git hooks..."
	./scripts/install-hooks.sh

install-wasm-tools: ## Install WASM build tools (wasm32-unknown-unknown target, wasm-pack)
	@echo "Installing WASM build tools..."
	@rustup target add wasm32-unknown-unknown
	@if ! command -v wasm-pack >/dev/null 2>&1; then \
		cargo install wasm-pack; \
	else \
		echo "✅ wasm-pack already installed"; \
	fi

install-dependencies: ## Install dependencies
	@echo "Installing dependencies..."
	./scripts/setup.sh

setup: install-dependencies install-hooks ## Setup the project

precommit: ## Run formatting, linting, and tests for Rust code
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features -- -D warnings
	cargo test

.DEFAULT_GOAL := all
