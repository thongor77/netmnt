//! netmnt — unprivileged CLI client.
//!
//! This is the binary the KDE service menu calls. It translates a user action
//! (mount / mount-as / mount-persistent) into a D-Bus call to `netmntd`.

mod creds;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use netmnt_common::{smb, MountRequest, MountResult, BUS_NAME, INTERFACE_NAME, OBJECT_PATH};
use zbus::proxy;

/// Mount network shares from a single click.
#[derive(Parser)]
#[command(name = "netmnt", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Mount a share for the current session.
    Mount {
        /// Source URL, e.g. smb://lab1.local/isos
        url: String,
        /// Persist the mount across reboots (systemd .mount unit).
        #[arg(long)]
        persistent: bool,
        /// Use an explicit username (implies credential prompt for the password).
        #[arg(long)]
        username: Option<String>,
        /// "Mount as": ask for credentials (or reuse a stored KWallet entry).
        #[arg(long)]
        ask: bool,
    },
    /// Unmount a previously mounted share by its mount point.
    Unmount {
        /// Absolute path of the mount point.
        mount_point: String,
    },
}

#[proxy(
    interface = "org.netmnt.Manager1",
    default_service = "org.netmnt",
    default_path = "/org/netmnt/Manager"
)]
trait Manager {
    async fn mount(&self, request: MountRequest) -> zbus::Result<MountResult>;
    async fn unmount(&self, mount_point: String) -> zbus::Result<()>;
}

/// Turn a file-manager argument (a bare path or a `file://` URL) into a plain
/// local path the daemon can match against `/proc` mount points.
fn normalize_local_path(input: &str) -> String {
    let path = input.strip_prefix("file://").unwrap_or(input);
    let decoded = smb::percent_decode(path);
    let trimmed = decoded.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else {
        trimmed.to_string()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Silence unused-constant warnings until the proxy attributes reference them.
    let _ = (BUS_NAME, INTERFACE_NAME, OBJECT_PATH);

    let cli = Cli::parse();
    let connection = zbus::Connection::system().await?;
    let manager = ManagerProxy::new(&connection).await?;

    match cli.command {
        Command::Mount {
            url,
            persistent,
            username,
            ask,
        } => {
            // The client runs as the user, so it resolves a default mount point
            // under $HOME/mnt and hands the absolute path to the daemon.
            let target = smb::parse_smb_url(&url)?;
            let home = std::env::var("HOME").map_err(|_| anyhow::anyhow!("HOME is not set"))?;
            let base = PathBuf::from(home).join("mnt");
            let mount_point = smb::default_mount_point(&base, &target.share)
                .to_string_lossy()
                .into_owned();

            // Gather credentials only for "mount as" (--ask) or an explicit user;
            // a plain mount stays a guest mount.
            let mut to_store = None;
            let (username, password) = if ask || username.is_some() {
                let wallet_key = smb::unc_path(&target);
                let creds = creds::acquire(&url, &wallet_key, &username.unwrap_or_default())?;
                if creds.freshly_entered && creds.remember && !creds.password.is_empty() {
                    to_store = Some((wallet_key, creds.username.clone(), creds.password.clone()));
                }
                (creds.username, creds.password)
            } else {
                (String::new(), String::new())
            };

            // The client runs as the user, so its own uid/gid are the desired
            // owner of the mounted files.
            let request = MountRequest {
                url,
                mount_point,
                username,
                password,
                persistent,
                // SAFETY: getuid/getgid always succeed and have no preconditions.
                uid: unsafe { libc::getuid() },
                gid: unsafe { libc::getgid() },
            };
            let result = manager.mount(request).await?;
            println!("mounted at {}", result.mount_point);

            // Persist credentials only once the mount actually succeeded.
            if let Some((key, user, pass)) = to_store {
                if let Err(e) = creds::kwallet_write(&key, &user, &pass) {
                    eprintln!("note: could not save credentials to KWallet: {e}");
                }
            }
        }
        Command::Unmount { mount_point } => {
            // Dolphin's %f may hand us a file:// URL with percent-escapes.
            manager.unmount(normalize_local_path(&mount_point)).await?;
            println!("unmounted");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::normalize_local_path;

    #[test]
    fn normalizes_bare_path() {
        assert_eq!(normalize_local_path("/home/u/mnt/Wiki"), "/home/u/mnt/Wiki");
    }

    #[test]
    fn strips_file_scheme_and_decodes() {
        assert_eq!(
            normalize_local_path("file:///home/u/mnt/TV%20Shows/"),
            "/home/u/mnt/TV Shows"
        );
    }
}
