//! Executable external processes and commands.

mod command_error;
mod get_deb_fields;

pub use command_error::Error as CommandError;
pub use get_deb_fields::{Error as GetDebFieldsError, get_deb_fields};

use std::{io, path::Path, process::Command};

/// Compare two Debian package version strings using `dpkg --compare-versions`.
///
/// Returns `true` if `a` is greater than `b`.
pub fn dpkg_version_is_greater(a: &str, b: &str) -> Result<bool, CommandError> {
    let status = Command::new("dpkg")
        .args(["--compare-versions", a, "gt", b])
        .status()
        .map_err(CommandError::Spawn)?;

    Ok(status.success())
}
use tracing::info;

/// Update repository metadata using `apt-ftparchive` with the given repository directory.
///
/// The argument should be the root of the repository, which contains the `pool` directory.
/// For the default pool directory (`/var/lib/local-apt/pool/main`), this would be `/var/lib/local-apt`.
pub fn update_repository_metadata(repo_dir: impl AsRef<Path>) -> Result<(), Error> {
    info!("Updating repository metadata...");

    let repo_dir = repo_dir.as_ref();

    std::fs::create_dir_all(
        repo_dir
            .join("dists")
            .join("stable")
            .join("main")
            .join("binary-amd64"),
    )
    .map_err(Error::CouldNotCreateFile)?;

    std::fs::create_dir_all(repo_dir.join("cache")).map_err(Error::CouldNotCreateFile)?;

    let status = Command::new("apt-ftparchive")
        .args([
            "-c",
            "/usr/share/local-apt/conf/apt.conf",
            "generate",
            "/usr/share/local-apt/conf/tree.conf",
        ])
        .current_dir(repo_dir)
        .status()
        .map_err(|e| Error::AptFtparchiveFailed(e.into()))?;

    if !status.success() {
        return Err(Error::AptFtparchiveFailed(CommandError::NonZeroExitStatus(
            status,
        )));
    }

    Ok(())
}

#[derive(Debug)]
pub enum Error {
    /// Failed to create a file during the update process.
    CouldNotCreateFile(io::Error),

    /// Failed to execute the `apt-ftparchive` command to update the repository metadata.
    AptFtparchiveFailed(CommandError),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CouldNotCreateFile(e) => write!(f, "Failed to create file: {}", e),
            Self::AptFtparchiveFailed(e) => {
                write!(f, "Failed to update repository metadata: {}", e)
            }
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::CouldNotCreateFile(e) => Some(e),
            Self::AptFtparchiveFailed(e) => Some(e),
        }
    }
}
