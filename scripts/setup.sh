#!/bin/bash

# Development environment setup script for LLMIR
# Run this after cloning the repository

set -e

echo "🚀 Setting up LLMIR for development..."

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Please run this script from the LLMIR project root"
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
    pnpm install
    
    cd ..
fi

echo "✅ Setup complete!"