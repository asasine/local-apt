# local-apt 8 "February 2026" "local-apt" "System Administration"

## NAME

local-apt - download packages from configured sources and update local APT repository

## SYNOPSIS

**local-apt** update [**OPTIONS**]

**local-apt** cleanup [**OPTIONS**]

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

## COMMANDS

**update**
: Download packages from configured sources and update the repository.

**cleanup**
: Remove old package versions from the pool, keeping only the latest version
of each package. Version comparison is performed using **dpkg**(1). After
removing old versions, repository metadata is regenerated.

## OPTIONS

**-d**, **--repository-directory** _PATH_
: Store repository state at _PATH_ instead of _/var/lib/local-apt/_.

**--output** _FORMAT_
: Write a stable machine-readable report to stdout. _FORMAT_ is **ndjson** for
one event per line or **json** for one final document containing all events.
With no output option, successful commands write nothing to stdout.

**-v**, **--verbose**
: Write informational progress to stderr. Repeat as **-vv** for debug details.

**-q**, **--quiet**
: Only write fatal errors to stderr. Conflicts with **--verbose**.

**--fail-on-partial**
: Return a nonzero status when any configured package fails, even if another
package was downloaded or was already up to date.

## FILES

_/etc/local-apt/packages.toml_
: TOML configuration file defining package sources

_/var/lib/local-apt/pool/main/_
: Package storage directory

_/run/lock/local-apt.lock_
: Lock file to prevent concurrent execution

_/lib/systemd/system/local-apt.timer_
: Systemd timer unit for daily execution

_/lib/systemd/system/local-apt.service_
: Systemd service unit invoked by the timer

## ENVIRONMENT

**LOCAL_APT_CONFIG**
: If set, specifies an alternate configuration file location

**RUST_LOG**
: Overrides the default warning-level stderr filter when neither **--verbose**
nor **--quiet** is supplied. Explicit verbosity options take precedence.

## EXIT STATUS

**0**
: Success. Package-level partial success also returns zero unless
**--fail-on-partial** is used.

**1**
: Command-wide failure, all configured packages failed, output failed, or partial
success with **--fail-on-partial**.

## EXAMPLES

Download configured packages and update repository:

```bash
sudo local-apt update
```

Remove old package versions from the pool:

```bash
sudo local-apt cleanup
```

Use an alternate configuration file:

```bash
sudo LOCAL_APT_CONFIG=/etc/local-apt/test-packages.toml local-apt update
```

Process a stream of update events:

```bash
local-apt update --output=ndjson | jq -c 'select(.event == "downloaded")'
```

Read the final cleanup summary:

```bash
local-apt cleanup --output=json | jq '.summary'
```

## CONFIGURATION

The configuration file uses TOML format with **\[\[package\]\]** entries.
Each entry requires a **type** field to specify the source type, along with
type-specific fields.

To disable a package, comment out its entry with **#**.

Example configuration:

```toml
# Discord (direct URL)
[[package]]
type = "url"
url = "https://discord.com/api/download?platform=linux&format=deb"

# ripgrep (GitHub Release)
[[package]]
type = "github-release"
repo = "BurntSushi/ripgrep"
asset_pattern = "ripgrep_.+_amd64\\.deb$"
```

## TIMER

A systemd timer unit is included to run **local-apt update** and
**local-apt cleanup** daily. The timer uses a randomized delay of up to
one hour to avoid fixed scheduling, and is persistent so missed runs are
caught up after reboot.

To enable the timer:

```bash
sudo systemctl enable --now local-apt.timer
```

To check timer status:

```bash
systemctl status local-apt.timer
```

## LOGGING

Operational events are logged to syslog with the **local-apt** tag. Syslog uses
informational detail independently of the interactive stderr filter. If syslog
is unavailable, the command continues and writes a warning to stderr.

Warnings and errors are written to stderr by default. Informational and debug
progress can be enabled with **--verbose**. Human-readable logs never use stdout.

Structured stdout uses schema version 1. URLs in all outputs omit user
information, query strings, and fragments. **ndjson** emits events as they occur
and ends with a **finished** event. **json** emits one document with
**schema_version**, **command**, **outcome**, **summary**, and **events** fields.
If a pipe consumer closes early, repository maintenance continues rather than
stopping after a partial mutation.

## SEE ALSO

**packages.toml**(5), **apt-ftparchive**(1), **dpkg-deb**(1), **flock**(1),
**systemd.timer**(5), **systemd.service**(5)
