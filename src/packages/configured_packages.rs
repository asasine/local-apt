//! [`ConfiguredPackages`] contains all packages to be processed.

/// All packages to be processed, parsed from the configuration file.
pub struct ConfiguredPackages {
    pub packages: Vec<super::ConfiguredPackage>,
}
