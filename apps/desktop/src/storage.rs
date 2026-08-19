use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

pub const INSTALLATION_MARKER: &str = "dsh-desktop.installed";

pub struct AppDirectories {
    pub config: PathBuf,
    pub data: PathBuf,
}

pub fn resolve() -> io::Result<AppDirectories> {
    let executable = std::env::current_exe()?;
    match directory_mode(&executable, is_development_build())? {
        DirectoryMode::System => Ok(system_directories()),
        DirectoryMode::Portable(data) => {
            ensure_writable_directory(&data)?;
            Ok(AppDirectories {
                config: data.clone(),
                data,
            })
        }
    }
}

fn is_development_build() -> bool {
    option_env!("DSH_DESKTOP_DEVELOPMENT") == Some("true")
}

fn system_directories() -> AppDirectories {
    AppDirectories {
        config: dirs::config_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("dsh-desktop"),
        data: dirs::data_local_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("dsh-desktop"),
    }
}

enum DirectoryMode {
    System,
    Portable(PathBuf),
}

fn directory_mode(executable: &Path, development: bool) -> io::Result<DirectoryMode> {
    if development || has_installation_marker(executable) {
        return Ok(DirectoryMode::System);
    }
    let parent = executable.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "the DSH Desktop executable has no parent directory",
        )
    })?;
    Ok(DirectoryMode::Portable(parent.join("data")))
}

fn has_installation_marker(executable: &Path) -> bool {
    fs::metadata(executable.with_file_name(INSTALLATION_MARKER))
        .is_ok_and(|metadata| metadata.is_file() && metadata.len() > 0)
}

fn ensure_writable_directory(directory: &Path) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    let probe = directory.join(format!(".dsh-desktop-write-test-{}", std::process::id()));
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&probe)?;
    fs::remove_file(probe)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_directory(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("dsh-desktop-storage-{name}-{}", std::process::id()))
    }

    #[test]
    fn development_build_uses_system_directories() {
        let executable = test_directory("development").join("dsh-desktop");
        assert!(matches!(
            directory_mode(&executable, true).expect("resolve directory mode"),
            DirectoryMode::System
        ));
    }

    #[test]
    fn installed_executable_uses_system_directories() {
        let directory = test_directory("installed");
        let executable = directory.join("dsh-desktop");
        fs::create_dir_all(&directory).expect("create executable directory");
        fs::write(directory.join(INSTALLATION_MARKER), "installed\n").expect("write marker");

        assert!(matches!(
            directory_mode(&executable, false).expect("resolve directory mode"),
            DirectoryMode::System
        ));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn unmarked_executable_uses_sibling_data_directory() {
        let directory = test_directory("portable");
        let executable = directory.join("dsh-desktop");
        assert!(matches!(
            directory_mode(&executable, false).expect("resolve directory mode"),
            DirectoryMode::Portable(data) if data == directory.join("data")
        ));
    }

    #[test]
    fn empty_installation_marker_does_not_disable_portable_mode() {
        let directory = test_directory("empty-marker");
        let executable = directory.join("dsh-desktop");
        fs::create_dir_all(&directory).expect("create executable directory");
        fs::write(directory.join(INSTALLATION_MARKER), "").expect("write marker");

        assert!(matches!(
            directory_mode(&executable, false).expect("resolve directory mode"),
            DirectoryMode::Portable(data) if data == directory.join("data")
        ));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn portable_data_directory_failure_is_reported() {
        let directory = test_directory("unwritable");
        fs::create_dir_all(&directory).expect("create test directory");
        let data = directory.join("data");
        fs::write(&data, "not a directory").expect("block data directory");

        assert!(ensure_writable_directory(&data).is_err());
        let _ = fs::remove_dir_all(directory);
    }
}
