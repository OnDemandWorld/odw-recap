#!/bin/bash
set -euo pipefail

# Anchor to the repo root regardless of the caller's CWD (audit #70).
cd "$(dirname "$0")/.."
ROOT="$(pwd)"

echo "Building Recap solution..."

# Build the Tauri desktop app. The Tauri CLI performs the single release build
# with --features custom-protocol; a separate `cargo build --release` beforehand
# only recompiles the crate graph twice (audit #71). Use cargo from PATH, not a
# hardcoded ~/.cargo/bin/cargo (audit #70).
echo "Building Tauri desktop app..."
cd "$ROOT/desktop-app"
npm ci
npm run tauri build

# Build Go team server
echo "Building team server..."
cd "$ROOT/team-server"
go build -o recap-team-server ./cmd/server

echo "Build complete!"