#!/bin/bash
# Build every component of the Recap solution.
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

CARGO_BIN="cargo"
if [ -x "$HOME/.cargo/bin/cargo" ]; then
    CARGO_BIN="$HOME/.cargo/bin/cargo"
fi

echo "Building Recap solution..."

echo "Building Rust backend..."
(cd "$REPO_ROOT/desktop-app/src-tauri" && "$CARGO_BIN" build --release)

echo "Building Tauri desktop app..."
(cd "$REPO_ROOT/desktop-app" && npm install && npm run tauri build)

echo "Building team server..."
(cd "$REPO_ROOT/team-server" && go build -o recap-team-server ./cmd/server)

echo "Build complete!"
