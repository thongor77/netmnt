//! polkit authorization gate.
//!
//! Each mutating D-Bus method asks polkit whether the *calling* client is
//! allowed to perform the corresponding action (declared in
//! `data/polkit/org.netmnt.policy`). We use the `system-bus-name` subject kind:
//! polkit resolves the caller's uid/pid itself from the bus name, which avoids
//! the TOCTOU races of looking those up separately.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use zbus::proxy;
use zbus::zvariant::{Type, Value};

/// Action ids — must match `data/polkit/org.netmnt.policy`.
pub const ACTION_MOUNT: &str = "org.netmnt.mount";
pub const ACTION_MOUNT_PERSISTENT: &str = "org.netmnt.mount-persistent";
pub const ACTION_UNMOUNT: &str = "org.netmnt.unmount";

/// polkit flag: allow polkit to interactively prompt the user for authentication.
const ALLOW_USER_INTERACTION: u32 = 1;

/// A polkit `Subject` (`(sa{sv})`): the `system-bus-name` of the caller.
#[derive(Serialize, Type)]
struct Subject<'a> {
    kind: &'a str,
    details: HashMap<&'a str, Value<'a>>,
}

/// polkit `CheckAuthorization` result (`(bba{ss})`).
#[derive(Debug, Deserialize, Type)]
struct AuthResult {
    is_authorized: bool,
    #[allow(dead_code)]
    is_challenge: bool,
    #[allow(dead_code)]
    details: HashMap<String, String>,
}

#[proxy(
    interface = "org.freedesktop.PolicyKit1.Authority",
    default_service = "org.freedesktop.PolicyKit1",
    default_path = "/org/freedesktop/PolicyKit1/Authority"
)]
trait Authority {
    fn check_authorization(
        &self,
        subject: Subject<'_>,
        action_id: &str,
        details: HashMap<&str, &str>,
        flags: u32,
        cancellation_id: &str,
    ) -> zbus::Result<AuthResult>;
}

/// Return `true` if `sender` is authorized for `action_id`.
///
/// `sender` is the caller's unique bus name (e.g. `:1.42`), taken from the
/// incoming message header.
pub async fn is_authorized(
    conn: &zbus::Connection,
    sender: &str,
    action_id: &str,
) -> anyhow::Result<bool> {
    let mut details = HashMap::new();
    details.insert("name", Value::from(sender));
    let subject = Subject {
        kind: "system-bus-name",
        details,
    };

    let authority = AuthorityProxy::new(conn).await?;
    let result = authority
        .check_authorization(
            subject,
            action_id,
            HashMap::new(),
            ALLOW_USER_INTERACTION,
            "",
        )
        .await?;

    Ok(result.is_authorized)
}
