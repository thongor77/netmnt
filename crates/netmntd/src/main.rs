//! netmntd — privileged system daemon.
//!
//! Owns `org.netmnt` on the system bus and performs the actual mounting.
//! Every mutating method is guarded by polkit (see `data/polkit/`) before any
//! privileged action runs.

mod exec;
mod polkit;

use netmnt_common::{MountRequest, MountResult, BUS_NAME, OBJECT_PATH};
use zbus::interface;
use zbus::message::Header;

/// Map an internal error into a D-Bus error returned to the client.
fn to_fdo(err: anyhow::Error) -> zbus::fdo::Error {
    zbus::fdo::Error::Failed(err.to_string())
}

/// Ask polkit whether the caller of this message may perform `action`.
async fn authorize(
    conn: &zbus::Connection,
    header: &Header<'_>,
    action: &str,
) -> zbus::fdo::Result<()> {
    let sender = header
        .sender()
        .ok_or_else(|| zbus::fdo::Error::AccessDenied("missing caller identity".into()))?;

    let authorized = polkit::is_authorized(conn, sender.as_str(), action)
        .await
        .map_err(to_fdo)?;

    if !authorized {
        tracing::warn!(%action, "polkit denied authorization");
        return Err(zbus::fdo::Error::AccessDenied(format!(
            "polkit denied action {action}"
        )));
    }
    Ok(())
}

/// The manager object served on the system bus.
struct Manager;

#[interface(name = "org.netmnt.Manager1")]
impl Manager {
    /// Mount the share described by `request` and return the resulting mount point.
    async fn mount(
        &self,
        request: MountRequest,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] conn: &zbus::Connection,
    ) -> zbus::fdo::Result<MountResult> {
        tracing::info!(url = %request.url, mount_point = %request.mount_point, "mount requested");
        let action = if request.persistent {
            polkit::ACTION_MOUNT_PERSISTENT
        } else {
            polkit::ACTION_MOUNT
        };
        authorize(conn, &header, action).await?;

        let result = exec::perform_mount(&request).await.map_err(|e| {
            tracing::warn!(url = %request.url, error = %e, "mount failed");
            to_fdo(e)
        })?;
        tracing::info!(mount_point = %result.mount_point, "mounted");
        Ok(result)
    }

    /// Unmount the share currently mounted at `mount_point`.
    async fn unmount(
        &self,
        mount_point: String,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] conn: &zbus::Connection,
    ) -> zbus::fdo::Result<()> {
        tracing::info!(%mount_point, "unmount requested");
        authorize(conn, &header, polkit::ACTION_UNMOUNT).await?;

        exec::perform_unmount(&mount_point).await.map_err(|e| {
            tracing::warn!(%mount_point, error = %e, "unmount failed");
            to_fdo(e)
        })?;
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
