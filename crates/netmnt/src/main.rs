//! netmnt — unprivileged CLI client.
//!
//! This is the binary the KDE service menu calls. It translates a user action
//! (mount / mount-as / mount-persistent) into a D-Bus call to `netmntd`.

mod creds;

use std::path::PathBuf;

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use netmnt_common::i18n::{self, tr, tr_args};
use netmnt_common::{nfs, smb, MountRequest, MountResult, BUS_NAME, INTERFACE_NAME, OBJECT_PATH};
use zbus::proxy;

/// A parsed mount source, dispatched by URL scheme.
///
/// NFS access is granted by the server's export ACL, not username/password,
/// so there is no `Nfs` credential path — see `run()`.
enum Target {
    Smb(smb::SmbTarget),
    Nfs(nfs::NfsTarget),
}

fn parse_url(url: &str) -> anyhow::Result<Target> {
    if let Ok(t) = smb::parse_smb_url(url) {
        return Ok(Target::Smb(t));
    }
    if let Ok(t) = nfs::parse_nfs_url(url) {
        return Ok(Target::Nfs(t));
    }
    let message = tr_args(
        "Unsupported URL scheme (expected smb:// or nfs://): {url}",
        &[("url", url)],
    );
    anyhow::bail!(message)
}

// User-visible descriptions are applied by localized_cli_command().
#[derive(Parser)]
#[command(name = "netmnt", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Mount {
        url: String,
        #[arg(long)]
        persistent: bool,
        #[arg(long)]
        username: Option<String>,
        #[arg(long)]
        ask: bool,
    },
    Unmount {
        mount_point: String,
    },
}

fn localized_cli_command() -> clap::Command {
    Cli::command()
        .about(tr("Mount network shares from a single click"))
        .mut_subcommand("mount", |command| {
            command
                .about(tr("Mount a share for the current session"))
                .mut_arg("url", |arg| {
                    arg.help(tr("Source URL, for example smb://nas.local/share"))
                })
                .mut_arg("persistent", |arg| {
                    arg.help(tr(
                        "Keep the mount across reboots using a systemd mount unit",
                    ))
                })
                .mut_arg("username", |arg| {
                    arg.help(tr("Use an explicit username and prompt for its password"))
                })
                .mut_arg("ask", |arg| {
                    arg.help(tr(
                        "Ask for credentials or reuse credentials stored in KWallet",
                    ))
                })
        })
        .mut_subcommand("unmount", |command| {
            command
                .about(tr("Unmount a share by its mount point"))
                .mut_arg("mount_point", |arg| {
                    arg.help(tr("Absolute path of the mount point"))
                })
        })
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

/// Show a desktop notification. Launched from Dolphin the CLI has no visible
/// stdout/stderr, so this is the only way the user learns a mount failed.
/// Best-effort: a missing `notify-send` is silently ignored.
fn notify(summary: &str, body: &str, error: bool) {
    let _ = std::process::Command::new("notify-send")
        .args([
            "-a",
            "netmnt",
            "-u",
            if error { "critical" } else { "normal" },
            "-i",
            "folder-network",
            summary,
            body,
        ])
        .status();
}

#[tokio::main]
async fn main() {
    i18n::init();
    let cli = Cli::from_arg_matches(&localized_cli_command().get_matches())
        .expect("clap generated invalid command-line matches");
    match run(cli).await {
        Ok(message) => {
            println!("{message}");
            notify("netmnt", &message, false);
        }
        Err(e) => {
            let message = tr_args("Error: {error}", &[("error", &e.to_string())]);
            eprintln!("{message}");
            notify(&tr("netmnt — failure"), &e.to_string(), true);
            std::process::exit(1);
        }
    }
}

/// Run the requested command, returning a human-readable success message.
async fn run(cli: Cli) -> anyhow::Result<String> {
    // Silence unused-constant warnings until the proxy attributes reference them.
    let _ = (BUS_NAME, INTERFACE_NAME, OBJECT_PATH);

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
            let target = parse_url(&url)?;
            let home_error = tr("HOME is not set");
            let home = std::env::var("HOME").map_err(|_| anyhow::anyhow!(home_error))?;
            let base = PathBuf::from(home).join("mnt");
            let mount_point = match &target {
                Target::Smb(t) => smb::default_mount_point(&base, &t.share),
                Target::Nfs(t) => nfs::default_mount_point(&base, &t.export),
            }
            .to_string_lossy()
            .into_owned();

            // Gather credentials only for SMB "mount as" (--ask) or an explicit
            // user; a plain mount stays a guest mount. NFS has no
            // username/password equivalent (access is server-ACL based), so
            // --ask/--username are silently ignored there.
            let mut to_store = None;
            let (username, password) = match &target {
                Target::Smb(t) if ask || username.is_some() => {
                    let wallet_key = smb::unc_path(t);
                    let creds = creds::acquire(&url, &wallet_key, &username.unwrap_or_default())?;
                    if creds.freshly_entered && creds.remember && !creds.password.is_empty() {
                        to_store =
                            Some((wallet_key, creds.username.clone(), creds.password.clone()));
                    }
                    (creds.username, creds.password)
                }
                _ => (String::new(), String::new()),
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

            // Persist credentials only once the mount actually succeeded.
            if let Some((key, user, pass)) = to_store {
                if let Err(e) = creds::kwallet_write(&key, &user, &pass) {
                    eprintln!(
                        "{}",
                        tr_args(
                            "Note: could not save credentials to KWallet: {error}",
                            &[("error", &e.to_string())],
                        )
                    );
                }
            }

            Ok(tr_args(
                "Mounted at: {path}",
                &[("path", &result.mount_point)],
            ))
        }
        Command::Unmount { mount_point } => {
            // Dolphin's %f may hand us a file:// URL with percent-escapes.
            let path = normalize_local_path(&mount_point);
            manager.unmount(path.clone()).await?;
            Ok(tr_args("Unmounted: {path}", &[("path", &path)]))
        }
    }
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
