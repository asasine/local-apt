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

[[package]]
type = "url"
url = "https://discord.com/api/download?platform=linux&format=deb"
```

Each `[[package]]` entry defines a package to download:

- **type**: The source type (currently only `"url"`)
- **url**: Direct download URL to a `.deb` file
- To disable a package, comment out its entry with `#`
- Package name is automatically extracted from the downloaded .deb file

## Usage

Download configured packages and update the repository:

```bash
sudo local-apt update
```

The script will:

1. Download each enabled package from its configured URL
2. Automatically place packages in the correct repository pool location
3. Update repository metadata
4. Log all operations to syslog

Install packages from your local repository:

```bash
sudo apt update
sudo apt install discord
```
