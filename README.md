# Local APT

Your favorite program only offers an HTTP download but you want APT? No problem! Create your own local APT repository from download URLs and use `apt-get` and `unattended-upgrade` to your heart's desire.

## Installation

```bash
sudo ./install.bash
```

## Configuration

After installation, configure your package sources in `/etc/local-apt/packages.toml`:

```toml
# Each [[package]] entry defines a package source

# Direct URL
[[package]]
type = "url"
url = "https://discord.com/api/download?platform=linux&format=deb"

# GitHub Release
[[package]]
type = "github-release"
repo = "BurntSushi/ripgrep"
asset_pattern = "ripgrep_.+_amd64\\.deb$"
```

Each `[[package]]` entry defines a package to download. The **type** field selects the source:

- `"url"` — Direct download URL to a `.deb` file
  - **url**: The download URL
- `"github-release"` — `.deb` asset from the latest GitHub Release
  - **repo**: GitHub repository in `owner/repo` format
  - **asset_pattern**: Regex matched against asset filenames
- To disable a package, comment out its entry with `#`

## Usage

Download configured packages and update the repository:

```bash
sudo local-apt update
```

The `update` command will:

1. Download each enabled package from its configured URL
2. Automatically place packages in the correct repository pool location
3. Update repository metadata
4. Log all operations to syslog

Install packages from your local repository:

```bash
sudo apt update
sudo apt install discord
```

Remove old package versions from the pool, keeping only the latest:

```bash
sudo local-apt cleanup
```

## Output and logging

Successful commands write nothing to stdout by default. Exit status is the
default interface for scripts:

- `0` means the command completed, including a partial package update.
- A nonzero status means the command could not complete. Use
  `--fail-on-partial` to also return a nonzero status when one or more packages
  failed but other packages succeeded.

Warnings and errors are written to stderr. Use `-v` for informational progress,
`-vv` for debug details, or `--quiet` for fatal errors only. When no verbosity
flag is present, `RUST_LOG` can override the stderr filter.

Operational details are also sent to syslog with the `local-apt` tag. URLs in
logs and structured output omit user information, query strings, and fragments.

Stdout can provide a stable, versioned report when explicitly requested:

```bash
local-apt update --output=ndjson | jq -c .
local-apt cleanup --output=json | jq .
```

`ndjson` writes one event per line as the command runs. `json` writes one final
document containing the command outcome, summary, and all events. Fatal commands
still produce valid structured output. Filesystem paths are absolute and omit
redundant `.` components.

The default repository uses `/var/lock/local-apt.lock`. A repository selected
with `-d` uses `<repository>/.local-apt.lock`, allowing safe non-root operation
without sharing a lock with other repository roots.
