#!/bin/sh
# Produce netmnt-<version>-vendor.tar.xz (Source1 of netmnt.spec): every crate
# from Cargo.lock, vendored so the RPM builds without network access.
#
#   packaging/rpm/make-vendor.sh [output-dir]   # default: ~/rpmbuild/SOURCES
set -eu

root=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$root/Cargo.toml")
outdir=${1:-"$HOME/rpmbuild/SOURCES"}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cargo vendor --locked --quiet --manifest-path "$root/Cargo.toml" "$work/vendor" >/dev/null
mkdir -p "$outdir"
tar -C "$work" -cJf "$outdir/netmnt-$version-vendor.tar.xz" vendor
echo "$outdir/netmnt-$version-vendor.tar.xz"
