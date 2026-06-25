# AUR packaging

Source of truth for the [`netmnt`](https://aur.archlinux.org/packages/netmnt)
AUR package. The AUR repository (`ssh://aur@aur.archlinux.org/netmnt.git`) only
needs `PKGBUILD` and `.SRCINFO`; this directory keeps them versioned alongside
the code.

## Files

- `PKGBUILD` — builds the workspace in release mode and installs via the
  top-level `Makefile` (`make install DESTDIR=… PREFIX=/usr`), so packaged paths
  stay in sync with a manual install.
- `.SRCINFO` — generated metadata; **must** be regenerated whenever `PKGBUILD`
  changes, or the AUR push is rejected.

## Releasing a new version

```sh
# 1. Tag the new release on GitHub (from the repo root)
git tag -a vX.Y.Z -m "netmnt vX.Y.Z" && git push origin vX.Y.Z

# 2. Update the package metadata
cd packaging/aur
sed -i "s/^pkgver=.*/pkgver=X.Y.Z/; s/^pkgrel=.*/pkgrel=1/" PKGBUILD
updpkgsums                                   # repins sha256sums to the new tarball
makepkg --printsrcinfo > .SRCINFO

# 3. Sanity-check the build
makepkg -f --noconfirm                       # full build + package
namcap *.pkg.tar.zst                          # optional lint

# 4. Publish to the AUR
git clone ssh://aur@aur.archlinux.org/netmnt.git /tmp/aur-netmnt
cp PKGBUILD .SRCINFO /tmp/aur-netmnt/
cd /tmp/aur-netmnt
git commit -am "Update to X.Y.Z-1" && git push

# 5. Commit the same PKGBUILD/.SRCINFO back here so this dir stays authoritative.
```

> Bump `pkgrel` (not `pkgver`) when only the packaging changes, not the source.
