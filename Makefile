.PHONY: all typescript python golang test clean help generate-types install-hooks

all: typescript python golang ## Build all bindings

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

golang: ## Build Golang bindings (CGo)
	@echo "Building Rust library with golang feature..."
	cargo build --release --features golang
	@echo "Golang bindings ready at target/release/liblingua"

test: test-rust test-typescript test-python test-golang ## Run all tests

test-rust: ## Run Rust tests
	@echo "Running Rust tests..."
	cargo test

test-typescript: generate-types ## Run TypeScript tests
	@echo "Running TypeScript tests..."
	cd bindings/typescript && pnpm run test:run

test-python: ## Run Python tests
	@echo "Running Python tests..."
	cd bindings/python && uv run pytest tests/ -v

test-golang: golang ## Run Golang tests
	@echo "Running Golang tests..."
	cd bindings/golang && go test -v

clean: ## Clean build artifacts
	@echo "Cleaning build artifacts..."
	cargo clean
	rm -rf bindings/typescript/wasm bindings/typescript/dist bindings/typescript/node_modules
	rm -rf bindings/typescript/src/generated
	rm -rf bindings/python/.venv bindings/python/target
	rm -rf bindings/golang/coverage.out bindings/golang/coverage.html
	rm -rf target/wheels

check: ## Check all code compiles
	@echo "Checking Rust code..."
	cargo check --all-features
	@echo "Checking TypeScript code..."
	cd bindings/typescript && pnpm run typecheck

fmt: ## Format all code
	@echo "Formatting Rust code..."
	cargo fmt
	@echo "Formatting Go code..."
	cd bindings/golang && go fmt ./...
	@echo "Formatting TypeScript code..."
	cd bindings/typescript && pnpm run lint

install-hooks: ## Install git pre-commit hooks
	@echo "Installing git hooks..."
	./scripts/install-hooks.sh

.DEFAULT_GOAL := all
