FROM debian:latest AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    debhelper \
    && rm -rf /var/lib/apt/lists/*

# Copy source into build directory
WORKDIR /build/sources
COPY . .

# Build the package (artifacts go to parent directory /)
RUN dpkg-buildpackage -b -uc -us

# Collect build outputs
RUN mkdir /dist && mv ../*.deb /dist/

# Export stage - extracts only the .deb files
FROM scratch AS export
COPY --from=builder /dist/ /
