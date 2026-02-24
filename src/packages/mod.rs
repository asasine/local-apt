//! [`ConfiguredPackages`] contains all packages to be processed, parsed from the
//! configuration file with [`from_config`][`ConfiguredPackages::from_config`].

use crate::{external::get_deb_fields, paths::PoolDir};
use anyhow::{Context, anyhow};
use core::{error::Error, fmt::Display};
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
#[derive(Debug)]
pub struct ConfiguredPackage {
    /// The download URL for the ``.deb`` package. This should point directly to
    /// a `.deb` file.
    pub url: String,
}

impl ConfiguredPackage {
    /// Download the package to `temp_dir`, verify it, and move it to the appropriate
    /// location in `pool_dir`.
    pub fn process<T: AsRef<Path>>(&self, pool_dir: &PoolDir, temp_dir: T) -> anyhow::Result<()> {
        info!("Processing package from: {}", self.url);
        let temp_file = temp_dir
            .as_ref()
            .join(format!("package-{}.deb", std::process::id()));

        self.download_to(&temp_file)?;

        // Extract package metadata to move to correct location in the pool
        // This also validates that the deb file is well-formed
        let [pkg_name, pkg_version, pkg_arch] =
            get_deb_fields(&temp_file, &["Package", "Version", "Architecture"])
                .context("Could not extract package fields")?;

        let standard_debian_filename = format!("{}_{}__{}.deb", pkg_name, pkg_version, pkg_arch);
        let target_dir = pool_dir
            .package_dir(&pkg_name)
            .ok_or_else(|| anyhow!("Package name is empty, cannot determine target directory"))?;

        // Validation done, move the file
        fs::create_dir_all(&target_dir).context("Failed to create target directory")?;
        let target_path = target_dir.join(&standard_debian_filename);
        fs::rename(&temp_file, &target_path)
            .context("Failed to move package to target directory")?;

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
    fn download_to<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let mut response =
            reqwest::blocking::get(&self.url).context("Failed to download package")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Download failed with status: {}",
                response.status()
            ));
        }

        let file = File::create(path).context("Failed to create temporary file")?;
        let mut file = std::io::BufWriter::new(file);
        let bytes_writte = io::copy(&mut response, &mut file)?;
        debug!("Downloaded {} bytes from {}", bytes_writte, self.url);
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
