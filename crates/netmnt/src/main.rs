//! netmnt — unprivileged CLI client.
//!
//! This is the binary the KDE service menu calls. It translates a user action
//! (mount / mount-as / mount-persistent) into a D-Bus call to `netmntd`.

use clap::{Parser, Subcommand};
use netmnt_common::{MountRequest, MountResult, BUS_NAME, INTERFACE_NAME, OBJECT_PATH};
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
            let request = MountRequest {
                url,
                mount_point: String::new(),
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
