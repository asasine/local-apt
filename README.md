# Local APT

Your favorite program only offers an HTTP download but you want APT? No problem! Create your own local APT repository from that download URL and unattended-upgrade to your heart's desire.


# Installation
1. Install `dpkg-dev`
1. Copy the contents of [bin/](bin/) to a location on your `$PATH`. I recommend `/usr/local/bin/`
1. Copy the [conf/](conf/) directory to `/usr/local/mydebs/` as `/usr/local/mydebs/conf/`

# Usage
```bash
sudo update-discord-repo
```
