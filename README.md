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
