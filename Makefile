.PHONY: all typescript python test clean help

help: ## Show this help message
	@echo "LLMIR Build Targets:"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

all: typescript python ## Build all bindings

typescript: ## Build TypeScript bindings (WASM)
	@echo "Building TypeScript bindings..."
	cd bindings/typescript && npm install && npm run build

python: ## Build Python bindings (PyO3)
	@echo "Building Python bindings..."
	cd bindings/python && uv sync --all-extras --group dev

test: test-rust test-typescript test-python ## Run all tests

test-rust: ## Run Rust tests
	@echo "Running Rust tests..."
	cargo test

test-typescript: ## Run TypeScript tests
	@echo "Running TypeScript tests..."
	cd bindings/typescript && npm run test:run

test-python: ## Run Python tests
	@echo "Running Python tests..."
	cd bindings/python && uv run pytest tests/ -v

clean: ## Clean build artifacts
	@echo "Cleaning build artifacts..."
	cargo clean
	rm -rf bindings/typescript/wasm bindings/typescript/dist bindings/typescript/node_modules
	rm -rf bindings/python/.venv bindings/python/target
	rm -rf target/wheels

check: ## Check all code compiles
	@echo "Checking Rust code..."
	cargo check --all-features
	@echo "Checking TypeScript code..."
	cd bindings/typescript && npm run typecheck

fmt: ## Format all code
	@echo "Formatting Rust code..."
	cargo fmt
	@echo "Formatting TypeScript code..."
	cd bindings/typescript && npm run lint

.DEFAULT_GOAL := help
