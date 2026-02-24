//! [`ConfiguredPackages`] contains all packages to be processed, parsed from the
//! configuration file with [`from_config`][`ConfiguredPackages::from_config`].

use crate::{external::get_deb_fields, paths::PoolDir};
use anyhow::{Context, anyhow};
use core::{error::Error, fmt::Display};
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
    process::Command,
};
use tracing::info;

/// A single package to be processed.
///
/// This object is parsed from the configuration file. It contains information to
/// download and process the package.
#[derive(Debug)]
pub struct ConfiguredPackage {
    /// The download URL for the ``.deb`` package. This should point directly to
    /// a `.deb` file.
    pub url: String,
}

impl ConfiguredPackage {
    /// Download the package to `temp_dir`, verify it, and move it to the appropriate
    /// location in `pool_dir`.
    pub fn process(&self, pool_dir: &PoolDir, temp_dir: &Path) -> anyhow::Result<()> {
        info!("Processing package from: {}", self.url);

        // Download to temp directory
        let download_file = temp_dir.join(format!("package-{}.deb", std::process::id()));

        let response = reqwest::blocking::get(&self.url).context("Failed to download package")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Download failed with status: {}",
                response.status()
            ));
        }

        let mut file = File::create(&download_file).context("Failed to create temporary file")?;
        let content = response
            .bytes()
            .context("Failed to read response content")?;

        file.write_all(&content)
            .context("Failed to write downloaded content")?;

        drop(file);

        // Verify it's a valid .deb file
        let verify_output = Command::new("dpkg-deb")
            .arg("-I")
            .arg(&download_file)
            .output()
            .context("Failed to run dpkg-deb")?;

        if !verify_output.status.success() {
            fs::remove_file(&download_file).ok();
            return Err(anyhow!("Downloaded file is not a valid .deb package"));
        }

        // Extract package metadata
        let [pkg_name, pkg_version, pkg_arch] =
            get_deb_fields(&download_file, &["Package", "Version", "Architecture"])
                .context("Could not extract package fields")?;

        // Construct standard Debian package filename
        let std_filename = format!("{}_{}__{}.deb", pkg_name, pkg_version, pkg_arch);

        // Auto-generate target path following Debian pool convention
        // pool/main/<first-letter>/<package-name>/
        let target_dir = pool_dir
            .package_dir(&pkg_name)
            .ok_or_else(|| anyhow!("Package name is empty, cannot determine target directory"))?;

        // Create target directory if it doesn't exist
        fs::create_dir_all(&target_dir).context("Failed to create target directory")?;

        // Move the .deb file to target directory with standard naming
        let target_path = target_dir.join(&std_filename);
        fs::rename(&download_file, &target_path)
            .context("Failed to move package to target directory")?;

        info!(
            "Successfully installed {} to {}",
            pkg_name,
            target_path.display()
        );

        Ok(())
    }
}

/// All packages to be processed, parsed from the configuration file.
pub struct ConfiguredPackages {
    pub packages: Vec<ConfiguredPackage>,
}

#[derive(Debug)]
pub enum PackagesFromConfigError {
    /// The configuration file could not be found or opened.
    ConfigFileNotFound(std::io::Error),

    /// An error occurred while reading the configuration file.
    ReadError(std::io::Error),
}

impl Display for PackagesFromConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackagesFromConfigError::ConfigFileNotFound(e) => {
                write!(f, "Configuration file not found: {}", e)
            }
            PackagesFromConfigError::ReadError(e) => {
                write!(f, "Failed to read configuration: {}", e)
            }
        }
    }
}

impl Error for PackagesFromConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            PackagesFromConfigError::ConfigFileNotFound(e) => Some(e),
            PackagesFromConfigError::ReadError(e) => Some(e),
        }
    }
}
