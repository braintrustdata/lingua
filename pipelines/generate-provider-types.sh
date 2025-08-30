#!/bin/bash

# Provider type generation script
# 
# This script follows the pipeline documented in generate-provider-types.md
# Currently supports: OpenAI, Anthropic, Google
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
        "anthropic")
            echo "Downloading Anthropic OpenAPI spec..."
            SPEC_URL="https://raw.githubusercontent.com/laszukdawid/anthropic-openapi-spec/main/hosted_spec.json"
            SPEC_FILE="$PROJECT_ROOT/specs/anthropic/openapi.json"
            ;;
        "google")
            echo "Downloading Google protobuf files..."
            download_google_protos
            return
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

download_google_protos() {
    local PROTO_DIR="$PROJECT_ROOT/specs/google/protos"
    mkdir -p "$PROTO_DIR/google/ai/generativelanguage/v1"
    mkdir -p "$PROTO_DIR/google/api"
    mkdir -p "$PROTO_DIR/google/protobuf"
    
    echo "Downloading Google AI GenerativeLanguage protobuf files..."
    
    # Core protobuf files for Generative AI API
    local googleapis_url="https://raw.githubusercontent.com/googleapis/googleapis/master"
    local protobuf_url="https://raw.githubusercontent.com/protocolbuffers/protobuf/main/src"
    
    # Google API files
    local googleapis_files=(
        "google/ai/generativelanguage/v1/generative_service.proto"
        "google/ai/generativelanguage/v1/content.proto"
        "google/ai/generativelanguage/v1/safety.proto"
        "google/ai/generativelanguage/v1/citation.proto"
        "google/api/annotations.proto"
        "google/api/http.proto"
        "google/api/field_behavior.proto"
        "google/api/resource.proto"
        "google/api/client.proto"
        "google/api/launch_stage.proto"
    )
    
    # Standard protobuf files
    local protobuf_files=(
        "google/protobuf/duration.proto"
        "google/protobuf/timestamp.proto" 
        "google/protobuf/descriptor.proto"
        "google/protobuf/any.proto"
    )
    
    # Download googleapis files
    for file in "${googleapis_files[@]}"; do
        local file_path="$PROTO_DIR/$file"
        local file_url="$googleapis_url/$file"
        
        echo "  Downloading $file..."
        mkdir -p "$(dirname "$file_path")"
        
        if command -v curl >/dev/null 2>&1; then
            curl -s "$file_url" -o "$file_path"
        elif command -v wget >/dev/null 2>&1; then
            wget -q "$file_url" -O "$file_path"
        else
            echo "❌ Neither curl nor wget is available"
            exit 1
        fi
        
        if [ ! -f "$file_path" ] || [ ! -s "$file_path" ]; then
            echo "❌ Failed to download $file"
            exit 1
        fi
    done
    
    # Download standard protobuf files
    for file in "${protobuf_files[@]}"; do
        local file_path="$PROTO_DIR/$file"
        local file_url="$protobuf_url/$file"
        
        echo "  Downloading $file..."
        mkdir -p "$(dirname "$file_path")"
        
        if command -v curl >/dev/null 2>&1; then
            curl -s "$file_url" -o "$file_path"
        elif command -v wget >/dev/null 2>&1; then
            wget -q "$file_url" -O "$file_path"
        else
            echo "❌ Neither curl nor wget is available"
            exit 1
        fi
        
        if [ ! -f "$file_path" ] || [ ! -s "$file_path" ]; then
            echo "❌ Failed to download $file"
            exit 1
        fi
    done
    
    local total_files=$((${#googleapis_files[@]} + ${#protobuf_files[@]}))
    echo "✅ Downloaded Google protobuf files to: $PROTO_DIR"
    echo "📊 Downloaded $total_files protobuf files"
}

download_provider_spec

# Step 2: Generate types using standalone script
echo "🔨 Step 2: Generating types from specifications..."

cd "$PROJECT_ROOT"

# Run the dedicated type generation script
echo "Running type generation script for $PROVIDER..."
cargo run --bin generate-types -- "$PROVIDER"

if [ $? -ne 0 ]; then
    echo "❌ Type generation failed"
    exit 1
fi

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
