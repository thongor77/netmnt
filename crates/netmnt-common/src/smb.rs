//! Pure SMB-URL parsing and mount-point helpers.
//!
//! Kept free of I/O so it can be unit-tested without root or a real share, and
//! shared between the client (which proposes a mount point) and the daemon
//! (which performs the mount).

use std::path::{Path, PathBuf};

/// Errors that can occur while interpreting an `smb://` URL.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SmbError {
    /// The URL did not start with `smb://`.
    #[error("unsupported scheme: expected smb://")]
    UnsupportedScheme,
    /// No host component was found.
    #[error("missing host in SMB URL")]
    MissingHost,
    /// No share component was found.
    #[error("missing share in SMB URL")]
    MissingShare,
}

/// A parsed SMB location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmbTarget {
    /// Host (and optional `:port`), e.g. `lab1.local`.
    pub host: String,
    /// Share name, percent-decoded, e.g. `isos`.
    pub share: String,
    /// Path under the share, percent-decoded, possibly empty.
    pub subpath: String,
}

/// Decode `%XX` percent-escapes; leaves everything else untouched.
///
/// Unlike query-string decoding, `+` is **not** treated as a space (it is a
/// valid path character).
pub fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Parse an `smb://host/share[/subpath]` URL.
pub fn parse_smb_url(url: &str) -> Result<SmbTarget, SmbError> {
    let rest = url
        .strip_prefix("smb://")
        .ok_or(SmbError::UnsupportedScheme)?
        .trim_end_matches('/');

    let (host, after) = match rest.split_once('/') {
        Some((host, after)) => (host, after),
        None => (rest, ""),
    };
    if host.is_empty() {
        return Err(SmbError::MissingHost);
    }
    if after.is_empty() {
        return Err(SmbError::MissingShare);
    }

    let (share_raw, subpath_raw) = match after.split_once('/') {
        Some((share, sub)) => (share, sub),
        None => (after, ""),
    };
    let share = percent_decode(share_raw);
    if share.is_empty() {
        return Err(SmbError::MissingShare);
    }

    Ok(SmbTarget {
        host: host.to_string(),
        share,
        subpath: percent_decode(subpath_raw),
    })
}

/// Build the UNC source string `//host/share` understood by `mount.cifs`.
pub fn unc_path(target: &SmbTarget) -> String {
    format!("//{}/{}", target.host, target.share)
}

/// Default mount point for a share under `base` (e.g. `~/mnt` + `isos`).
///
/// Path separators in the share name are replaced so a share can never escape
/// `base`.
pub fn default_mount_point(base: &Path, share: &str) -> PathBuf {
    let safe = share.replace(['/', '\\'], "_");
    base.join(safe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_url() {
        let t = parse_smb_url("smb://lab1.local/isos").unwrap();
        assert_eq!(t.host, "lab1.local");
        assert_eq!(t.share, "isos");
        assert_eq!(t.subpath, "");
    }

    #[test]
    fn parses_url_with_encoded_subpath_and_trailing_slash() {
        let t = parse_smb_url("smb://lab1.local/isos/Linux%20Distributions/").unwrap();
        assert_eq!(t.host, "lab1.local");
        assert_eq!(t.share, "isos");
        assert_eq!(t.subpath, "Linux Distributions");
    }

    #[test]
    fn keeps_port_in_host() {
        let t = parse_smb_url("smb://lab1.local:445/share").unwrap();
        assert_eq!(t.host, "lab1.local:445");
        assert_eq!(t.share, "share");
    }

    #[test]
    fn rejects_wrong_scheme_and_missing_parts() {
        assert_eq!(parse_smb_url("nfs://h/s"), Err(SmbError::UnsupportedScheme));
        assert_eq!(parse_smb_url("smb:///share"), Err(SmbError::MissingHost));
        assert_eq!(parse_smb_url("smb://host"), Err(SmbError::MissingShare));
        assert_eq!(parse_smb_url("smb://host/"), Err(SmbError::MissingShare));
    }

    #[test]
    fn builds_unc_path() {
        let t = parse_smb_url("smb://lab1.local/isos").unwrap();
        assert_eq!(unc_path(&t), "//lab1.local/isos");
    }

    #[test]
    fn mount_point_is_confined_to_base() {
        let mp = default_mount_point(Path::new("/home/u/mnt"), "isos");
        assert_eq!(mp, PathBuf::from("/home/u/mnt/isos"));
        // A malicious share name cannot escape the base directory.
        let mp = default_mount_point(Path::new("/home/u/mnt"), "../../etc");
        assert_eq!(mp, PathBuf::from("/home/u/mnt/.._.._etc"));
    }

    #[test]
    fn percent_decode_handles_trailing_percent() {
        assert_eq!(percent_decode("abc%"), "abc%");
        assert_eq!(percent_decode("a%2"), "a%2");
        assert_eq!(percent_decode("a%2Fb"), "a/b");
    }
}
