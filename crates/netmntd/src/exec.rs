//! Mount execution: turns a validated [`MountRequest`] into an actual
//! `mount.cifs` invocation. Requires CAP_SYS_ADMIN (the daemon runs as root).
//!
//! Credentials never appear on the command line: the password is passed through
//! the `PASSWD` environment variable that `mount.cifs` reads.

use std::path::Path;

use netmnt_common::{smb, MountRequest, MountResult};
use tokio::process::Command;

/// Mount the share described by `request`.
pub async fn perform_mount(request: &MountRequest) -> anyhow::Result<MountResult> {
    if request.persistent {
        // Persistent mounts (systemd .mount unit generation) land in Phase 3.
        anyhow::bail!("persistent mounts are not implemented yet");
    }

    let target = smb::parse_smb_url(&request.url)?;
    let source = smb::unc_path(&target);

    if request.mount_point.is_empty() {
        anyhow::bail!("mount_point must be provided by the client");
    }
    let mount_point = Path::new(&request.mount_point);

    tokio::fs::create_dir_all(mount_point).await.map_err(|e| {
        anyhow::anyhow!("cannot create mount point {}: {e}", mount_point.display())
    })?;

    let mut options = vec!["rw".to_string()];
    if request.username.is_empty() {
        options.push("guest".to_string());
    } else {
        options.push(format!("username={}", request.username));
    }

    let mut cmd = Command::new("mount.cifs");
    cmd.arg(&source)
        .arg(mount_point)
        .arg("-o")
        .arg(options.join(","));

    // Pass the password out-of-band so it never shows up in `ps`/argv.
    if !request.username.is_empty() && !request.password.is_empty() {
        cmd.env("PASSWD", &request.password);
    }

    let output = cmd.output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("mount.cifs failed ({}): {}", output.status, stderr.trim());
    }

    Ok(MountResult {
        mount_point: mount_point.to_string_lossy().into_owned(),
        persisted: false,
    })
}

/// Unmount the share currently mounted at `mount_point`.
pub async fn perform_unmount(mount_point: &str) -> anyhow::Result<()> {
    let output = Command::new("umount").arg(mount_point).output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("umount failed ({}): {}", output.status, stderr.trim());
    }
    Ok(())
}
