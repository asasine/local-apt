//! [`ConfigFile`] lists package sources to download from.

use std::{fmt::Display, fs, path::PathBuf};

use thiserror::Error;

use crate::packages::ConfiguredPackages;

/// The configuration file which lists package sources to download from.
#[derive(Debug)]
pub struct ConfigFile(PathBuf);

impl ConfigFile {
    /// Either the `LOCAL_APT_CONFIG` environment variable or `/etc/local-apt/packages.toml`.
    ///
    /// `LOCAL_APT_CONFIG` path must be absolute, otherwise it will be ignored and
    /// the default will be used.
    pub fn env_or_default() -> Self {
        let path = std::env::var_os("LOCAL_APT_CONFIG")
            .and_then(is_absolute_path)
            .unwrap_or_else(|| PathBuf::from("/etc/local-apt/packages.toml"));

        Self(path)
    }

    /// Check if the configuration file exists.
    pub fn exists(&self) -> bool {
        self.0.exists()
    }

    /// Parse the TOML configuration file and return the packages to process.
    pub fn read_packages(&self) -> Result<ConfiguredPackages, ReadPackagesError> {
        let content =
            fs::read_to_string(self.0.as_path()).map_err(ReadPackagesError::ConfigFileNotFound)?;

        let packages: ConfiguredPackages =
            toml::from_str(&content).map_err(|source: toml::de::Error| {
                ReadPackagesError::ParseError {
                    message: source.message().to_owned(),
                    source,
                }
            })?;

        Ok(packages)
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

/// Errors that can occur when reading the configuration file.
///
/// See [`ConfigFile::read_packages`] for details.
#[derive(Debug, Error)]
pub enum ReadPackagesError {
    /// The configuration file could not be found or opened.
    #[error("configuration file could not be read: {0}")]
    ConfigFileNotFound(std::io::Error),

    /// The configuration file could not be parsed as valid TOML.
    #[error("failed to parse configuration: {message}")]
    ParseError {
        message: String,
        source: toml::de::Error,
    },
}
