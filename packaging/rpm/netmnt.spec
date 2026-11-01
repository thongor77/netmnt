# RPM spec for netmnt (Fedora / RHEL-likes).
#
# Builds fully offline: crates come from a vendor tarball (Source1) produced by
# `packaging/rpm/make-vendor.sh`, so the spec also works under mock and COPR
# where the build root has no network access.

# Cargo release builds carry no debug info; skip the empty debuginfo package.
%global debug_package %{nil}

Name:           netmnt
Version:        0.3.0
Release:        1%{?dist}
Summary:        Mount SMB/NFS network shares from Dolphin's right-click menu

License:        MIT
URL:            https://github.com/thongor77/netmnt
Source0:        %{url}/archive/refs/tags/v%{version}/%{name}-%{version}.tar.gz
Source1:        %{name}-%{version}-vendor.tar.xz

ExclusiveArch:  %{rust_arches}

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  gcc
BuildRequires:  make
BuildRequires:  gettext
BuildRequires:  systemd-rpm-macros

Requires:       cifs-utils
Requires:       nfs-utils
Requires:       polkit
Requires:       dbus
Recommends:     kdialog
Recommends:     libnotify
Suggests:       dolphin
Suggests:       kwalletmanager

%description
netmnt mounts SMB/CIFS and NFS network shares from the Dolphin context menu
(Mount, Mount as..., Mount persistent, Unmount). An unprivileged CLI talks over
the D-Bus system bus to a privileged Rust daemon (netmntd) that performs the
mounts, with every mutating call authorized by polkit. Persistent mounts are
managed as systemd .mount units; SMB credentials can be kept in KWallet.

%prep
%autosetup -n %{name}-%{version}
# Vendored crates: point cargo at them and forbid network access.
tar -xJf %{SOURCE1}
mkdir -p .cargo
cat > .cargo/config.toml <<'EOF'
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
EOF

%build
export CARGO_TARGET_DIR=target
make i18n
NETMNT_DEFAULT_LOCALE_DIR=%{_datadir}/locale \
    cargo build --frozen --offline --release --all
# Stamp the prefix so `make install` accepts the build (see Makefile).
install -d build
printf '%s\n' %{_prefix} > build/install-prefix

%install
# Reuse the Makefile so packaged paths stay in sync with `make install`.
%make_install PREFIX=%{_prefix}
%find_lang %{name}

%check
export CARGO_TARGET_DIR=target
cargo test --frozen --offline --release --all

%post
%systemd_post netmntd.service

%preun
%systemd_preun netmntd.service

%postun
%systemd_postun_with_restart netmntd.service

%files -f %{name}.lang
%license LICENSE
%doc README.md
%{_bindir}/netmnt
%{_bindir}/netmntd
%{_datadir}/dbus-1/system.d/org.netmnt.conf
%{_datadir}/dbus-1/system-services/org.netmnt.service
%{_datadir}/polkit-1/actions/org.netmnt.policy
%{_unitdir}/netmntd.service
%dir %{_datadir}/kio
%dir %{_datadir}/kio/servicemenus
%{_datadir}/kio/servicemenus/netmnt.desktop
%{_datadir}/kio/servicemenus/netmnt-unmount.desktop

%changelog
* Sun Nov 01 2026 Modo Ltunzher <offnormal@gmail.com> - 0.3.0-1
- Initial RPM packaging
