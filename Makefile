.PHONY: all lingua-wasm typescript python test clean help generate-types generate-all-providers install-hooks install-wasm-tools setup verify clippy fmt-check

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

test: test-rust test-typescript test-python ## Run all tests

test-rust: ## Run Rust tests
	@echo "Running Rust tests..."
	cargo test

test-typescript: typescript ## Run TypeScript tests
	@echo "Running TypeScript tests..."
	cd bindings/typescript && pnpm run test:run

test-typescript-integration: typescript ## Run TypeScript integration tests
	@echo "Running TypeScript integration tests..."
	cd bindings/typescript && pnpm run test:integration

test-python: ## Run Python tests
	@echo "Running Python tests..."
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

fmt-check: ## Check formatting without modifying
	cargo fmt --all -- --check

clippy: ## Run clippy with warnings as errors (matches CI)
	cargo clippy --all-targets --all-features -- -D warnings

verify: fmt-check clippy ## Run all CI checks locally (run before committing)
	RUSTFLAGS="-D warnings" $(MAKE) test-rust

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

.DEFAULT_GOAL := all
