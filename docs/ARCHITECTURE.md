# Architecture

This document describes the architecture of **local-apt** for contributors.

## Purpose

local-apt creates and manages a local APT repository from HTTP download URLs. Users configure `.deb` download URLs in a TOML config file; the `local-apt update` command downloads each package, validates it, places it in an APT repository on the local filesystem, and regenerates metadata so that `apt-get` and `unattended-upgrade` can consume the packages normally.

## High-Level Flow

```mermaid
flowchart TD
    A[Acquire exclusive lock] --> B[Parse packages.toml]
    B --> Loop

    subgraph Loop [For each URL]
        D[Download .deb to temp dir] --> E[Extract metadata via dpkg-deb]
        E --> F[Move .deb to pool directory]
    end

    Loop --> G{Any successes?}
    G -- Yes --> H[Run apt-ftparchive\ngenerate + release]
    G -- No --> I[Report failure]
    H --> J[Release lock]
    I --> J
```

## Repository layout

The repository is both a native Debian package and Rust package.

```
.
├── build.sh                      # Builds the .deb via Docker
├── Cargo.toml                    # Rust project manifest
├── Dockerfile                    # Multi-stage build: compile + package
├── src/                          # Rust source (the local-apt binary)
├── install/                      # Static files installed by the .deb package
│   ├── packages.toml             # Default config file: /etc/local-apt/packages.toml
│   ├── local.sources             # APT sources entry: /etc/apt/sources.list.d/
│   ├── apt-ftparchive-conf/      # Configuration files for apt-ftparchive
│   └── man/                      # Manual pages
|
└── debian/                       # Debian packaging metadata
```

## Variable data

The `local-apt` command maintains variable data in the installed filesystem.

### APT repo

The APT repo is stored in `/var/lib/local-apt/` and follows standard a APT repository structure. One suite (`stable`) is available with one component (`main`).

- `/var/lib/local-apt/pool/main/`: downloaded `.deb` files in directories based on the package name
- `/var/lib/local-apt/dists/stable/main/`: the `main` component of the `stable` suite. See `sources.list(5)`.

### Lock file

- `/var/lock/local-apt.lock`: locked when `local-apt` runs to ensure only one process modifies `/var/lib/local-apt/`.
- `<repository>/.local-apt.lock`: used for a repository selected with `-d`, so
  non-root and concurrent multi-repository runs retain independent locking.

## Output, logging, and errors

The binary keeps machine output separate from operational diagnostics:

- `Reporter` in `src/output.rs` owns stdout. It is silent unless JSON or NDJSON
  output is requested and emits versioned domain events rather than tracing
  metadata.
- `src/logging.rs` configures independent tracing layers. Syslog receives
  informational operational detail under the `local-apt` identity; stderr
  receives warnings and errors by default and more detail when requested.
- `AppError` in `src/main.rs` is the command boundary. Fatal errors are logged
  once, represented in requested structured output, and mapped to a deliberate
  exit status.
- Errors from external commands retain the command name, status, and bounded
  stderr. Child stdout and stderr are captured so they cannot violate the CLI
  stream contract.

New command behavior should emit a typed `Event` for facts useful to scripts and
a tracing event for operational diagnosis. Do not put human progress on stdout,
serialize Rust debug output as a public interface, or log complete download URLs.
Source labels must omit credentials, query strings, and fragments.

JSON output is buffered so failures still produce one valid final document.
NDJSON is streamed one record at a time. A closed pipe disables further output
without aborting repository mutation midway.
