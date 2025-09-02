#!/bin/bash

# TypeScript generation script for Elmir universal message format
# This script runs the Rust binary that generates TypeScript bindings using ts-rs

set -e

echo "🚀 Generating TypeScript bindings for Elmir universal message format..."

# Build the project first to ensure all types are compiled
echo "📦 Building Rust project..."
cargo build --bin generate-ts

# Run the TypeScript generation binary
echo "⚡ Running TypeScript generation..."
cargo run --bin generate-ts

# Verify generated files
echo "✅ Verifying generated files..."
if [ -d "bindings/typescript" ]; then
    echo "📂 Generated files in bindings/typescript/:"
    ls -1 bindings/typescript/*.ts | sed 's/.*\//  📄 /'
    echo ""
    echo "🎉 TypeScript bindings generated successfully!"
    echo "💡 Import types with: import { Message, UserContentPart } from './bindings/typescript'"
else
    echo "❌ Error: bindings/typescript directory not found"
    exit 1
fi