#!/bin/bash

# Streamlined TypeScript generation for Elmir universal message format
# Uses ts-rs with minimal configuration for automatic type generation

set -e

echo "🚀 Generating TypeScript bindings (streamlined approach)..."

# Run the simple TypeScript generation
echo "⚡ Generating types..."
cargo run --bin simple-ts-gen

# Verify generated files  
echo "✅ Generated files:"
if [ -d "bindings/typescript" ]; then
    ls -1 bindings/typescript/*.ts | sed 's/.*\//  📄 /'
    echo ""
    echo "🎉 TypeScript bindings generated successfully!"
    echo "💡 Import: import { Message, Citation } from './bindings/typescript'"
else
    echo "❌ Error: bindings/typescript directory not found"
    exit 1
fi