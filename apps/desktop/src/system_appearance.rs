use crate::model::SystemColorScheme;

#[cfg(target_os = "linux")]
pub fn detect() -> Option<SystemColorScheme> {
    let color_scheme = portal_color_scheme()
        .or_else(|| gsettings("color-scheme").and_then(|value| parse_gnome_color_scheme(&value)));

    color_scheme.or_else(|| {
        gsettings("gtk-theme")
            .or_else(|| std::env::var("GTK_THEME").ok())
            .filter(|theme| theme.to_ascii_lowercase().contains("dark"))
            .map(|_| SystemColorScheme::Dark)
    })
}

#[cfg(target_os = "linux")]
fn portal_color_scheme() -> Option<SystemColorScheme> {
    std::process::Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.freedesktop.portal.Desktop",
            "--object-path",
            "/org/freedesktop/portal/desktop",
            "--method",
            "org.freedesktop.portal.Settings.Read",
            "org.freedesktop.appearance",
            "color-scheme",
        ])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .and_then(|value| parse_portal_color_scheme(&value))
}

#[cfg(target_os = "linux")]
fn gsettings(key: &str) -> Option<String> {
    std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", key])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(not(target_os = "linux"))]
pub fn detect() -> Option<SystemColorScheme> {
    None
}

#[cfg(target_os = "linux")]
fn parse_gnome_color_scheme(value: &str) -> Option<SystemColorScheme> {
    match value.trim().trim_matches('\'') {
        "prefer-dark" => Some(SystemColorScheme::Dark),
        "default" | "prefer-light" => Some(SystemColorScheme::Light),
        _ => None,
    }
}

#[cfg(target_os = "linux")]
fn parse_portal_color_scheme(value: &str) -> Option<SystemColorScheme> {
    let value = value.strip_prefix("(<<uint32 ")?.strip_suffix(">>,)\n")?;
    match value {
        "1" => Some(SystemColorScheme::Dark),
        "2" => Some(SystemColorScheme::Light),
        _ => None,
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn parses_gnome_color_scheme_values() {
        assert_eq!(
            parse_gnome_color_scheme("'prefer-dark'\n"),
            Some(SystemColorScheme::Dark)
        );
        assert_eq!(
            parse_gnome_color_scheme("'default'\n"),
            Some(SystemColorScheme::Light)
        );
        assert_eq!(parse_gnome_color_scheme("'invalid'"), None);
    }

    #[test]
    fn parses_portal_color_scheme_values() {
        assert_eq!(
            parse_portal_color_scheme("(<<uint32 1>>,)\n"),
            Some(SystemColorScheme::Dark)
        );
        assert_eq!(
            parse_portal_color_scheme("(<<uint32 2>>,)\n"),
            Some(SystemColorScheme::Light)
        );
        assert_eq!(parse_portal_color_scheme("(<<uint32 0>>,)\n"), None);
    }
}
