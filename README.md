# netmnt

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

Early scaffold. The D-Bus contract, project layout and system-integration files
(polkit/dbus/systemd/servicemenu) are in place; the daemon methods are stubs.
See [`docs/Roadmap.md`](docs/Roadmap.md).

## Build

```sh
cargo build
```

## Existing alternatives

Evaluated before starting (see `docs/Decisions-Techniques.md`): **Smb4K**,
**kio-fuse**, **gio mount / gvfs**, fstab/systemd. netmnt's niche is the
right-click-to-stable-path workflow driven by a Rust service.

## License

MIT
