use std::env;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::Command;

use crate::error::{AppError, AppResult};

/// Environment handed to the user-facing terminal so the upstream CLI,
/// including `dsh plugin`, resolves to the selected runtime and DSH Home.
pub struct TerminalEnvironment {
    pub path: Option<OsString>,
    pub dsh_home: Option<String>,
    pub working_directory: Option<PathBuf>,
}

/// Opens a system terminal the user owns; the child is not supervised and
/// must survive Host exit.
pub fn open(environment: &TerminalEnvironment) -> AppResult<()> {
    let mut command = terminal_command(environment)?;
    apply_environment(&mut command, environment);
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| AppError::new("terminal.error.launchFailed").technical(error.to_string()))
}

fn apply_environment(command: &mut Command, environment: &TerminalEnvironment) {
    if let Some(path) = &environment.path {
        command.env("PATH", path);
    }
    if let Some(dsh_home) = &environment.dsh_home {
        command.env("DSH_HOME", dsh_home);
    }
    if let Some(directory) = &environment.working_directory {
        command.current_dir(directory);
    }
}

#[cfg(windows)]
fn terminal_command(_: &TerminalEnvironment) -> AppResult<Command> {
    use std::os::windows::process::CommandExt;

    const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
    let mut command = Command::new("cmd.exe");
    command.arg("/k").creation_flags(CREATE_NEW_CONSOLE);
    Ok(command)
}

#[cfg(target_os = "macos")]
fn terminal_command(environment: &TerminalEnvironment) -> AppResult<Command> {
    let mut script = String::new();
    if let Some(directory) = &environment.working_directory {
        script.push_str(&format!(
            "cd {}; ",
            shell_quote(&directory.to_string_lossy())
        ));
    }
    if let Some(path) = &environment.path {
        script.push_str(&format!(
            "export PATH={}; ",
            shell_quote(&path.to_string_lossy())
        ));
    }
    if let Some(dsh_home) = &environment.dsh_home {
        script.push_str(&format!("export DSH_HOME={}; ", shell_quote(dsh_home)));
    }
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_owned());
    script.push_str(&format!("exec {}", shell_quote(&shell)));
    let apple_script = format!(
        "tell application \"Terminal\" to do script \"{}\"",
        apple_script_escape(&script)
    );
    let mut command = Command::new("/usr/bin/osascript");
    command.arg("-e").arg(apple_script);
    Ok(command)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn terminal_command(_: &TerminalEnvironment) -> AppResult<Command> {
    let (program, style_arguments) =
        find_terminal().ok_or_else(|| AppError::new("terminal.error.unavailable"))?;
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_owned());
    let mut command = Command::new(program);
    command.args(style_arguments).arg(&shell).arg("-il");
    Ok(command)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn find_terminal() -> Option<(PathBuf, &'static [&'static str])> {
    const CANDIDATES: &[(&str, &[&str])] = &[
        ("x-terminal-emulator", &["-e"]),
        ("gnome-terminal", &["--"]),
        ("konsole", &["-e"]),
        ("xfce4-terminal", &["-x"]),
        ("alacritty", &["-e"]),
        ("kitty", &["--"]),
        ("xterm", &["-e"]),
    ];
    let path = env::var_os("PATH")?;
    for (program, arguments) in CANDIDATES {
        if let Some(found) = find_on_path(program, &path) {
            return Some((found, arguments));
        }
    }
    None
}

#[cfg(all(unix, not(target_os = "macos")))]
fn find_on_path(program: &str, path: &OsStr) -> Option<PathBuf> {
    env::split_paths(path)
        .map(|directory| directory.join(program))
        .find(|candidate| candidate.is_file())
}

#[cfg(target_os = "macos")]
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(target_os = "macos")]
fn apple_script_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "macos")]
    use super::{apple_script_escape, shell_quote};

    #[cfg(target_os = "macos")]
    #[test]
    fn single_quotes_shell_values() {
        assert_eq!(shell_quote("/tmp/dsh home"), "'/tmp/dsh home'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn escapes_apple_script_strings() {
        assert_eq!(
            apple_script_escape("export PATH='/a'; exec \"/bin/zsh\""),
            "export PATH='/a'; exec \\\"/bin/zsh\\\""
        );
    }
}
