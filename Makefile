# netmnt install/uninstall.
#
#   make build            # compile release binaries (run as your user)
#   sudo make install     # place binaries + system integration files
#   sudo make reload      # refresh systemd and D-Bus so they pick up the new files
#   sudo make uninstall
#   make i18n             # validate and compile translation catalogs locally
#   make update-translations # refresh po/netmnt.pot and merge language catalogs

PREFIX  ?= /usr
DESTDIR ?=

BINDIR      = $(DESTDIR)$(PREFIX)/bin
DBUS_CONF   = $(DESTDIR)$(PREFIX)/share/dbus-1/system.d
DBUS_SVC    = $(DESTDIR)$(PREFIX)/share/dbus-1/system-services
POLKIT      = $(DESTDIR)$(PREFIX)/share/polkit-1/actions
SYSTEMD     = $(DESTDIR)$(PREFIX)/lib/systemd/system
SERVICEMENU = $(DESTDIR)$(PREFIX)/share/kio/servicemenus
LOCALEDIR   = $(DESTDIR)$(PREFIX)/share/locale

LINGUAS = fr
PACKAGE_VERSION = $(shell sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml)
RUST_I18N_SOURCES = \
	crates/netmnt-common/src/i18n.rs \
	crates/netmnt/src/main.rs \
	crates/netmnt/src/creds.rs \
	crates/netmntd/src/main.rs \
	crates/netmntd/src/exec.rs

.PHONY: build i18n update-translations install reload uninstall clean

build:
	cargo build --release

i18n:
	@for lang in $(LINGUAS); do \
		install -d build/locale/$$lang/LC_MESSAGES; \
		msgfmt --check --check-format \
			-o build/locale/$$lang/LC_MESSAGES/netmnt.mo po/$$lang.po; \
	done

update-translations:
	install -d po
	xgettext --language=Rust --from-code=UTF-8 --keyword=tr:1 --keyword=tr_args:1 \
		--package-name=netmnt --package-version=$(PACKAGE_VERSION) \
		--copyright-holder="netmnt contributors" \
		--msgid-bugs-address="https://github.com/thongor77/netmnt/issues" \
		--output=po/netmnt.pot $(RUST_I18N_SOURCES)
	@for lang in $(LINGUAS); do \
		msgmerge --update --backup=none po/$$lang.po po/netmnt.pot; \
	done

install: i18n
	install -Dm755 target/release/netmntd      $(BINDIR)/netmntd
	install -Dm755 target/release/netmnt        $(BINDIR)/netmnt
	install -Dm644 data/dbus/org.netmnt.conf    $(DBUS_CONF)/org.netmnt.conf
	install -Dm644 data/dbus/org.netmnt.service $(DBUS_SVC)/org.netmnt.service
	install -Dm644 data/polkit/org.netmnt.policy $(POLKIT)/org.netmnt.policy
	install -Dm644 data/systemd/netmntd.service  $(SYSTEMD)/netmntd.service
	install -Dm644 data/servicemenus/netmnt.desktop $(SERVICEMENU)/netmnt.desktop
	install -Dm644 data/servicemenus/netmnt-unmount.desktop $(SERVICEMENU)/netmnt-unmount.desktop
	@for lang in $(LINGUAS); do \
		install -Dm644 build/locale/$$lang/LC_MESSAGES/netmnt.mo \
			$(LOCALEDIR)/$$lang/LC_MESSAGES/netmnt.mo; \
	done
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
	@for lang in $(LINGUAS); do \
		rm -f $(LOCALEDIR)/$$lang/LC_MESSAGES/netmnt.mo; \
	done
	systemctl daemon-reload 2>/dev/null || true
	@echo "Removed."

clean:
	rm -rf build/locale
