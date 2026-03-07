# local-apt 8 "February 2026" "local-apt" "System Administration"

## NAME

local-apt - download packages from configured sources and update local APT repository

## SYNOPSIS

**local-apt** update

## DESCRIPTION

**local-apt** downloads Debian package (.deb) files from URLs configured in
_/etc/local-apt/packages.toml_, installs them into the local repository pool,
and updates the repository metadata.

The command parses the TOML configuration file, downloads each enabled
package, extracts package metadata, automatically determines the correct
pool directory path following Debian conventions, and regenerates
repository metadata using **apt-ftparchive**(1).

The command performs the following operations:

- Acquires an exclusive lock to prevent concurrent runs
- Parses the TOML configuration file
- For each enabled package:
  - Downloads the .deb file using **wget**(1) with timestamping to avoid unnecessary downloads
  - Validates it is a valid Debian package
  - Extracts package name and architecture using **dpkg-deb**(1)
  - Auto-generates the target path following Debian pool convention
    (pool/main/\<first-letter\>/\<package-name\>/)
  - Moves the package to the appropriate directory
- Regenerates repository metadata using **apt-ftparchive**(1)
- Logs all operations to syslog

If individual package downloads fail, the command continues processing
remaining packages and updates the repository with partial progress.

## FILES

_/etc/local-apt/packages.toml_
: TOML configuration file defining package sources

_/var/lib/local-apt/pool/main/_
: Package storage directory

_/run/lock/local-apt.lock_
: Lock file to prevent concurrent execution

## ENVIRONMENT

**LOCAL_APT_CONFIG**
: If set, specifies an alternate configuration file location

## EXIT STATUS

**0**
: Success - at least one package was successfully downloaded and repository updated

**1**
: Failure - configuration file not found, already running, or no packages downloaded

## EXAMPLES

Download configured packages and update repository:

```bash
sudo local-apt update
```

Use an alternate configuration file:

```bash
sudo LOCAL_APT_CONFIG=/etc/local-apt/test-packages.toml local-apt update
```

## CONFIGURATION

The configuration file uses TOML format with **\[\[package\]\]** entries.
Each entry defines a package source with a **url** field.

To disable a package, comment out its entry with **#**.

Example configuration:

```toml
# Discord package
[[package]]
url = "https://discord.com/api/download?platform=linux&format=deb"
```

## LOGGING

All operations are logged to syslog with the **local-apt** tag.
Messages are also written to stdout/stderr.

## SEE ALSO

**packages.toml**(5), **apt-ftparchive**(1), **dpkg-deb**(1), **flock**(1)
