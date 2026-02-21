PREFIX = /usr
BINDIR = $(PREFIX)/bin
SYSCONFDIR = /etc
DATADIR = $(PREFIX)/share/local-apt

install:
	# Install scripts
	install -D -m 755 src/bin/update-local-repo $(DESTDIR)$(BINDIR)/update-local-repo
	install -D -m 755 src/bin/update-discord-repo $(DESTDIR)$(BINDIR)/update-discord-repo
	# Install APT sources config
	install -D -m 644 src/local.sources $(DESTDIR)$(SYSCONFDIR)/apt/sources.list.d/local.sources
	# Install apt-ftparchive configs
	install -D -m 644 src/conf/apt.conf $(DESTDIR)$(DATADIR)/conf/apt.conf
	install -D -m 644 src/conf/generate.conf $(DESTDIR)$(DATADIR)/conf/generate.conf
	install -D -m 644 src/conf/tree.conf $(DESTDIR)$(DATADIR)/conf/tree.conf
