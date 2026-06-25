//! Mount execution: turns a validated [`MountRequest`] into an actual mount.
//! Requires CAP_SYS_ADMIN (the daemon runs as root).
//!
//! - Session mounts call `mount.cifs` directly; the password is passed through
//!   the `PASSWD` environment variable so it never appears on the command line.
//! - Persistent mounts generate a systemd `.mount` unit (and a root-only
//!   credentials file when authenticated) so they survive a reboot.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Stdio;

use netmnt_common::{smb, MountRequest, MountResult};
use tokio::process::Command;

/// Directory holding root-only credentials files for persistent mounts.
const CRED_DIR: &str = "/etc/netmnt";
/// Where generated systemd `.mount` units are written.
const UNIT_DIR: &str = "/etc/systemd/system";

/// Mount the share for the current session via `mount.cifs`.
pub async fn perform_mount(request: &MountRequest) -> anyhow::Result<MountResult> {
    let target = smb::parse_smb_url(&request.url)?;
    let source = smb::unc_path(&target);
    let mount_point = mount_point_of(request)?;

    // Idempotent: if something is already mounted here, treat it as success
    // instead of letting mount.cifs fail with a cryptic EBUSY.
    if is_mountpoint(mount_point).await {
        tracing::info!(mount_point = %mount_point.display(), "already mounted");
        return Ok(mounted(mount_point, false));
    }

    tokio::fs::create_dir_all(mount_point).await.map_err(|e| {
        anyhow::anyhow!("cannot create mount point {}: {e}", mount_point.display())
    })?;

    let mut options = vec![
        "rw".to_string(),
        format!("uid={}", request.uid),
        format!("gid={}", request.gid),
    ];
    if request.username.is_empty() {
        options.push("guest".to_string());
    } else {
        options.push(format!("username={}", request.username));
    }

    let mut cmd = Command::new("mount.cifs");
    cmd.arg(&source)
        .arg(mount_point)
        .arg("-o")
        .arg(options.join(","))
        // Never let mount.cifs block on an interactive password prompt: the
        // daemon has no terminal, so a missing password must fail, not hang.
        .stdin(Stdio::null());

    // Always set PASSWD when a username is given (even if empty) to suppress prompting.
    if !request.username.is_empty() {
        cmd.env("PASSWD", &request.password);
    }

    let output = cmd.output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("mount.cifs failed ({}): {}", output.status, stderr.trim());
    }

    Ok(mounted(mount_point, false))
}

/// Mount the share and register a systemd `.mount` unit so it survives reboot.
pub async fn perform_persistent_mount(request: &MountRequest) -> anyhow::Result<MountResult> {
    let target = smb::parse_smb_url(&request.url)?;
    let source = smb::unc_path(&target);
    let mount_point = mount_point_of(request)?;

    tokio::fs::create_dir_all(mount_point).await?;

    let unit_name = systemd_escape_mount(mount_point).await?;
    let base = unit_name.trim_end_matches(".mount");

    // Credentials go in a root-only file referenced by the unit (never in the
    // world-readable unit itself).
    let mut options = vec![
        "rw".to_string(),
        "_netdev".to_string(),
        format!("uid={}", request.uid),
        format!("gid={}", request.gid),
    ];
    if request.username.is_empty() {
        options.push("guest".to_string());
    } else {
        let cred_path = format!("{CRED_DIR}/{base}.cred");
        write_credentials(&cred_path, &request.username, &request.password).await?;
        options.push(format!("credentials={cred_path}"));
    }

    let unit = mount_unit(&source, &mount_point.to_string_lossy(), &options.join(","));
    tokio::fs::write(format!("{UNIT_DIR}/{unit_name}"), unit).await?;

    run("systemctl", &["daemon-reload"]).await?;
    run("systemctl", &["enable", &unit_name]).await?;
    if !is_mountpoint(mount_point).await {
        run("systemctl", &["start", &unit_name]).await?;
    }

    tracing::info!(%unit_name, "persistent mount enabled");
    Ok(mounted(mount_point, true))
}

/// Unmount `mount_point`. If it is backed by a netmnt persistent unit, tear the
/// unit (and its credentials) down so it does not come back on reboot.
pub async fn perform_unmount(mount_point: &str) -> anyhow::Result<()> {
    let path = Path::new(mount_point);

    if let Ok(unit_name) = systemd_escape_mount(path).await {
        let unit_path = format!("{UNIT_DIR}/{unit_name}");
        if Path::new(&unit_path).exists() {
            run("systemctl", &["disable", "--now", &unit_name]).await?;
            let base = unit_name.trim_end_matches(".mount");
            let _ = tokio::fs::remove_file(&unit_path).await;
            let _ = tokio::fs::remove_file(format!("{CRED_DIR}/{base}.cred")).await;
            run("systemctl", &["daemon-reload"]).await.ok();
            remove_empty_mount_point(path).await;
            tracing::info!(%unit_name, "persistent mount removed");
            return Ok(());
        }
    }

    if !is_mountpoint(path).await {
        anyhow::bail!("{mount_point} is not mounted");
    }
    let output = Command::new("umount").arg(mount_point).output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("umount failed ({}): {}", output.status, stderr.trim());
    }
    remove_empty_mount_point(path).await;
    Ok(())
}

/// Remove the mount-point directory netmnt created at mount time, now that the
/// share is unmounted. Uses a non-recursive `remove_dir`, so it only succeeds on
/// an empty directory — any leftover content (or a path that is still a mount)
/// is left untouched. Failures are non-fatal: the unmount itself already worked.
async fn remove_empty_mount_point(path: &Path) {
    if let Err(e) = tokio::fs::remove_dir(path).await {
        tracing::debug!(
            mount_point = %path.display(),
            error = %e,
            "left mount-point directory in place"
        );
    } else {
        tracing::info!(mount_point = %path.display(), "removed empty mount-point directory");
    }
}

fn mount_point_of(request: &MountRequest) -> anyhow::Result<&Path> {
    if request.mount_point.is_empty() {
        anyhow::bail!("mount_point must be provided by the client");
    }
    Ok(Path::new(&request.mount_point))
}

fn mounted(mount_point: &Path, persisted: bool) -> MountResult {
    MountResult {
        mount_point: mount_point.to_string_lossy().into_owned(),
        persisted,
    }
}

/// Return true if `path` is currently a mount point.
async fn is_mountpoint(path: &Path) -> bool {
    Command::new("mountpoint")
        .arg("-q")
        .arg(path)
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Compute the systemd unit name (`home-user-mnt-isos.mount`) for a mount point.
async fn systemd_escape_mount(path: &Path) -> anyhow::Result<String> {
    let out = Command::new("systemd-escape")
        .arg("--path")
        .arg("--suffix=mount")
        .arg(path)
        .output()
        .await?;
    if !out.status.success() {
        anyhow::bail!("systemd-escape failed");
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Write a root-only (0600) CIFS credentials file.
async fn write_credentials(path: &str, username: &str, password: &str) -> anyhow::Result<()> {
    tokio::fs::create_dir_all(CRED_DIR).await?;
    tokio::fs::write(path, format!("username={username}\npassword={password}\n")).await?;
    let mut perms = tokio::fs::metadata(path).await?.permissions();
    perms.set_mode(0o600);
    tokio::fs::set_permissions(path, perms).await?;
    Ok(())
}

/// Run a command and fail with its stderr if it returns non-zero.
async fn run(program: &str, args: &[&str]) -> anyhow::Result<()> {
    let out = Command::new(program).args(args).output().await?;
    if !out.status.success() {
        anyhow::bail!(
            "{program} {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

/// Render a systemd `.mount` unit.
fn mount_unit(what: &str, where_: &str, options: &str) -> String {
    format!(
        "[Unit]\n\
         Description=netmnt persistent mount of {what}\n\
         After=network-online.target\n\
         Wants=network-online.target\n\
         \n\
         [Mount]\n\
         What={what}\n\
         Where={where_}\n\
         Type=cifs\n\
         Options={options}\n\
         \n\
         [Install]\n\
         WantedBy=multi-user.target\n"
    )
}

#[cfg(test)]
mod tests {
    use super::mount_unit;

    #[test]
    fn renders_mount_unit() {
        let unit = mount_unit("//lab1.local/isos", "/home/u/mnt/isos", "rw,_netdev,guest");
        assert!(unit.contains("What=//lab1.local/isos"));
        assert!(unit.contains("Where=/home/u/mnt/isos"));
        assert!(unit.contains("Type=cifs"));
        assert!(unit.contains("Options=rw,_netdev,guest"));
        assert!(unit.contains("[Install]"));
    }
}
