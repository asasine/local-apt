//! [`ConfiguredPackages`] contains all packages to be processed, parsed from the
//! configuration file with [`from_config`][`ConfiguredPackages::from_config`].

mod configured_package;
mod configured_packages;
mod url_timestamps;
pub use configured_package::{
    ConfiguredPackage, DownloadError, InvalidDebError, ProcessPackageError, ProcessResult,
};
pub use configured_packages::ConfiguredPackages;
pub use url_timestamps::UrlTimestamps;
