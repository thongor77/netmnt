//! netmnt — unprivileged CLI client.
//!
//! This is the binary the KDE service menu calls. It translates a user action
//! (mount / mount-as / mount-persistent) into a D-Bus call to `netmntd`.

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
        /// Prompt for / pass an explicit username (mount-as).
        #[arg(long)]
        username: Option<String>,
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
        } => {
            // The client runs as the user, so it resolves a default mount point
            // under $HOME/mnt and hands the absolute path to the daemon.
            let target = smb::parse_smb_url(&url)?;
            let home = std::env::var("HOME").map_err(|_| anyhow::anyhow!("HOME is not set"))?;
            let base = PathBuf::from(home).join("mnt");
            let mount_point = smb::default_mount_point(&base, &target.share)
                .to_string_lossy()
                .into_owned();

            let request = MountRequest {
                url,
                mount_point,
                username: username.unwrap_or_default(),
                // TODO: read the password securely (KWallet / KDialog prompt), never argv.
                password: String::new(),
                persistent,
            };
            let result = manager.mount(request).await?;
            println!("mounted at {}", result.mount_point);
        }
        Command::Unmount { mount_point } => {
            manager.unmount(mount_point).await?;
            println!("unmounted");
        }
    }

    Ok(())
}
