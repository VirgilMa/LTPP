#!/bin/bash
# Build LTPP for WebAssembly

set -e

echo "=== LTPP WASM Build Script ==="

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "wasm-pack not found. Installing..."
    cargo install wasm-pack
fi

echo "Building WASM module..."
wasm-pack build --release --target web

echo ""
echo "Build complete! Output files are in ./pkg/"
echo ""
echo "To run locally:"
echo "  python3 -m http.server 8080"
echo "Then open http://localhost:8080 in your browser"
