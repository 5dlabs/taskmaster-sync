#!/bin/bash
# Setup script to install git hooks for automatic formatting

set -e

echo "🔧 Setting up git hooks for TaskMaster Sync..."

# Create hooks directory if it doesn't exist
mkdir -p .git/hooks

# Install pre-commit hook
if [ -f "scripts/pre-commit.sh" ]; then
    ln -sf ../../scripts/pre-commit.sh .git/hooks/pre-commit
    echo "✅ Pre-commit hook installed"
else
    echo "❌ scripts/pre-commit.sh not found"
    exit 1
fi

echo "🎉 Git hooks setup complete!"
echo ""
echo "The pre-commit hook will now:"
echo "  • Auto-format code with cargo fmt"
echo "  • Run clippy linting"
echo "  • Run tests"
echo "  • Add formatted files to the commit"
echo ""
echo "To disable temporarily: git commit --no-verify"