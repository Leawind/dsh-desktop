#[cfg(target_os = "linux")]
use std::ffi::{OsStr, OsString};
#[cfg(target_os = "linux")]
use std::process::Command;
#[cfg(target_os = "linux")]
use std::sync::OnceLock;

#[cfg(target_os = "windows")]
pub const WINDOWS_BROWSER_ARGS: &str =
    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --no-proxy-server";

#[cfg(target_os = "linux")]
const PROXY_ENVIRONMENT_VARIABLES: [&str; 8] = [
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "ALL_PROXY",
    "NO_PROXY",
    "http_proxy",
    "https_proxy",
    "all_proxy",
    "no_proxy",
];

#[cfg(target_os = "linux")]
static ORIGINAL_PROXY_ENVIRONMENT: OnceLock<Vec<(&'static str, Option<OsString>)>> =
    OnceLock::new();

pub fn configure_process() {
    #[cfg(target_os = "linux")]
    {
        let original = ORIGINAL_PROXY_ENVIRONMENT.get_or_init(|| {
            PROXY_ENVIRONMENT_VARIABLES
                .iter()
                .map(|name| (*name, std::env::var_os(name)))
                .collect()
        });

        // WebKitGTK asks GIO for its proxy resolver before creating the first webview.
        // Select the environment-backed resolver, then remove proxy variables only from
        // the desktop process. Managed DSH children receive the original values below.
        // SAFETY: this runs at application startup, before Tauri creates worker threads.
        unsafe {
            std::env::set_var("GIO_USE_PROXY_RESOLVER", "environment");
            for (name, _) in original {
                std::env::remove_var(name);
            }
        }
    }
}

#[cfg(target_os = "linux")]
pub fn restore_child_proxy_environment(command: &mut Command) {
    command.env_remove("GIO_USE_PROXY_RESOLVER");
    if let Some(original) = ORIGINAL_PROXY_ENVIRONMENT.get() {
        for (name, value) in original {
            restore(command, name, value.as_deref());
        }
    }
}

#[cfg(target_os = "linux")]
fn restore(command: &mut Command, name: &str, value: Option<&OsStr>) {
    if let Some(value) = value {
        command.env(name, value);
    } else {
        command.env_remove(name);
    }
}
