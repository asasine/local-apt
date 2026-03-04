# packages.txt 5 "March 2026" "local-apt" "File Formats"

## NAME

packages.txt - local-apt package sources configuration file

## SYNOPSIS

_/etc/local-apt/packages.txt_

## DESCRIPTION

The **packages.txt** file defines the URLs of Debian packages (.deb) to be
downloaded and managed by **local-apt**(8). Each URL should point directly to
a .deb file or an endpoint that serves one.

The file is read by **local-apt**(8) during the **update** command. Packages
are downloaded, validated, and placed into the local APT repository at
_/var/lib/local-apt/pool/main/_.

## FORMAT

The file uses a simple line-oriented format:

- Each non-empty, non-comment line is treated as a download URL for a .deb package.
- Lines starting with **#** are comments and are ignored.
- Empty lines are ignored.
- Leading and trailing whitespace on each line is stripped before processing.

There is no support for continuation lines, quoting, or escape sequences.

## EXAMPLES

A typical configuration file:

```ini
# Discord
https://discord.com/api/download?platform=linux&format=deb
```

To temporarily disable a package, comment out its line:

```ini
# Discord (disabled)
# https://discord.com/api/download?platform=linux&format=deb
```

## ENVIRONMENT

**LOCAL_APT_CONFIG**
: If set, **local-apt**(8) reads this path instead of the default
_/etc/local-apt/packages.txt_. The path must be absolute.

## FILES

_/etc/local-apt/packages.txt_
: Default configuration file location.

## SEE ALSO

**local-apt**(8)
