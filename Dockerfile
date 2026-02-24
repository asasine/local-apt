FROM rust:latest AS build

RUN apt-get update && apt-get install -y \
    build-essential \
    debhelper \
    devscripts

WORKDIR /build/sources

# Copy the debian/control file just to install build dependencies first
COPY debian/control debian/control
RUN apt-get update \
    && mk-build-deps \
        --install \
        --tool 'apt-get -y' \
        debian/control

# Copy the rest of the source code to the build the binary package
COPY . .
RUN --mount=type=cache,target=/build/sources/target \
    dpkg-buildpackage -b -uc -us
RUN mkdir /dist && mv ../*.deb /dist/

FROM scratch AS export
COPY --from=build /dist/ /
