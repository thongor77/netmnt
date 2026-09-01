# AUR packaging

Source of truth for the [`netmnt`](https://aur.archlinux.org/packages/netmnt)
AUR package. The AUR repository (`ssh://aur@aur.archlinux.org/netmnt.git`) only
needs `PKGBUILD` and `.SRCINFO`; this directory keeps them versioned alongside
the code.

## Files

- `PKGBUILD` — builds gettext catalogs with the top-level `Makefile`, builds all
  Cargo workspace members in frozen mode with `/usr/share/locale` embedded,
  then uses `make install DESTDIR=… PREFIX=/usr` only to copy the artifacts.
  Between releases, the source and checksum may pin an exact post-release
  commit with an Arch-style snapshot version such as `0.2.0.r3.ge1c29a0`.
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
