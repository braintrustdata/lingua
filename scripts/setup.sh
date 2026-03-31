#!/bin/bash

# Development environment setup script for Lingua
# Run this after cloning the repository

set -e

echo "🚀 Setting up Lingua for development..."

# Get the absolute path to the scripts directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Get the absolute path to the repo root (one level up from scripts/)
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Check if we're in the right directory
if [ "$(pwd)" != "$REPO_ROOT" ]; then
    echo "❌ Please run this script from the Lingua project root: $REPO_ROOT"
    exit 1
fi

# Install Git hooks
echo "🪝 Installing Git hooks..."
if [ -f "scripts/install-hooks.sh" ]; then
    ./scripts/install-hooks.sh
else
    echo "⚠️  Git hooks installation script not found"
fi

# Setup Rust environment
echo "🦀 Checking Rust environment..."
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust/Cargo not found. Please install Rust: https://rustup.rs/"
    exit 1
fi

# Setup WASM build tools
echo "📦 Setting up WASM build tools..."
rustup target add wasm32-unknown-unknown

# Install wasm-bindgen-cli (matching the version in Cargo.toml)
echo "📦 Installing wasm-bindgen-cli..."
if ! command -v wasm-bindgen &> /dev/null; then
    cargo install wasm-bindgen-cli@0.2.100
else
    echo "✅ wasm-bindgen already installed"
fi

# Check for wasm-opt (optional but recommended)
if ! command -v wasm-opt &> /dev/null; then
    echo "⚠️  wasm-opt not found - WASM files will not be optimized"
    echo "   Install binaryen for smaller WASM files: https://github.com/WebAssembly/binaryen"
fi

# Setup TypeScript environment
if [ -f "payloads/package.json" ]; then
    echo "📦 Setting up TypeScript environment..."
    cd payloads

    # Check for pnpm
    if ! command -v pnpm &> /dev/null; then
        echo "❌ pnpm not found. Please install pnpm: https://pnpm.io/installation"
        exit 1
    fi

    echo "📦 Installing TypeScript dependencies..."
    pnpm install --frozen-lockfile

    cd ..
fi

# Setup Python environment
if [ -f "bindings/python/pyproject.toml" ]; then
    echo "🐍 Setting up Python environment..."

    # Check for uv
    if ! command -v uv &> /dev/null; then
        echo "❌ uv not found. Please install uv: https://astral.sh/uv/install.sh"
        exit 1
    fi

    echo "📦 Installing Python dependencies..."
    cd bindings/python
    uv sync --extra dev
    cd ../..
fi

echo "✅ Setup complete!"
