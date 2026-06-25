//! netmntd — privileged system daemon.
//!
//! Owns `org.netmnt` on the system bus and performs the actual mounting.
//! Every method is meant to be guarded by polkit (see `data/polkit/`); the
//! authorization wiring is a TODO tracked in docs/Roadmap.md.

mod exec;

use netmnt_common::{MountRequest, MountResult, BUS_NAME, OBJECT_PATH};
use zbus::interface;

/// Map an internal error into a D-Bus error returned to the client.
fn to_fdo(err: anyhow::Error) -> zbus::fdo::Error {
    zbus::fdo::Error::Failed(err.to_string())
}

/// The manager object served on the system bus.
struct Manager;

#[interface(name = "org.netmnt.Manager1")]
impl Manager {
    /// Mount the share described by `request` and return the resulting mount point.
    ///
    /// TODO (Phase 2): guard with polkit using the caller's uid/pid.
    async fn mount(&self, request: MountRequest) -> zbus::fdo::Result<MountResult> {
        tracing::info!(url = %request.url, mount_point = %request.mount_point, "mount requested");
        let result = exec::perform_mount(&request).await.map_err(to_fdo)?;
        tracing::info!(mount_point = %result.mount_point, "mounted");
        Ok(result)
    }

    /// Unmount the share currently mounted at `mount_point`.
    async fn unmount(&self, mount_point: String) -> zbus::fdo::Result<()> {
        tracing::info!(%mount_point, "unmount requested");
        exec::perform_unmount(&mount_point).await.map_err(to_fdo)?;
        tracing::info!(%mount_point, "unmounted");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    tracing::info!("starting netmntd, claiming {BUS_NAME} on the system bus");

    let _conn = zbus::connection::Builder::system()?
        .name(BUS_NAME)?
        .serve_at(OBJECT_PATH, Manager)?
        .build()
        .await?;

    tracing::info!("netmntd ready");

    // Park forever; the connection runs in the background.
    std::future::pending::<()>().await;
    Ok(())
}
