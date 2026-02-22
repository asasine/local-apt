PREFIX = /usr
BINDIR = $(PREFIX)/bin
SYSCONFDIR = /etc
DATADIR = $(PREFIX)/share/local-apt

install:
	# Install Rust binaries
	install -D -m 755 target/release/hello-world $(DESTDIR)$(BINDIR)/hello-world
	# Install shell scripts
	install -D -m 755 src/bin/update-local-repo $(DESTDIR)$(BINDIR)/update-local-repo
	install -D -m 755 src/bin/update-packages $(DESTDIR)$(BINDIR)/update-packages
