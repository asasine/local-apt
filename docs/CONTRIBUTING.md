# Contributing

## Rust package

The repository is a standard Rust package and can be built and tested with `cargo`.

When executing locally, the output APT repo directory can be changed with the `-d <PATH>` option on the `update` subcommand.

## Debian packaging

The debian source and binary packages are built in a container with [build.sh](../build.sh). The binary package is copied into [dist](../dist/).

### Manual pages

Manual pages in [install/man/](../install/man/) should be kept up-to-date as changes are made to the project.
