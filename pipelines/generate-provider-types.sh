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

# Step 1: Check for provider SDK updates
echo "📦 Step 1: Checking for $PROVIDER SDK updates..."

PROVIDER_TEST_DIR="$PROJECT_ROOT/tests/typescript/$PROVIDER"
if [ ! -d "$PROVIDER_TEST_DIR" ]; then
    echo "Creating test directory: $PROVIDER_TEST_DIR"
    mkdir -p "$PROVIDER_TEST_DIR"
    cd "$PROVIDER_TEST_DIR"
    
    # Initialize package.json for provider testing
    cat > package.json <<EOF
{
  "name": "llmir-${PROVIDER}-tests",
  "version": "0.1.0",
  "private": true,
  "description": "TypeScript compatibility tests for LLMIR ${PROVIDER} provider",
  "scripts": {
    "typecheck": "tsc --noEmit",
    "extract-types": "node extract-types.js"
  },
  "devDependencies": {
    "typescript": "^5.0.0",
    "@types/node": "^20.0.0"
  }
}
EOF
fi

cd "$PROVIDER_TEST_DIR"

# Add provider SDK if not already present
case "$PROVIDER" in
    "openai")
        echo "Installing/updating OpenAI SDK..."
        if ! command -v pnpm &> /dev/null; then
            npm install --save-dev openai@latest
        else
            pnpm add -D openai@latest
        fi
        ;;
    *)
        echo "❌ Unsupported provider: $PROVIDER"
        exit 1
        ;;
esac

# Step 2: Extract type definitions using Claude Code
echo "🔍 Step 2: Extracting type definitions..."

# Create type extraction script that Claude Code will execute
cat > extract-types.js <<'EOF'
// This script will be executed by Claude Code to extract provider types
const fs = require('fs');
const path = require('path');

// This is a placeholder - Claude Code will implement the actual extraction logic
console.log('Type extraction script ready for Claude Code implementation');
console.log('Provider SDK installed and ready for type analysis');
EOF

echo "📋 Invoking Claude to extract $PROVIDER types..."

# Change to project root so Claude can access the entire codebase
cd "$PROJECT_ROOT"

# Define the prompt content directly as a variable
PROMPT_CONTENT="I need you to analyze the ${PROVIDER} TypeScript SDK and extract type definitions for chat completion requests and responses.

Working from project root: $PROJECT_ROOT
Provider SDK location: tests/typescript/${PROVIDER}/node_modules/${PROVIDER}/

Please follow the pipeline documented in pipelines/generate-provider-types.md:

1. Examine the ${PROVIDER} SDK types in tests/typescript/${PROVIDER}/node_modules/${PROVIDER}/
2. Find the chat completion request and response interfaces  
3. Generate equivalent Rust types in src/providers/${PROVIDER}/
4. Create separate files: request.rs and response.rs
5. Update the mod.rs file to export both modules

Focus on:
- ChatCompletionCreateParams (request)
- ChatCompletion (response)
- All supporting types and enums

Make sure to:
- Use #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] on all types
- Handle optional fields with Option<T>
- Convert TypeScript unions to appropriate Rust enums
- Follow the TypeScript → Rust conversion rules in the pipeline doc

After generating the types, also update the translator in src/translators/${PROVIDER}.rs to use the new types."
echo "$PROMPT_CONTENT"
echo ""
echo "🤖 Starting Claude Code session..."


# Use Claude to analyze the provider SDK and extract types
if [ "$HEADLESS" = true ]; then
    # Use --print flag for headless/non-interactive execution
    claude --print "$PROMPT_CONTENT"
    CLAUDE_EXIT_CODE=$?
else
    # Interactive mode - pass prompt directly as an argument
    claude "$PROMPT_CONTENT"
    CLAUDE_EXIT_CODE=$?
fi

if [ $CLAUDE_EXIT_CODE -ne 0 ]; then
    echo "❌ Claude type extraction failed"
    exit 1
fi

echo "✅ Type extraction completed"

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
echo "- $PROJECT_ROOT/src/providers/$PROVIDER/request.rs"
echo "- $PROJECT_ROOT/src/providers/$PROVIDER/response.rs" 
echo "- $PROJECT_ROOT/src/providers/$PROVIDER/mod.rs"
echo "- Updated: $PROJECT_ROOT/src/translators/$PROVIDER.rs"
echo ""
echo "Next steps:"
echo "1. Review the generated types for accuracy"
echo "2. Run 'cargo test' to ensure all tests pass"
echo "3. Test with real API calls if needed"
