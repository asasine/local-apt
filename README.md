# Local APT

Your favorite program only offers an HTTP download but you want APT? No problem! Create your own local APT repository from download URLs and use `apt-get` and `unattended-upgrade` to your heart's desire.

## Installation

```bash
sudo ./install.bash
```

## Configuration

After installation, configure your package sources in `/etc/local-apt/packages.txt`:

```
# Lines starting with # are comments
# One URL per line

https://discord.com/api/download?platform=linux&format=deb
```

Each line contains a URL to download:

- One URL per line
- Lines starting with `#` are comments
- To disable a package, comment it out with `#`
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

## Manual Repository Management

You can also manually add packages and regenerate metadata:

```bash
sudo cp my-package.deb /var/lib/local-apt/pool/main/m/my-package/
sudo update-local-repo
```

## Advanced

### Custom Configuration Location

```bash
sudo LOCAL_APT_CONFIG=/path/to/custom.conf local-apt update
```

See `man local-apt` and `man update-local-repo` for more details.
