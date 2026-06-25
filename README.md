# netmnt · v0.1.0

[![Version](https://img.shields.io/badge/version-0.1.0-brightgreen.svg)](https://github.com/thongor77/netmnt/releases/latest)
[![Rust](https://img.shields.io/badge/Rust-2021-orange.svg?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Linux-informational.svg?logo=linux&logoColor=white)](#install--try-it-real-mount)
[![Desktop](https://img.shields.io/badge/desktop-KDE%20%7C%20Dolphin-1d99f3.svg?logo=kde&logoColor=white)](#)
[![Shares](https://img.shields.io/badge/shares-SMB%2FCIFS-purple.svg)](#)
[![Donate](https://img.shields.io/badge/Donate-PayPal-blue.svg)](https://www.paypal.com/donate/?business=JFQGY7NU3ANCN&no_recurring=0&item_name=Every+donation%2C+no+matter+how+small%2C+helps+me+keep+this+project+alive.+Thank+you%21%0A&currency_code=EUR)
[![Bitcoin](https://img.shields.io/badge/Donate-Bitcoin-orange.svg)](#support-the-project)

Mount network shares (SMB/CIFS, and later NFS) from a single click in your file
manager — and unmount them just as easily.

## Problem

Browsing `smb://host/share` in Dolphin gives you a transient KIO/FUSE view, not a
real, predictable mount point usable from the terminal and every other app.
Setting up `mount.cifs`, fstab entries or systemd units by hand is tedious, and
existing GUIs (Smb4K) are standalone apps rather than a right-click action where
you already are.

netmnt adds three actions to the file manager context menu:

- **Mount** — mount the share for this session at a stable path (e.g. `~/mnt/<share>`).
- **Mount as…** — same, with explicit credentials.
- **Mount (persistent)** — register a systemd `.mount` unit so it survives reboot.

## Stack

- **Rust** (workspace, edition 2021)
- **zbus** — D-Bus client/server
- **systemd** — daemon lifecycle + persistent `.mount` units
- **polkit** — privilege authorization
- **KDE ServiceMenus** — Dolphin context-menu integration
- Backend: `mount.cifs` (cifs-utils); KWallet for credential storage

## Architecture (short)

```
Dolphin (right-click smb://) → ServiceMenu .desktop → netmnt (CLI, unprivileged)
                                                          │ D-Bus (system bus)
                                                   netmntd (daemon, privileged)
                                                          │ polkit-authorized
                                                   mount.cifs / systemd .mount
```

- `crates/netmnt` — unprivileged CLI client, invoked by the service menu.
- `crates/netmntd` — privileged daemon owning `org.netmnt` on the system bus.
- `crates/netmnt-common` — shared D-Bus types and constants.

Full detail: [`docs/Architecture.md`](docs/Architecture.md).

## Status

**Working (v0.1.0)** — tested end to end against a real NAS, including across a
reboot. Mount (guest), Mount as… (authenticated, via kdialog + KWallet, password
kept out of argv), persistent mounts (systemd `.mount` units that survive reboot),
and Unmount (which also tears down the unit and cleans up the empty mount-point
directory) all work. Mounts are owned by the calling user. SMB/CIFS only for now;
NFS and SSHFS are next. See [`docs/Roadmap.md`](docs/Roadmap.md).

## Build

```sh
cargo build            # debug
make build             # release binaries used by `make install`
```

## Install & try it (real mount)

> Needs a working SMB share. For a first test use a **guest-readable** share;
> authenticated shares work too via `--ask` (see below).

```sh
make build             # as your user
sudo make install      # binaries + D-Bus/polkit/systemd/servicemenu files
sudo make reload       # refresh systemd + D-Bus

# CLI test (the daemon is D-Bus activated on first call):
netmnt mount smb://nas.local/public
#   → polkit prompts for admin authentication, then:
ls ~/mnt/public
netmnt unmount ~/mnt/public

# Authenticated share ("mount as"): prompts for credentials (kdialog or tty),
# can store them in KWallet, and reuses them next time.
netmnt mount --ask smb://nas.local/wiki
```

To watch the daemon logs during a test, run it in the foreground instead of
relying on activation:

```sh
sudo /usr/bin/netmntd        # terminal A (logs)
netmnt mount smb://nas.local/public   # terminal B
```

In Dolphin: right-click an `smb://` location → **netmnt → Mount**.

Uninstall: `sudo make uninstall`.

## Existing alternatives

Evaluated before starting (see `docs/Decisions-Techniques.md`): **Smb4K**,
**kio-fuse**, **gio mount / gvfs**, fstab/systemd. netmnt's niche is the
right-click-to-stable-path workflow driven by a Rust service.

## Support the project

If netmnt is useful to you, consider supporting its development:

- **PayPal** — [Donate via PayPal](https://www.paypal.com/donate/?business=JFQGY7NU3ANCN&no_recurring=0&item_name=Every+donation%2C+no+matter+how+small%2C+helps+me+keep+this+project+alive.+Thank+you%21%0A&currency_code=EUR)
- **Bitcoin** — `bc1qspe0tky7552qas72wgn8w9dswr0dxlv24w39t6ztjqk3nz6kc5tqv753a4`

## License

MIT — see [LICENSE](LICENSE).
