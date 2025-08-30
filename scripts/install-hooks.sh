#!/bin/bash

# Git hooks installation script for LLMIR project
# This script installs pre-commit hooks to ensure code quality

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
HOOKS_DIR="$PROJECT_ROOT/.git/hooks"

echo "🔗 Installing git hooks for LLMIR project..."

# Check if we're in a git repository
if [ ! -d "$PROJECT_ROOT/.git" ]; then
    echo "❌ Error: Not in a git repository"
    echo "   Make sure you're running this from the LLMIR project root"
    exit 1
fi

# Create hooks directory if it doesn't exist
mkdir -p "$HOOKS_DIR"

# Install pre-commit hook
if [ -f "$PROJECT_ROOT/hooks/pre-commit" ]; then
    echo "📝 Installing pre-commit hook..."
    cp "$PROJECT_ROOT/hooks/pre-commit" "$HOOKS_DIR/pre-commit"
    chmod +x "$HOOKS_DIR/pre-commit"
    echo "✅ Pre-commit hook installed"
else
    echo "❌ Error: hooks/pre-commit not found"
    exit 1
fi

# Test the hook
echo "🧪 Testing pre-commit hook..."
if "$HOOKS_DIR/pre-commit"; then
    echo "✅ Pre-commit hook test passed"
else
    echo "❌ Pre-commit hook test failed"
    echo "   You may need to run 'cargo fmt' and fix any clippy issues"
    exit 1
fi

echo ""
echo "🎉 Git hooks installed successfully!"
echo ""
echo "The following hooks are now active:"
echo "  • pre-commit: Runs 'cargo fmt --check' and 'cargo clippy'"
echo ""
echo "These hooks will run automatically before each commit to ensure code quality."
echo "If a hook fails, the commit will be aborted."
echo ""
echo "To bypass hooks temporarily (not recommended), use: git commit --no-verify"