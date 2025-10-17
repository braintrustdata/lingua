.PHONY: all typescript python test clean help generate-types install-hooks install-wasm-tools setup

all: typescript python ## Build all bindings

help: ## Show this help message
	@echo "Lingua Build Targets:"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

generate-types: ## Generate TypeScript types from Rust (via ts-rs)
	@echo "Generating TypeScript types from Rust..."
	@cargo test --lib --no-run --quiet

typescript: generate-types ## Build TypeScript bindings (WASM)
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
	rm -rf bindings/typescript/wasm bindings/typescript/dist bindings/typescript/node_modules
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

install-wasm-tools: ## Install WASM build tools (wasm32-unknown-unknown target, wasm-bindgen-cli)
	@echo "Installing WASM build tools..."
	@rustup target add wasm32-unknown-unknown
	@if ! command -v wasm-bindgen >/dev/null 2>&1; then \
		cargo install wasm-bindgen-cli@0.2.100; \
	else \
		echo "✅ wasm-bindgen already installed"; \
	fi

install-dependencies: ## Install dependencies
	@echo "Installing dependencies..."
	./scripts/setup.sh

setup: install-dependencies install-hooks ## Setup the project

.DEFAULT_GOAL := all
