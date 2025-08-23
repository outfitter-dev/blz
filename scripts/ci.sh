#!/usr/bin/env bash
# Run full CI checks locally

set -e

echo "🔍 Running full CI checks..."
echo ""

# Format check
echo "📝 Checking formatting..."
cargo fmt -- --check

# Linting
echo "🔍 Running linting..."
./scripts/lint.sh

# Tests
echo "🧪 Running tests..."
make test

# Build
echo "🔨 Building release..."
make release

# Dependency checks
echo "📦 Checking dependencies..."
make check-deps

echo ""
echo "✅ All CI checks passed!"