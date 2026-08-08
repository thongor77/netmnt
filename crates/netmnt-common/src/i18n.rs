//! GNU gettext initialization and message formatting shared by netmnt binaries.

use std::ffi::OsString;

use gettextrs::LocaleCategory;
use gettextrs::{bind_textdomain_codeset, bindtextdomain, gettext, setlocale, textdomain};

const DOMAIN: &str = "netmnt";
const DEFAULT_LOCALE_DIR: &str = "/usr/share/locale";

/// Select the process locale and bind netmnt's gettext catalog.
///
/// `NETMNT_LOCALEDIR` is useful when testing an uninstalled catalog. Installed
/// builds use the standard system location by default.
pub fn init() {
    setlocale(LocaleCategory::LcAll, "");
    let locale_dir =
        std::env::var_os("NETMNT_LOCALEDIR").unwrap_or_else(|| OsString::from(DEFAULT_LOCALE_DIR));
    let _ = bindtextdomain(DOMAIN, locale_dir);
    let _ = bind_textdomain_codeset(DOMAIN, "UTF-8");
    let _ = textdomain(DOMAIN);
}

/// Translate a message, falling back to its English source text.
pub fn tr(message: &str) -> String {
    gettext(message)
}

/// Translate a message and substitute named `{placeholders}` exactly once.
///
/// Values are inserted as opaque text, so braces inside a URL, username, or
/// path cannot be interpreted as another placeholder.
pub fn tr_args(message: &str, values: &[(&str, &str)]) -> String {
    let template = tr(message);
    interpolate(&template, values)
}

fn interpolate(template: &str, values: &[(&str, &str)]) -> String {
    let mut output = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(open) = rest.find('{') {
        output.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('}') else {
            output.push_str(&rest[open..]);
            return output;
        };
        let name = &after_open[..close];
        if let Some((_, value)) = values.iter().find(|(key, _)| *key == name) {
            output.push_str(value);
        } else {
            output.push('{');
            output.push_str(name);
            output.push('}');
        }
        rest = &after_open[close + 1..];
    }
    output.push_str(rest);
    output
}

#[cfg(test)]
mod tests {
    use super::interpolate;

    #[test]
    fn substitutes_named_values_once() {
        assert_eq!(
            interpolate(
                "Mount {url} at {path}",
                &[("url", "smb://host/{path}"), ("path", "/mnt/share")],
            ),
            "Mount smb://host/{path} at /mnt/share"
        );
    }

    #[test]
    fn preserves_unknown_or_unclosed_placeholders() {
        assert_eq!(
            interpolate("{known} {unknown} {open", &[("known", "yes")]),
            "yes {unknown} {open"
        );
    }
}
