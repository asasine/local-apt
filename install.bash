#!/bin/bash
apt-get install dpkg-dev

# recursively copy the contents of root/ to /, prompting before overwriting existing files
cp -riv root/. /
