#!/bin/bash
# Build the Debian package and extract to the output directory

set -e

OUTPUT_DIR="${1:-dist}"

echo "Building package..."
docker build --output="$OUTPUT_DIR" --target=export .

echo "Build complete! Package(s) available in: $OUTPUT_DIR"
