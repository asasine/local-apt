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
/// download and process the package. The `type` field determines the source type.
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type")]
pub enum ConfiguredPackage {
    /// A direct URL to a `.deb` package.
    #[serde(rename = "url")]
    Url {
        /// The download URL for the `.deb` package. This should point directly to a
        /// `.deb` file.
        url: String,
    },

    /// A `.deb` package attached to the latest GitHub Release.
    #[serde(rename = "github-release")]
    GithubRelease {
        /// The GitHub repository in `owner/repo` format.
        repo: String,

        /// A regex pattern matched against release asset filenames to select the
        /// `.deb` file to download.
        asset_pattern: String,
    },
}

impl ConfiguredPackage {
    /// Download the package to `temp_dir`, verify it, and move it to the appropriate
    /// location in `pool_dir`.
    pub fn process<T: AsRef<Path>>(
        &self,
        pool_dir: &PoolDir,
        temp_dir: T,
    ) -> Result<(), ProcessPackageError> {
        let download_url = self
            .resolve_download_url()
            .map_err(ProcessPackageError::DownloadFailed)?;

        info!("Processing package from: {}", download_url);
        let temp_file = temp_dir
            .as_ref()
            .join(format!("package-{}.deb", std::process::id()));

        download_to(&download_url, &temp_file).map_err(ProcessPackageError::DownloadFailed)?;

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

    /// Resolve the download URL for this package source.
    ///
    /// For `Url` types, this is the URL itself. For `GithubRelease` types, this
    /// queries the GitHub API for the latest release and finds a matching asset.
    fn resolve_download_url(&self) -> Result<String, DownloadError> {
        match self {
            ConfiguredPackage::Url { url } => Ok(url.clone()),
            ConfiguredPackage::GithubRelease {
                repo,
                asset_pattern,
            } => {
                let pattern = regex::Regex::new(asset_pattern)
                    .map_err(|e| DownloadError::InvalidAssetPattern(e.to_string()))?;

                let api_url = format!("https://api.github.com/repos/{repo}/releases/latest");
                info!("Fetching latest release from: {}", api_url);

                let response = http_client()
                    .get(&api_url)
                    .header("Accept", "application/vnd.github+json")
                    .send()
                    .map_err(DownloadError::RequestFailed)?;

                let status = response.status();
                if !status.is_success() {
                    return Err(DownloadError::RequestNotSuccessful(status));
                }

                let release: GithubRelease =
                    response.json().map_err(DownloadError::RequestFailed)?;

                let asset = release
                    .assets
                    .iter()
                    .find(|a| pattern.is_match(&a.name))
                    .ok_or_else(|| {
                        let available: Vec<String> =
                            release.assets.iter().map(|a| a.name.clone()).collect();
                        DownloadError::NoMatchingAsset {
                            pattern: asset_pattern.clone(),
                            available,
                        }
                    })?;

                info!(
                    "Found matching asset: {} ({})",
                    asset.name, asset.browser_download_url
                );
                Ok(asset.browser_download_url.clone())
            }
        }
    }
}

/// Build an HTTP client with a User-Agent header (required by GitHub API).
fn http_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent("local-apt")
        .build()
        .expect("failed to build HTTP client")
}

/// Download a URL to the specified path.
// TODO: timestamping to avoid redownloading if the same package already exists
// in the pool and shares a timestamp with the source (e.g., from HTTP headers)
fn download_to<P: AsRef<Path>>(url: &str, path: P) -> Result<(), DownloadError> {
    let mut response = http_client()
        .get(url)
        .send()
        .map_err(DownloadError::RequestFailed)?;

    let status = response.status();
    if !status.is_success() {
        return Err(DownloadError::RequestNotSuccessful(status));
    }

    let file = File::create(path).map_err(DownloadError::IoError)?;
    let mut file = std::io::BufWriter::new(file);
    let bytes_written = io::copy(&mut response, &mut file).map_err(DownloadError::IoError)?;
    debug!("Downloaded {} bytes from {}", bytes_written, url);
    Ok(())
}

#[derive(serde::Deserialize)]
struct GithubRelease {
    assets: Vec<GithubAsset>,
}

#[derive(serde::Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
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
    InvalidAssetPattern(String),
    NoMatchingAsset {
        pattern: String,
        available: Vec<String>,
    },
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
            DownloadError::InvalidAssetPattern(e) => {
                write!(f, "Invalid asset_pattern regex: {}", e)
            }
            DownloadError::NoMatchingAsset { pattern, available } => {
                write!(
                    f,
                    "No release asset matched pattern '{}'. Available assets: {:?}",
                    pattern, available
                )
            }
        }
    }
}

impl core::error::Error for DownloadError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            DownloadError::RequestFailed(e) => Some(e),
            DownloadError::RequestNotSuccessful(_) => None,
            DownloadError::IoError(e) => Some(e),
            DownloadError::InvalidAssetPattern(_) => None,
            DownloadError::NoMatchingAsset { .. } => None,
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
