#!/bin/bash
set -e

echo "Running all tests..."

# Run Rust tests
echo "Running Rust tests..."
cd desktop-app/src-tauri
~/.cargo/bin/cargo test
cd ../..

# Run Go tests
echo "Running Go tests..."
cd team-server
go test ./...
cd ..

# Run frontend tests (Jest: utils unit tests + command contract tests)
echo "Running frontend tests..."
cd desktop-app
npm test
cd ..

echo "All tests complete!"
