//! Pure NFS-URL parsing and mount-point helpers.
//!
//! Mirrors `smb.rs`: kept free of I/O so it can be unit-tested without root or
//! a real export. Unlike CIFS, NFS access is granted by the server's export
//! ACL (host/network based) and ownership follows the server's UID mapping —
//! there is no username/password or `uid=`/`gid=` mount-option equivalent.

use std::path::{Path, PathBuf};

use crate::smb::percent_decode;

/// Errors that can occur while interpreting an `nfs://` URL.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NfsError {
    /// The URL did not start with `nfs://`.
    #[error("unsupported scheme: expected nfs://")]
    UnsupportedScheme,
    /// No host component was found.
    #[error("missing host in NFS URL")]
    MissingHost,
    /// No export path was found.
    #[error("missing export path in NFS URL")]
    MissingExport,
}

/// A parsed NFS location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NfsTarget {
    /// Host (and optional `:port`), e.g. `192.168.1.64`.
    pub host: String,
    /// Export path on the server, percent-decoded, e.g. `/volume1/testing`.
    pub export: String,
}

/// Parse an `nfs://host/export/path` URL.
pub fn parse_nfs_url(url: &str) -> Result<NfsTarget, NfsError> {
    let rest = url
        .strip_prefix("nfs://")
        .ok_or(NfsError::UnsupportedScheme)?
        .trim_end_matches('/');

    let (host, after) = match rest.split_once('/') {
        Some((host, after)) => (host, after),
        None => (rest, ""),
    };
    if host.is_empty() {
        return Err(NfsError::MissingHost);
    }
    if after.is_empty() {
        return Err(NfsError::MissingExport);
    }

    Ok(NfsTarget {
        host: host.to_string(),
        export: format!("/{}", percent_decode(after)),
    })
}

/// Build the `host:/export` source string understood by `mount.nfs`.
pub fn nfs_source(target: &NfsTarget) -> String {
    format!("{}:{}", target.host, target.export)
}

/// Default mount point for an export under `base` (e.g. `~/mnt` + `testing`
/// from `/volume1/testing`).
///
/// Only the last path segment is used, so a malicious export path can never
/// escape `base`.
pub fn default_mount_point(base: &Path, export: &str) -> PathBuf {
    let leaf = export.rsplit('/').find(|s| !s.is_empty()).unwrap_or(export);
    base.join(leaf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_url() {
        let t = parse_nfs_url("nfs://192.168.1.64/volume1/testing").unwrap();
        assert_eq!(t.host, "192.168.1.64");
        assert_eq!(t.export, "/volume1/testing");
    }

    #[test]
    fn parses_url_with_encoded_subpath_and_trailing_slash() {
        let t = parse_nfs_url("nfs://nas.local/volume1/My%20Data/").unwrap();
        assert_eq!(t.host, "nas.local");
        assert_eq!(t.export, "/volume1/My Data");
    }

    #[test]
    fn keeps_port_in_host() {
        let t = parse_nfs_url("nfs://nas.local:2049/export").unwrap();
        assert_eq!(t.host, "nas.local:2049");
        assert_eq!(t.export, "/export");
    }

    #[test]
    fn rejects_wrong_scheme_and_missing_parts() {
        assert_eq!(parse_nfs_url("smb://h/s"), Err(NfsError::UnsupportedScheme));
        assert_eq!(parse_nfs_url("nfs:///export"), Err(NfsError::MissingHost));
        assert_eq!(parse_nfs_url("nfs://host"), Err(NfsError::MissingExport));
        assert_eq!(parse_nfs_url("nfs://host/"), Err(NfsError::MissingExport));
    }

    #[test]
    fn builds_source() {
        let t = parse_nfs_url("nfs://192.168.1.64/volume1/testing").unwrap();
        assert_eq!(nfs_source(&t), "192.168.1.64:/volume1/testing");
    }

    #[test]
    fn mount_point_uses_last_segment_and_is_confined_to_base() {
        let mp = default_mount_point(Path::new("/home/u/mnt"), "/volume1/testing");
        assert_eq!(mp, PathBuf::from("/home/u/mnt/testing"));
        let mp = default_mount_point(Path::new("/home/u/mnt"), "/../../etc");
        assert_eq!(mp, PathBuf::from("/home/u/mnt/etc"));
    }
}
