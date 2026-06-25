# netmnt install/uninstall.
#
#   make build            # compile release binaries (run as your user)
#   sudo make install     # place binaries + system integration files
#   sudo make reload      # refresh systemd and D-Bus so they pick up the new files
#   sudo make uninstall

PREFIX  ?= /usr
DESTDIR ?=

BINDIR      = $(DESTDIR)$(PREFIX)/bin
DBUS_CONF   = $(DESTDIR)$(PREFIX)/share/dbus-1/system.d
DBUS_SVC    = $(DESTDIR)$(PREFIX)/share/dbus-1/system-services
POLKIT      = $(DESTDIR)$(PREFIX)/share/polkit-1/actions
SYSTEMD     = $(DESTDIR)$(PREFIX)/lib/systemd/system
SERVICEMENU = $(DESTDIR)$(PREFIX)/share/kio/servicemenus

.PHONY: build install reload uninstall

build:
	cargo build --release

install:
	install -Dm755 target/release/netmntd      $(BINDIR)/netmntd
	install -Dm755 target/release/netmnt        $(BINDIR)/netmnt
	install -Dm644 data/dbus/org.netmnt.conf    $(DBUS_CONF)/org.netmnt.conf
	install -Dm644 data/dbus/org.netmnt.service $(DBUS_SVC)/org.netmnt.service
	install -Dm644 data/polkit/org.netmnt.policy $(POLKIT)/org.netmnt.policy
	install -Dm644 data/systemd/netmntd.service  $(SYSTEMD)/netmntd.service
	install -Dm644 data/servicemenus/netmnt.desktop $(SERVICEMENU)/netmnt.desktop
	install -Dm644 data/servicemenus/netmnt-unmount.desktop $(SERVICEMENU)/netmnt-unmount.desktop
	@echo
	@echo "Installed. Now run: sudo make reload"

reload:
	systemctl daemon-reload
	# Pick up the new D-Bus system policy (works for dbus-broker or dbus-daemon).
	systemctl reload dbus 2>/dev/null || systemctl reload dbus-broker 2>/dev/null || true
	@echo "Done. The daemon is D-Bus activated on first 'netmnt' call."

uninstall:
	rm -f $(BINDIR)/netmntd $(BINDIR)/netmnt
	rm -f $(DBUS_CONF)/org.netmnt.conf
	rm -f $(DBUS_SVC)/org.netmnt.service
	rm -f $(POLKIT)/org.netmnt.policy
	rm -f $(SYSTEMD)/netmntd.service
	rm -f $(SERVICEMENU)/netmnt.desktop
	rm -f $(SERVICEMENU)/netmnt-unmount.desktop
	systemctl daemon-reload 2>/dev/null || true
	@echo "Removed."
