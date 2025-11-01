#!/bin/bash
# WKMP Packaging Script
# Builds different versions (Full, Lite, Minimal) by compiling specific binaries

set -e  # Exit on error

VERSION=${1:-full}
BUILD_MODE=${2:-release}
OUTPUT_DIR="dist/${VERSION}"

echo "=== WKMP Packaging Script ==="
echo "Version: $VERSION"
echo "Build mode: $BUILD_MODE"
echo "Output directory: $OUTPUT_DIR"
echo ""

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Build flags
if [ "$BUILD_MODE" = "release" ]; then
    BUILD_FLAGS="--release"
    TARGET_DIR="target/release"
else
    BUILD_FLAGS=""
    TARGET_DIR="target/debug"
fi

# Function to build and copy binary
build_binary() {
    local package=$1
    local binary=$2

    echo "Building $package..."
    cargo build -p "$package" $BUILD_FLAGS

    echo "Copying $binary to $OUTPUT_DIR/"
    cp "$TARGET_DIR/$binary" "$OUTPUT_DIR/"
}

case "$VERSION" in
    full)
        echo "Building Full version (6 binaries)..."
        build_binary "wkmp-ap" "wkmp-ap"
        build_binary "wkmp-ui" "wkmp-ui"
        build_binary "wkmp-pd" "wkmp-pd"
        build_binary "wkmp-ai" "wkmp-ai"
        build_binary "wkmp-le" "wkmp-le"
        build_binary "wkmp-dr" "wkmp-dr"
        ;;

    lite)
        echo "Building Lite version (3 binaries)..."
        build_binary "wkmp-ap" "wkmp-ap"
        build_binary "wkmp-ui" "wkmp-ui"
        build_binary "wkmp-pd" "wkmp-pd"
        ;;

    minimal)
        echo "Building Minimal version (2 binaries)..."
        build_binary "wkmp-ap" "wkmp-ap"
        build_binary "wkmp-ui" "wkmp-ui"
        ;;

    *)
        echo "Error: Unknown version '$VERSION'"
        echo "Usage: $0 {full|lite|minimal} [release|debug]"
        exit 1
        ;;
esac

echo ""
echo "=== Packaging complete ==="
echo "Binaries in: $OUTPUT_DIR"
ls -lh "$OUTPUT_DIR"
