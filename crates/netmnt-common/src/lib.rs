//! Shared types and constants for the netmnt daemon and CLI client.
//!
//! The D-Bus interface is the single contract between the unprivileged client
//! (`netmnt`) and the privileged daemon (`netmntd`). Wire types stay primitive
//! on purpose: D-Bus has no native tagged union, so credential/lifetime choices
//! are encoded as plain fields rather than Rust enums.

use serde::{Deserialize, Serialize};
use zvariant::Type;

pub mod smb;

/// Well-known bus name owned by the daemon on the system bus.
pub const BUS_NAME: &str = "org.netmnt";
/// Object path of the manager object.
pub const OBJECT_PATH: &str = "/org/netmnt/Manager";
/// Manager interface name.
pub const INTERFACE_NAME: &str = "org.netmnt.Manager1";

/// A single mount request sent from the client to the daemon.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MountRequest {
    /// Source URL, e.g. `smb://lab1.local/isos`.
    pub url: String,
    /// Desired mount point. Empty means "let the daemon choose" (e.g. `~/mnt/<share>`).
    pub mount_point: String,
    /// Username. Empty means a guest mount.
    pub username: String,
    /// Password passed inline for this request. Empty when guest or when the
    /// daemon should resolve it from the secret store (KWallet).
    ///
    /// Never logged; for persistent mounts the daemon moves it into KWallet.
    pub password: String,
    /// When true, register a persistent systemd `.mount` unit (survives reboot).
    pub persistent: bool,
    /// Owner uid for the mounted files (the calling user), passed as `uid=`.
    pub uid: u32,
    /// Owner gid for the mounted files (the calling user), passed as `gid=`.
    pub gid: u32,
}

/// Result returned to the client after a successful mount.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MountResult {
    /// Absolute path where the share is now mounted.
    pub mount_point: String,
    /// True when a persistent systemd unit was created.
    pub persisted: bool,
}
