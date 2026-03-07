//! [`ConfiguredPackage`] represents a single package to be processed.

use crate::{
    external::{GetDebFieldsError, get_deb_fields},
    paths::PoolDir,
};
use core::fmt::Display;
use std::{
    fs::{self, File},
    io,
    path::Path,
};
use tracing::{debug, info};

/// A single package to be processed.
///
/// This object is parsed from the configuration file. It contains information to
/// download and process the package.
#[derive(Debug, serde::Deserialize)]
pub struct ConfiguredPackage {
    /// The download URL for the `.deb` package. This should point directly to a
    /// `.deb` file.
    pub url: String,
}

impl ConfiguredPackage {
    /// Download the package to `temp_dir`, verify it, and move it to the appropriate
    /// location in `pool_dir`.
    pub fn process<T: AsRef<Path>>(
        &self,
        pool_dir: &PoolDir,
        temp_dir: T,
    ) -> Result<(), ProcessPackageError> {
        info!("Processing package from: {}", self.url);
        let temp_file = temp_dir
            .as_ref()
            .join(format!("package-{}.deb", std::process::id()));

        self.download_to(&temp_file)
            .map_err(ProcessPackageError::DownloadFailed)?;

        // Extract package metadata to move to correct location in the pool
        // This also validates that the deb file is well-formed
        let [pkg_name, pkg_version, pkg_arch] =
            get_deb_fields(&temp_file, &["Package", "Version", "Architecture"])
                .map_err(|e| ProcessPackageError::InvalidDeb(InvalidDebError::Fields(e)))?;

        let standard_debian_filename = format!("{}_{}_{}.deb", pkg_name, pkg_version, pkg_arch);
        let target_dir = pool_dir
            .package_dir(&pkg_name)
            .ok_or(ProcessPackageError::InvalidDeb(InvalidDebError::NameEmpty))?;

        // Validation done, move the file
        fs::create_dir_all(&target_dir).map_err(ProcessPackageError::IoError)?;
        let target_path = target_dir.join(&standard_debian_filename);
        fs::rename(&temp_file, &target_path).map_err(ProcessPackageError::IoError)?;

        info!(
            "Successfully installed {} to {}",
            standard_debian_filename,
            target_path.display()
        );

        Ok(())
    }

    /// Download the package to the specified path.
    // TODO: timestamping to avoid redownloading if the same package already exists
    // in the pool and shares a timestamp with the source (e.g., from HTTP headers)
    fn download_to<P: AsRef<Path>>(&self, path: P) -> Result<(), DownloadError> {
        let mut response =
            reqwest::blocking::get(&self.url).map_err(DownloadError::RequestFailed)?;

        let status = response.status();
        if !status.is_success() {
            return Err(DownloadError::RequestNotSuccessful(status));
        }

        let file = File::create(path).map_err(DownloadError::IoError)?;
        let mut file = std::io::BufWriter::new(file);
        let bytes_writte = io::copy(&mut response, &mut file).map_err(DownloadError::IoError)?;
        debug!("Downloaded {} bytes from {}", bytes_writte, self.url);
        Ok(())
    }
}

/// Errors that can occur when processing a package.
///
/// See [`ConfiguredPackage::process`] for details.
#[derive(Debug)]
pub enum ProcessPackageError {
    DownloadFailed(DownloadError),
    InvalidDeb(InvalidDebError),
    IoError(io::Error),
}

impl Display for ProcessPackageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessPackageError::DownloadFailed(e) => {
                write!(f, "Failed to download package: {}", e)
            }
            ProcessPackageError::InvalidDeb(e) => write!(f, "Invalid deb file: {}", e),
            ProcessPackageError::IoError(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl core::error::Error for ProcessPackageError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            ProcessPackageError::DownloadFailed(e) => Some(e),
            ProcessPackageError::InvalidDeb(e) => Some(e),
            ProcessPackageError::IoError(e) => Some(e),
        }
    }
}

/// Errors that can occur when downloading a package.
///
/// See [`ConfiguredPackage::process`] for details.
#[derive(Debug)]
pub enum DownloadError {
    RequestFailed(reqwest::Error),
    RequestNotSuccessful(reqwest::StatusCode),
    IoError(io::Error),
}

impl Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadError::RequestFailed(e) => write!(f, "HTTP request failed: {}", e),
            DownloadError::RequestNotSuccessful(status) => {
                write!(
                    f,
                    "HTTP request returned non-success status code: {}",
                    status
                )
            }
            DownloadError::IoError(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl core::error::Error for DownloadError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            DownloadError::RequestFailed(e) => Some(e),
            DownloadError::RequestNotSuccessful(_) => None,
            DownloadError::IoError(e) => Some(e),
        }
    }
}

/// Errors that can occur when validating a deb file and extracting metadata from it.
///
/// See [`ConfiguredPackage::process`] for details.
#[derive(Debug)]
pub enum InvalidDebError {
    Fields(GetDebFieldsError),
    NameEmpty,
}

impl Display for InvalidDebError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvalidDebError::Fields(e) => write!(f, "Failed to extract fields from deb: {}", e),
            InvalidDebError::NameEmpty => write!(f, "Package name is empty"),
        }
    }
}

impl core::error::Error for InvalidDebError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            InvalidDebError::Fields(e) => Some(e),
            InvalidDebError::NameEmpty => None,
        }
    }
}
