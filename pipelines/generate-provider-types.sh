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

# Step 1: Download provider spec
echo "📦 Step 1: Downloading $PROVIDER provider specification..."

download_provider_spec() {
    case "$PROVIDER" in
        "openai")
            echo "Downloading OpenAI OpenAPI spec..."
            SPEC_URL="https://app.stainless.com/api/spec/documented/openai/openapi.documented.yml"
            SPEC_FILE="$PROJECT_ROOT/specs/openai/openapi.yml"
            ;;
        "anthropic")
            echo "Downloading Anthropic OpenAPI spec from the official SDK..."
            # The SDK's scripts/mock uses this bundled spec; .stats.yml no longer publishes its URL.
            SPEC_URL="https://raw.githubusercontent.com/anthropics/anthropic-sdk-typescript/main/scripts/mock-spec.json.gz"
            SPEC_FILE="$PROJECT_ROOT/specs/anthropic/openapi.yml"
            ;;
        "google")
            echo "Downloading Google Discovery spec..."
            SPEC_URL='https://generativelanguage.googleapis.com/$discovery/rest?version=v1beta'
            SPEC_FILE="$PROJECT_ROOT/specs/google/discovery.json"
            ;;
        *)
            echo "❌ Unknown provider: $PROVIDER"
            exit 1
            ;;
    esac
    
    # Create specs directory if it doesn't exist
    mkdir -p "$(dirname "$SPEC_FILE")"

    # Stage downloads so failed transfers or decompression preserve the existing spec.
    SPEC_DOWNLOAD_DIR="$(mktemp -d)"
    trap 'rm -rf "$SPEC_DOWNLOAD_DIR"' EXIT
    
    # Download the spec
    if command -v curl >/dev/null 2>&1; then
        curl -fL -s "$SPEC_URL" -o "$SPEC_DOWNLOAD_DIR/spec"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$SPEC_URL" -O "$SPEC_DOWNLOAD_DIR/spec"
    else
        echo "❌ Neither curl nor wget is available"
        exit 1
    fi

    if [ "$PROVIDER" = "anthropic" ]; then
        gzip -dc "$SPEC_DOWNLOAD_DIR/spec" > "$SPEC_DOWNLOAD_DIR/spec.json"
        mv "$SPEC_DOWNLOAD_DIR/spec.json" "$SPEC_DOWNLOAD_DIR/spec"
    fi

    if [ ! -s "$SPEC_DOWNLOAD_DIR/spec" ]; then
        echo "❌ Downloaded provider spec is empty"
        exit 1
    fi

    mv "$SPEC_DOWNLOAD_DIR/spec" "$SPEC_FILE"
    rm -rf "$SPEC_DOWNLOAD_DIR"
    trap - EXIT
    
    if [ -f "$SPEC_FILE" ] && [ -s "$SPEC_FILE" ]; then
        if [ "$PROVIDER" = "google" ]; then
            cargo run --quiet --manifest-path "$PROJECT_ROOT/Cargo.toml" --bin canonicalize-json -- "$SPEC_FILE"
        fi

        echo "✅ Downloaded provider spec to: $SPEC_FILE"
        echo "📊 Spec size: $(wc -l < "$SPEC_FILE") lines"
    else
        echo "❌ Failed to download provider spec"
        exit 1
    fi
}

download_google_protos() {
    local PROTO_DIR="$PROJECT_ROOT/specs/google/protos"
    mkdir -p "$PROTO_DIR/google/ai/generativelanguage/v1beta"
    mkdir -p "$PROTO_DIR/google/api"
    mkdir -p "$PROTO_DIR/google/protobuf"
    mkdir -p "$PROTO_DIR/google/type"
    
    echo "Downloading Google AI GenerativeLanguage protobuf files..."
    
    # Core protobuf files for Generative AI API
    local googleapis_url="https://raw.githubusercontent.com/googleapis/googleapis/master"
    local protobuf_url="https://raw.githubusercontent.com/protocolbuffers/protobuf/main/src"
    
    # Google API files - using v1beta for function calling support
    local googleapis_files=(
        "google/ai/generativelanguage/v1beta/generative_service.proto"
        "google/ai/generativelanguage/v1beta/content.proto"
        "google/ai/generativelanguage/v1beta/safety.proto"
        "google/ai/generativelanguage/v1beta/citation.proto"
        "google/ai/generativelanguage/v1beta/tool.proto"
        "google/ai/generativelanguage/v1beta/retriever.proto"
        "google/api/annotations.proto"
        "google/api/http.proto"
        "google/api/field_behavior.proto"
        "google/api/resource.proto"
        "google/api/client.proto"
        "google/api/launch_stage.proto"
        "google/type/interval.proto"
    )
    
    # Standard protobuf files
    local protobuf_files=(
        "google/protobuf/duration.proto"
        "google/protobuf/timestamp.proto" 
        "google/protobuf/descriptor.proto"
        "google/protobuf/any.proto"
        "google/protobuf/struct.proto"
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

# Step 4: Validation complete
echo "✅ Type generation and validation completed"

echo "🎉 Provider type generation completed successfully for: $PROVIDER"
echo ""
echo "Generated files:"
if [ "$PROVIDER" = "google" ]; then
    echo "- $PROJECT_ROOT/crates/lingua/src/providers/$PROVIDER/generated.rs (types from Google Discovery spec)"
    echo "- $PROJECT_ROOT/specs/$PROVIDER/discovery.json (local Google Discovery spec)"
else
    echo "- $PROJECT_ROOT/crates/lingua/src/providers/$PROVIDER/generated.rs (types from OpenAPI spec)"
    echo "- $PROJECT_ROOT/specs/$PROVIDER/openapi.yml (local OpenAPI spec)"
fi
echo ""
echo "Next steps:"
echo "1. Types are automatically integrated into your build"
echo "2. Run 'cargo test' manually to validate conversion logic"
echo "3. Update conversion logic in crates/lingua/src/providers/$PROVIDER/convert.rs if needed"
