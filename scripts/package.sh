#!/bin/bash
set -euo pipefail

# Anchor to the repo root regardless of the caller's CWD (audit #20: relative
# cd/cp paths previously resolved from team-server/ and silently copied nothing).
cd "$(dirname "$0")/.."
ROOT="$(pwd)"

echo "Packaging Recap for distribution..."

# Detect platform
PLATFORM=$(uname -s)
ARCH=$(uname -m)

echo "Packaging for $PLATFORM ($ARCH)..."

# Package desktop app
echo "Packaging desktop app..."
cd "$ROOT/desktop-app"
npm ci
npm run tauri build

# Package team server
echo "Packaging team server..."
cd "$ROOT/team-server"
go build -o recap-team-server ./cmd/server

# Create distribution directory
mkdir -p "$ROOT/dist"
cp recap-team-server "$ROOT/dist/"

# Desktop bundles live under desktop-app/src-tauri/target/release/bundle —
# reference them from the repo root, not the team-server CWD. Errors are no
# longer suppressed: a missing bundle for the current platform fails loudly
# instead of shipping a dist/ with only the Go binary.
BUNDLE="$ROOT/desktop-app/src-tauri/target/release/bundle"
if [ "$PLATFORM" = "Darwin" ]; then
    cp -r "$BUNDLE"/dmg/*.dmg "$ROOT/dist/"
    cp -r "$BUNDLE"/macos/*.app "$ROOT/dist/"
elif [ "$PLATFORM" = "Linux" ]; then
    cp "$BUNDLE"/deb/*.deb "$ROOT/dist/"
    cp "$BUNDLE"/appimage/*.AppImage "$ROOT/dist/"
elif [ "$PLATFORM" = "MINGW"* ] || [ "$PLATFORM" = "CYGWIN"* ]; then
    cp "$BUNDLE"/msi/*.msi "$ROOT/dist/"
    cp "$BUNDLE"/nsis/*.exe "$ROOT/dist/"
fi

echo "Packaging complete! Artifacts in dist/"