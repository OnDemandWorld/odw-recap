#!/bin/bash
set -euo pipefail

# Anchor to the repo root regardless of the caller's CWD (audit #72).
cd "$(dirname "$0")/.."
ROOT="$(pwd)"

echo "Running all tests..."

# Run Rust tests (cargo from PATH, not a hardcoded ~/.cargo/bin/cargo).
echo "Running Rust tests..."
cd "$ROOT/desktop-app/src-tauri"
cargo test

# Run Go tests
echo "Running Go tests..."
cd "$ROOT/team-server"
go test ./...

# Run frontend tests (Jest: utils unit tests + command contract tests).
# Install deps first so a fresh clone has jest available (audit #72).
echo "Running frontend tests..."
cd "$ROOT/desktop-app"
npm ci
npm test

echo "All tests complete!"