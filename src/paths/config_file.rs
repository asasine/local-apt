//! [`ConfigFile`] lists package sources to download from.

use core::fmt::Display;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

use crate::packages::{ConfiguredPackage, ConfiguredPackages, PackagesFromConfigError};

/// The configuration file which lists package sources to download from.
#[derive(Debug)]
pub struct ConfigFile(PathBuf);

impl ConfigFile {
    /// Either the `LOCAL_APT_CONFIG` environment variable or `/etc/local-apt/packages.txt`.
    ///
    /// `LOCAL_APT_CONFIG` path must be absolute, otherwise it will be ignored and
    /// the default will be used.
    pub fn env_or_default() -> Self {
        let path = std::env::var_os("LOCAL_APT_CONFIG")
            .and_then(is_absolute_path)
            .unwrap_or_else(|| PathBuf::from("/etc/local-apt/packages.txt"));

        Self(path)
    }

    /// Check if the configuration file exists.
    pub fn exists(&self) -> bool {
        self.0.exists()
    }

    /// Parse the configuration file and return the packages to process.
    ///
    /// The configuration file should contain one URL per line.
    /// Lines starting with a `#` character are treated as comments and ignored.
    pub fn read_packages(&self) -> Result<ConfiguredPackages, PackagesFromConfigError> {
        let file =
            File::open(self.0.as_path()).map_err(PackagesFromConfigError::ConfigFileNotFound)?;

        let reader = BufReader::new(file);

        let mut packages = Vec::new();

        for line in reader.lines() {
            let line = line.map_err(PackagesFromConfigError::ReadError)?;
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            packages.push(ConfiguredPackage {
                url: trimmed.to_string(),
            });
        }

        Ok(ConfiguredPackages { packages })
    }
}

impl Display for ConfigFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

/// The path if it's absolute or [`None`]. Empty paths are not absolute.
fn is_absolute_path(path: impl Into<PathBuf>) -> Option<PathBuf> {
    let path = path.into();
    if path.is_absolute() { Some(path) } else { None }
}
