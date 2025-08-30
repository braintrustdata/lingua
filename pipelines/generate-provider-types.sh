#!/bin/bash

# Provider type generation script
# 
# This script follows the pipeline documented in generate-provider-types.md
# Currently supports: OpenAI (more providers to be added)
#
# Usage: ./generate-provider-types.sh [provider] [--headless]
# Default provider: openai

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Parse arguments
PROVIDER=""
HEADLESS=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --headless)
            HEADLESS=true
            shift
            ;;
        -*)
            echo "Unknown option $1"
            exit 1
            ;;
        *)
            if [ -z "$PROVIDER" ]; then
                PROVIDER="$1"
            else
                echo "Unexpected argument: $1"
                exit 1
            fi
            shift
            ;;
    esac
done

# Set default provider if none specified
PROVIDER="${PROVIDER:-openai}"

echo "🔄 Generating types for provider: $PROVIDER"

# Step 1: Download provider OpenAPI spec
echo "📦 Step 1: Downloading $PROVIDER OpenAPI specification..."

download_provider_spec() {
    case "$PROVIDER" in
        "openai")
            echo "Downloading OpenAI OpenAPI spec..."
            SPEC_URL="https://app.stainless.com/api/spec/documented/openai/openapi.documented.yml"
            SPEC_FILE="$PROJECT_ROOT/specs/openai/openapi.yml"
            ;;
        *)
            echo "❌ Unknown provider: $PROVIDER"
            exit 1
            ;;
    esac
    
    # Create specs directory if it doesn't exist
    mkdir -p "$(dirname "$SPEC_FILE")"
    
    # Download the spec
    if command -v curl >/dev/null 2>&1; then
        curl -s "$SPEC_URL" -o "$SPEC_FILE"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$SPEC_URL" -O "$SPEC_FILE"
    else
        echo "❌ Neither curl nor wget is available"
        exit 1
    fi
    
    if [ -f "$SPEC_FILE" ] && [ -s "$SPEC_FILE" ]; then
        echo "✅ Downloaded OpenAPI spec to: $SPEC_FILE"
        echo "📊 Spec size: $(wc -l < "$SPEC_FILE") lines"
    else
        echo "❌ Failed to download OpenAPI spec"
        exit 1
    fi
}

download_provider_spec

# Step 2: Generate types using build script  
echo "🔨 Step 2: Generating types from OpenAPI specification..."

# Types are automatically generated during cargo build via build.rs
# No manual intervention needed - typify generates types from OpenAPI spec
cd "$PROJECT_ROOT"

# Step 3: Build and validate
echo "🔨 Step 3: Building and validating..."

cd "$PROJECT_ROOT"

# Build Rust code to check for compilation errors
echo "Building Rust code..."
cargo build

if [ $? -ne 0 ]; then
    echo "❌ Rust build failed - check generated types"
    exit 1
fi

# Generate TypeScript bindings
echo "Generating TypeScript bindings..."
cargo run --example simple_${PROVIDER} > /dev/null

# Step 4: Run validation tests
echo "🧪 Step 4: Running validation tests..."

cd "$PROVIDER_TEST_DIR"

# Install TypeScript if not present
if ! command -v tsc &> /dev/null; then
    if ! command -v pnpm &> /dev/null; then
        npm install typescript
    else
        pnpm add typescript
    fi
fi

# Check if we can create compatibility test
if [ -f "$PROJECT_ROOT/bindings/typescript/SimpleMessage.ts" ]; then
    echo "TypeScript bindings generated successfully"
    
    # Run the existing TypeScript tests
    cd "$PROJECT_ROOT/tests/typescript"
    if ! command -v pnpm &> /dev/null; then
        npm run test
    else
        pnpm run test
    fi
    
    if [ $? -eq 0 ]; then
        echo "✅ Validation tests passed"
    else
        echo "❌ Validation tests failed"
        exit 1
    fi
else
    echo "⚠️  TypeScript bindings not found - manual validation required"
fi

echo "🎉 Provider type generation completed successfully for: $PROVIDER"
echo ""
echo "Generated files:"
echo "- $PROJECT_ROOT/src/providers/$PROVIDER/generated.rs (essential types only)"
echo "- $PROJECT_ROOT/specs/$PROVIDER/openapi.yml (local OpenAPI spec)"
echo ""
echo "Next steps:"
echo "1. Types are automatically integrated into your build"
echo "2. Run 'cargo test' to ensure all tests pass"
echo "3. Update translators in src/translators/$PROVIDER.rs if needed"
