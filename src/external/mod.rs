//! Executable external processes and commands.

mod command_error;
mod get_deb_fields;

pub use command_error::Error as CommandError;
pub use get_deb_fields::{Error as GetDebFieldsError, get_deb_fields};
use thiserror::Error;

use std::{io, path::Path, process::Command};

/// Compare two Debian package version strings using `dpkg --compare-versions`.
///
/// Returns `true` if `a` is greater than `b`.
pub fn dpkg_version_is_greater(a: &str, b: &str) -> Result<bool, CommandError> {
    let mut command = Command::new("dpkg");
    command.args(["--compare-versions", a, "gt", b]);
    match command_error::run(&mut command) {
        Ok(_) => Ok(true),
        Err(CommandError::NonZeroExitStatus { status, .. }) if status.code() == Some(1) => {
            Ok(false)
        }
        Err(error) => Err(error),
    }
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

    let mut generate = Command::new("apt-ftparchive");
    generate
        .args([
            "-c",
            "/usr/share/local-apt/conf/apt.conf",
            "generate",
            "/usr/share/local-apt/conf/tree.conf",
        ])
        .current_dir(repo_dir);
    command_error::run(&mut generate).map_err(Error::AptFtparchiveFailed)?;

    let mut release = Command::new("apt-ftparchive");
    release
        .args([
            "-c",
            "/usr/share/local-apt/conf/apt.conf",
            "release",
            "dists/stable",
        ])
        .current_dir(repo_dir);
    let output = command_error::run(&mut release).map_err(Error::AptFtparchiveFailed)?;

    std::fs::write(repo_dir.join("dists/stable/Release"), output.stdout)
        .map_err(Error::CouldNotCreateFile)?;

    Ok(())
}

#[derive(Debug, Error)]
pub enum Error {
    /// Failed to create a file during the update process.
    #[error("failed to create repository metadata: {0}")]
    CouldNotCreateFile(io::Error),

    /// Failed to execute the `apt-ftparchive` command to update the repository metadata.
    #[error("failed to update repository metadata: {0}")]
    AptFtparchiveFailed(CommandError),
}
