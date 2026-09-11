#!/bin/bash
set -e

echo "Building Recap solution..."

# Build Rust desktop app backend
echo "Building Rust backend..."
cd desktop-app/src-tauri
~/.cargo/bin/cargo build --release
cd ../..

# Build Tauri frontend
echo "Building Tauri frontend..."
cd desktop-app
npm install
npm run tauri build
cd ..

# Build Go team server
echo "Building team server..."
cd ../team-server
go build -o recap-team-server ./cmd/server

echo "Build complete!"
