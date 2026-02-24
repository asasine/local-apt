//! [`ConfiguredPackages`] contains all packages to be processed, parsed from the
//! configuration file with [`from_config`][`ConfiguredPackages::from_config`].

mod configured_package;
mod configured_packages;
pub use configured_package::{
    ConfiguredPackage, DownloadError, InvalidDebError, ProcessPackageError,
};
pub use configured_packages::ConfiguredPackages;
