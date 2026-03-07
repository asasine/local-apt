//! [`ConfiguredPackages`] contains all packages to be processed.

/// All packages to be processed, parsed from the configuration file.
#[derive(serde::Deserialize)]
pub struct ConfiguredPackages {
    #[serde(default, rename = "package")]
    pub packages: Vec<super::ConfiguredPackage>,
}
