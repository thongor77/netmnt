# RPM packaging

`netmnt.spec` builds netmnt for Fedora (and other RPM distributions with
`systemd-rpm-macros`). The build is fully offline: Rust crates come from a
vendor tarball, so it also works under `mock` and COPR.

## Files

- `netmnt.spec` — builds the gettext catalogs and the Cargo workspace (frozen,
  offline, `/usr/share/locale` embedded), then reuses `make install
  DESTDIR=… PREFIX=/usr` to lay out the files, like the AUR `PKGBUILD`.
- `make-vendor.sh` — creates `netmnt-<version>-vendor.tar.xz` (`Source1`) with
  `cargo vendor --locked`.

## Building locally

```sh
sudo dnf install rpm-build rpmdevtools cargo rust gcc make gettext systemd-rpm-macros
rpmdev-setuptree

# Sources: release tarball (Source0) + vendored crates (Source1)
spectool -g -R packaging/rpm/netmnt.spec
git checkout vX.Y.Z && packaging/rpm/make-vendor.sh   # must match the tag's Cargo.lock

rpmbuild -ba packaging/rpm/netmnt.spec
sudo dnf install ~/rpmbuild/RPMS/x86_64/netmnt-*.rpm
```

The daemon is D-Bus activated on the first `netmnt` call; no service needs to
be enabled. After installing, restart Dolphin so it picks up the service menus.

## Building in mock / COPR

```sh
rpmbuild -bs packaging/rpm/netmnt.spec            # → ~/rpmbuild/SRPMS/netmnt-*.src.rpm
mock -r fedora-rawhide-x86_64 ~/rpmbuild/SRPMS/netmnt-*.src.rpm
copr-cli build <project> ~/rpmbuild/SRPMS/netmnt-*.src.rpm
```

## Releasing a new version

1. Tag the release (see `packaging/aur/README.md`).
2. In `netmnt.spec`: bump `Version`, reset `Release` to `1`, add a
   `%changelog` entry (`rpmdev-bumpspec -n X.Y.Z netmnt.spec` does all three).
3. Regenerate the vendor tarball from the new tag and rebuild.

> Bump `Release` (not `Version`) when only the packaging changes.
