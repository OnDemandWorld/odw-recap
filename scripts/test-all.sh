#!/bin/bash
# Run every test suite in the repository.
set -e

# Always resolve paths relative to the repository root, regardless of where
# the script is invoked from.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

CARGO_BIN="cargo"
if [ -x "$HOME/.cargo/bin/cargo" ]; then
    CARGO_BIN="$HOME/.cargo/bin/cargo"
fi

echo "Running all tests..."

echo "Running Rust tests..."
(cd "$REPO_ROOT/desktop-app/src-tauri" && "$CARGO_BIN" test)

echo "Running Go tests..."
(cd "$REPO_ROOT/team-server" && go test ./...)

echo "Running frontend tests..."
(cd "$REPO_ROOT/desktop-app" && npm test)

echo "All tests complete!"
