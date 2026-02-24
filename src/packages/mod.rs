//! [`ConfiguredPackages`] contains all packages to be processed, parsed from the
//! configuration file with [`from_config`][`ConfiguredPackages::from_config`].

mod configured_package;
pub use configured_package::{
    ConfiguredPackage, DownloadError, InvalidDebError, ProcessPackageError,
};

use core::{error::Error, fmt::Display};

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
