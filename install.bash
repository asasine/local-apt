#!/bin/bash
apt-get install dpkg-dev
mkdir -p /usr/local/mydebs/
cp -riv conf/ /usr/local/mydebs/
cp -iv bin/* /usr/local/bin/
