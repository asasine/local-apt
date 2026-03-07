# packages.toml 5 "March 2026" "local-apt" "File Formats"

## NAME

packages.toml - local-apt package sources configuration file

## SYNOPSIS

_/etc/local-apt/packages.toml_

## DESCRIPTION

The **packages.toml** file defines the package sources to be downloaded and
managed by **local-apt**(8).

The file is read by **local-apt**(8) during the **update** command. Packages
are downloaded, validated, and placed into the local APT repository at
_/var/lib/local-apt/pool/main/_.

## FORMAT

The file uses TOML format (https://toml.io). Each package source is defined
as a **\[\[package\]\]** entry (a TOML array of tables).

### Fields

**type** (string, required)
: The source type. Must be one of the types listed below.

### Type: url

Download a .deb package directly from a URL.

**url** (string, required)
: The download URL for the .deb package.

### Type: github-release

Download a .deb package from the latest release of a GitHub repository.

**repo** (string, required)
: The GitHub repository in `owner/repo` format.

**asset_pattern** (string, required)
: A regular expression matched against release asset filenames. The first
asset whose name matches is downloaded.

### Comments

Lines starting with **#** are comments and are ignored by the TOML parser.

## EXAMPLES

A typical configuration file:

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

Multiple packages:

```toml
[[package]]
type = "url"
url = "https://discord.com/api/download?platform=linux&format=deb"

[[package]]
type = "github-release"
repo = "BurntSushi/ripgrep"
asset_pattern = "ripgrep_.+_amd64\\.deb$"
```

To temporarily disable a package, comment out its entry:

```toml
# Discord (disabled)
# [[package]]
# type = "url"
# url = "https://discord.com/api/download?platform=linux&format=deb"
```

## ENVIRONMENT

**LOCAL_APT_CONFIG**
: If set, **local-apt**(8) reads this path instead of the default
_/etc/local-apt/packages.toml_. The path must be absolute.

## FILES

_/etc/local-apt/packages.toml_
: Default configuration file location.

## SEE ALSO

**local-apt**(8)
