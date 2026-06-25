//! Credential acquisition for "mount as", run in the user's session.
//!
//! Priority: stored KWallet entry → interactive prompt (kdialog when a GUI is
//! available, otherwise a terminal prompt). Passwords are never placed on a
//! command line; they travel to the daemon over D-Bus only.

use std::io::Write;
use std::process::{Command, Stdio};

/// KWallet folder under which netmnt stores its entries.
const WALLET_FOLDER: &str = "netmnt";

/// Credentials gathered for a mount.
pub struct Credentials {
    pub username: String,
    pub password: String,
    /// True when freshly entered by the user (candidate for saving).
    pub freshly_entered: bool,
    /// User asked to store them in KWallet.
    pub remember: bool,
}

/// Obtain credentials for `url`.
///
/// `wallet_key` is the stable per-share key (the UNC path). `username_hint`,
/// when non-empty, forces that username and skips the wallet lookup.
pub fn acquire(url: &str, wallet_key: &str, username_hint: &str) -> anyhow::Result<Credentials> {
    if username_hint.is_empty() {
        if let Some((username, password)) = kwallet_read(wallet_key) {
            return Ok(Credentials {
                username,
                password,
                freshly_entered: false,
                remember: false,
            });
        }
    }

    let username = if username_hint.is_empty() {
        prompt_username(url)?
    } else {
        username_hint.to_string()
    };
    let password = prompt_password(url, &username)?;
    let remember = ask_remember();

    Ok(Credentials {
        username,
        password,
        freshly_entered: true,
        remember,
    })
}

fn gui_available() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some() || std::env::var_os("DISPLAY").is_some()
}

fn wallet_name() -> String {
    std::env::var("NETMNT_WALLET").unwrap_or_else(|_| "kdewallet".to_string())
}

fn prompt_username(url: &str) -> anyhow::Result<String> {
    if gui_available() {
        if let Some(out) = kdialog(&[
            "--title",
            "netmnt",
            "--inputbox",
            &format!("Nom d'utilisateur pour {url}"),
        ]) {
            return Ok(out);
        }
    }
    eprint!("Nom d'utilisateur pour {url} : ");
    std::io::stderr().flush().ok();
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

fn prompt_password(url: &str, user: &str) -> anyhow::Result<String> {
    if gui_available() {
        if let Some(out) = kdialog(&[
            "--title",
            "netmnt",
            "--password",
            &format!("Mot de passe pour {user}@{url}"),
        ]) {
            return Ok(out);
        }
    }
    Ok(rpassword::prompt_password(format!(
        "Mot de passe pour {user}@{url} : "
    ))?)
}

fn ask_remember() -> bool {
    if gui_available() {
        return Command::new("kdialog")
            .args(["--title", "netmnt", "--yesno", "Mémoriser dans KWallet ?"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    }
    false
}

/// Run kdialog with `args`; return trimmed stdout on success, `None` otherwise
/// (cancelled, or kdialog not installed).
fn kdialog(args: &[&str]) -> Option<String> {
    let out = Command::new("kdialog").args(args).output().ok()?;
    if out.status.success() {
        Some(
            String::from_utf8_lossy(&out.stdout)
                .trim_end_matches('\n')
                .to_string(),
        )
    } else {
        None
    }
}

/// Read a `username\npassword` entry from KWallet. Best-effort: any failure
/// (no wallet, locked, tool missing, malformed) yields `None`.
fn kwallet_read(key: &str) -> Option<(String, String)> {
    let out = Command::new("kwallet-query")
        .args(["-f", WALLET_FOLDER, "-r", key, &wallet_name()])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_wallet_value(&String::from_utf8_lossy(&out.stdout))
}

/// Store a `username\npassword` entry in KWallet. Best-effort.
pub fn kwallet_write(key: &str, username: &str, password: &str) -> anyhow::Result<()> {
    let mut child = Command::new("kwallet-query")
        .args(["-f", WALLET_FOLDER, "-w", key, &wallet_name()])
        .stdin(Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        write!(stdin, "{username}\n{password}")?;
    }
    if !child.wait()?.success() {
        anyhow::bail!("kwallet-query write failed");
    }
    Ok(())
}

/// Split a stored wallet value into `(username, password)`.
fn parse_wallet_value(raw: &str) -> Option<(String, String)> {
    let value = raw.trim_end_matches('\n');
    if value.is_empty() || value.contains("Failed to read entry") {
        return None;
    }
    let (username, password) = value.split_once('\n')?;
    Some((username.to_string(), password.to_string()))
}

#[cfg(test)]
mod tests {
    use super::parse_wallet_value;

    #[test]
    fn parses_username_password() {
        assert_eq!(
            parse_wallet_value("alice\ns3cret\n"),
            Some(("alice".to_string(), "s3cret".to_string()))
        );
    }

    #[test]
    fn password_may_contain_special_chars() {
        assert_eq!(
            parse_wallet_value("bob\np@ss/w:rd"),
            Some(("bob".to_string(), "p@ss/w:rd".to_string()))
        );
    }

    #[test]
    fn rejects_empty_or_failed_or_single_line() {
        assert_eq!(parse_wallet_value(""), None);
        assert_eq!(parse_wallet_value("\n"), None);
        assert_eq!(parse_wallet_value("Failed to read entry"), None);
        assert_eq!(parse_wallet_value("no-newline"), None);
    }
}
