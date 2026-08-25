//! [`ConfiguredPackage`] represents a single package to be processed.

use super::UrlTimestamps;
use crate::{
    external::{GetDebFieldsError, get_deb_fields},
    paths::PoolDir,
};
use std::{
    fs::{self, File},
    io,
    path::Path,
};
use thiserror::Error;
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

/// The outcome of processing a package.
///
/// See [`ConfiguredPackage::process`] for details.
pub enum ProcessResult {
    /// The package was downloaded and installed into the pool.
    Downloaded {
        source: String,
        package: String,
        version: String,
        path: std::path::PathBuf,
    },

    /// The package was already up-to-date (HTTP 304 Not Modified).
    AlreadyUpToDate { source: String },
}

impl ConfiguredPackage {
    /// Download the package to `temp_dir`, verify it, and move it to the appropriate
    /// location in `pool_dir`.
    ///
    /// Uses `url_timestamps` to send conditional HTTP requests (`If-Modified-Since`)
    /// and avoid re-downloading packages that haven't changed.
    pub fn process<T: AsRef<Path>>(
        &self,
        pool_dir: &PoolDir,
        temp_dir: T,
        url_timestamps: &mut UrlTimestamps,
    ) -> Result<ProcessResult, ProcessPackageError> {
        let (download_url, source) = self
            .resolve_download_url()
            .map_err(ProcessPackageError::DownloadFailed)?;

        info!("Processing package from {source}");
        let if_modified_since = url_timestamps.get_if_modified_since(&download_url);

        let temp_file = temp_dir
            .as_ref()
            .join(format!("package-{}.deb", std::process::id()));

        let download_result = download_to(&download_url, &temp_file, if_modified_since.as_deref())
            .map_err(ProcessPackageError::DownloadFailed)?;

        let last_modified = match download_result {
            DownloadResult::NotModified => {
                info!("Package already up-to-date: {source}");
                return Ok(ProcessResult::AlreadyUpToDate { source });
            }
            DownloadResult::Downloaded { last_modified } => last_modified,
        };

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

        url_timestamps.set(download_url, target_path.clone(), last_modified.as_deref());

        info!(
            "Successfully installed {} to {}",
            standard_debian_filename,
            target_path.display()
        );

        Ok(ProcessResult::Downloaded {
            source,
            package: pkg_name,
            version: pkg_version,
            path: target_path,
        })
    }

    /// A source description safe for persistent logs and structured output.
    pub fn source_label(&self) -> String {
        match self {
            Self::Url { url } => sanitized_url(url),
            Self::GithubRelease { repo, .. } => format!("github:{repo}"),
        }
    }

    /// Resolve the download URL for this package source.
    ///
    /// For `Url` types, this is the URL itself. For `GithubRelease` types, this
    /// queries the GitHub API for the latest release and finds a matching asset.
    fn resolve_download_url(&self) -> Result<(String, String), DownloadError> {
        match self {
            ConfiguredPackage::Url { url } => Ok((url.clone(), sanitized_url(url))),
            ConfiguredPackage::GithubRelease {
                repo,
                asset_pattern,
            } => {
                let pattern = regex::Regex::new(asset_pattern)
                    .map_err(|e| DownloadError::InvalidAssetPattern(e.to_string()))?;

                let api_url = format!("https://api.github.com/repos/{repo}/releases/latest");
                info!("Fetching latest release for github:{repo}");

                let response = http_client()
                    .get(&api_url)
                    .header("Accept", "application/vnd.github+json")
                    .send()
                    .map_err(DownloadError::request_failed)?;

                let status = response.status();
                if !status.is_success() {
                    return Err(DownloadError::RequestNotSuccessful(status));
                }

                let release: GithubRelease =
                    response.json().map_err(DownloadError::request_failed)?;

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

                let source = format!("github:{repo}/{}", asset.name);
                info!("Found matching asset: {source}");
                Ok((asset.browser_download_url.clone(), source))
            }
        }
    }
}

fn sanitized_url(value: &str) -> String {
    let Ok(mut url) = reqwest::Url::parse(value) else {
        return "invalid-url".to_owned();
    };
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_query(None);
    url.set_fragment(None);
    url.to_string()
}

/// Build an HTTP client with a User-Agent header (required by GitHub API).
fn http_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent("local-apt")
        .build()
        .expect("failed to build HTTP client")
}

/// The result of a download attempt, distinguishing between a successful download
/// and a server-indicated "not modified" response.
enum DownloadResult {
    /// The server responded with `304 Not Modified`.
    NotModified,

    /// The file was downloaded. Contains the `Last-Modified` header value if present.
    Downloaded { last_modified: Option<String> },
}

/// Download a URL to the specified path.
///
/// If `if_modified_since` is provided, it is sent as the `If-Modified-Since` header.
/// If the server responds with `304 Not Modified`, [`DownloadResult::NotModified`] is
/// returned.
fn download_to<P: AsRef<Path>>(
    url: &str,
    path: P,
    if_modified_since: Option<&str>,
) -> Result<DownloadResult, DownloadError> {
    let mut request = http_client().get(url);
    if let Some(since) = if_modified_since {
        request = request.header("If-Modified-Since", since);
    }

    let mut response = request.send().map_err(DownloadError::request_failed)?;

    let status = response.status();
    if status == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(DownloadResult::NotModified);
    }

    if !status.is_success() {
        return Err(DownloadError::RequestNotSuccessful(status));
    }

    let last_modified = response
        .headers()
        .get("Last-Modified")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let file = File::create(path).map_err(DownloadError::IoError)?;
    let mut file = std::io::BufWriter::new(file);
    let bytes_written = io::copy(&mut response, &mut file).map_err(DownloadError::IoError)?;
    debug!(
        "Downloaded {} bytes from {}",
        bytes_written,
        sanitized_url(url)
    );
    Ok(DownloadResult::Downloaded { last_modified })
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
#[derive(Debug, Error)]
pub enum ProcessPackageError {
    #[error("failed to download package: {0}")]
    DownloadFailed(DownloadError),
    #[error("invalid deb file: {0}")]
    InvalidDeb(InvalidDebError),
    #[error("package I/O failed: {0}")]
    IoError(io::Error),
}

/// Errors that can occur when downloading a package.
///
/// See [`ConfiguredPackage::process`] for details.
#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("HTTP request failed ({kind})")]
    RequestFailed {
        kind: &'static str,
        source: reqwest::Error,
    },
    #[error("HTTP request returned {0}")]
    RequestNotSuccessful(reqwest::StatusCode),
    #[error("download I/O failed: {0}")]
    IoError(io::Error),
    #[error("invalid asset_pattern regex: {0}")]
    InvalidAssetPattern(String),
    #[error("no release asset matched '{pattern}'; available assets: {available:?}")]
    NoMatchingAsset {
        pattern: String,
        available: Vec<String>,
    },
}

impl DownloadError {
    fn request_failed(source: reqwest::Error) -> Self {
        let kind = if source.is_timeout() {
            "request timed out"
        } else if source.is_connect() {
            "connection failed"
        } else if source.is_decode() {
            "response decoding failed"
        } else if source.is_body() {
            "response body failed"
        } else if source.is_builder() {
            "request could not be built"
        } else {
            "request transport failed"
        };
        Self::RequestFailed { kind, source }
    }
}

/// Errors that can occur when validating a deb file and extracting metadata from it.
///
/// See [`ConfiguredPackage::process`] for details.
#[derive(Debug, Error)]
pub enum InvalidDebError {
    #[error("failed to extract fields from deb: {0}")]
    Fields(GetDebFieldsError),
    #[error("package name is empty")]
    NameEmpty,
}

#[cfg(test)]
mod tests {
    use super::sanitized_url;

    #[test]
    fn source_labels_remove_secrets() {
        assert_eq!(
            sanitized_url("https://user:password@example.com/pkg.deb?token=secret#fragment"),
            "https://example.com/pkg.deb"
        );
    }
}
