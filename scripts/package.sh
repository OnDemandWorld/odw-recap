#!/bin/bash
# Package Recap artifacts for distribution.
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

PLATFORM=$(uname -s)
ARCH=$(uname -m)

echo "Packaging for $PLATFORM ($ARCH)..."

echo "Packaging desktop app..."
(cd "$REPO_ROOT/desktop-app" && npm install && npm run tauri build)

echo "Packaging team server..."
(cd "$REPO_ROOT/team-server" && go build -o recap-team-server ./cmd/server)

DIST_DIR="$REPO_ROOT/dist"
mkdir -p "$DIST_DIR"
cp "$REPO_ROOT/team-server/recap-team-server" "$DIST_DIR/"

if [ "$PLATFORM" = "Darwin" ]; then
    cp -r "$REPO_ROOT"/desktop-app/src-tauri/target/release/bundle/dmg/*.dmg "$DIST_DIR/" 2>/dev/null || true
    cp -r "$REPO_ROOT"/desktop-app/src-tauri/target/release/bundle/macos/*.app "$DIST_DIR/" 2>/dev/null || true
elif [ "$PLATFORM" = "Linux" ]; then
    cp "$REPO_ROOT"/desktop-app/src-tauri/target/release/bundle/deb/*.deb "$DIST_DIR/" 2>/dev/null || true
    cp "$REPO_ROOT"/desktop-app/src-tauri/target/release/bundle/appimage/*.AppImage "$DIST_DIR/" 2>/dev/null || true
elif [[ "$PLATFORM" == MINGW* ]] || [[ "$PLATFORM" == CYGWIN* ]]; then
    cp "$REPO_ROOT"/desktop-app/src-tauri/target/release/bundle/msi/*.msi "$DIST_DIR/" 2>/dev/null || true
    cp "$REPO_ROOT"/desktop-app/src-tauri/target/release/bundle/nsis/*.exe "$DIST_DIR/" 2>/dev/null || true
fi

echo "Packaging complete! Artifacts in $DIST_DIR"
