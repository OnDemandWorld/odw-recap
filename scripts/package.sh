#!/bin/bash
set -e

echo "Packaging Recap for distribution..."

# Detect platform
PLATFORM=$(uname -s)
ARCH=$(uname -m)

echo "Packaging for $PLATFORM ($ARCH)..."

# Package desktop app
echo "Packaging desktop app..."
cd desktop-app
npm install
npm run tauri build

# Package team server
echo "Packaging team server..."
cd ../team-server
go build -o recap-team-server ./cmd/server

# Create distribution directory
mkdir -p ../dist
cp recap-team-server ../dist/

if [ "$PLATFORM" = "Darwin" ]; then
    cp -r src-tauri/target/release/bundle/dmg/*.dmg ../dist/ 2>/dev/null || true
    cp -r src-tauri/target/release/bundle/macos/*.app ../dist/ 2>/dev/null || true
elif [ "$PLATFORM" = "Linux" ]; then
    cp src-tauri/target/release/bundle/deb/*.deb ../dist/ 2>/dev/null || true
    cp src-tauri/target/release/bundle/appimage/*.AppImage ../dist/ 2>/dev/null || true
elif [ "$PLATFORM" = "MINGW"* ] || [ "$PLATFORM" = "CYGWIN"* ]; then
    cp src-tauri/target/release/bundle/msi/*.msi ../dist/ 2>/dev/null || true
    cp src-tauri/target/release/bundle/nsis/*.exe ../dist/ 2>/dev/null || true
fi

echo "Packaging complete! Artifacts in dist/"
