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
    echo "   You may need to run 'cargo fmt' to fix formatting issues"
    exit 1
fi

echo ""
echo "🎉 Git hooks installed successfully!"
echo ""
echo "The following hooks are now active:"
echo "  • pre-commit: Checks code formatting and clippy compliance"
echo ""
echo "📋 How the pre-commit hook works:"
echo "  • If code needs formatting: FAILS and shows what needs fixing"
echo "  • If clippy finds issues: FAILS and tells you to run 'cargo clippy --fix'"
echo "  • The hook will NOT automatically fix issues - you must fix them manually"
echo ""
echo "🔧 To fix issues before committing:"
echo "  • Run 'cargo fmt' to fix formatting"
echo "  • Run 'cargo clippy --fix' to fix clippy suggestions"
echo "  • Then commit your changes again"
echo ""
echo "⚠️  The hook will abort commits that have formatting/clippy issues!"
echo "To bypass hooks temporarily (not recommended), use: git commit --no-verify"