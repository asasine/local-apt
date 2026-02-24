#!/bin/bash
# Build the Debian package and extract to the output directory

set -e

OUTPUT_DIR="${1:-dist}"

echo "Building package..."
DOCKER_BUILDKIT=1 docker build \
  --output="$OUTPUT_DIR" \
  --target=export \
  --build-arg BUILDKIT_INLINE_CACHE=1 \
  .

echo "Build complete! Package(s) available in: $OUTPUT_DIR"
