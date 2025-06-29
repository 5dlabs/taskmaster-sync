#!/bin/bash
# Pre-commit hook for automatic Rust formatting
# Install with: ln -s ../../scripts/pre-commit.sh .git/hooks/pre-commit

set -e

echo "🔧 Running cargo fmt..."
cargo fmt

# Check if there are any changes after formatting
if ! git diff --exit-code --quiet; then
    echo "✅ Code formatted successfully"
    echo "📝 Adding formatted files to commit..."
    git add .
else
    echo "✅ Code already properly formatted"
fi

echo "🔍 Running cargo clippy..."
cargo clippy -- -D warnings

echo "🧪 Running tests..."
cargo test --quiet

echo "✅ All pre-commit checks passed!"