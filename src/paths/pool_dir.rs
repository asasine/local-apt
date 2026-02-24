//! [`PoolDir`] contains the downloaded packages.

use std::path::{Path, PathBuf};

/// The package pool directory where downloaded packages are sorted and stored.
///
/// Packages are moved here after being downloaded and verified by [`ConfiguredPackage::process`][`local_apt::packages::ConfiguredPackage::process`]
/// in a temporary directory.
///
/// Use [`package_dir`][`Self::package_dir`] to get the appropriate subdirectory
/// for a package in this directory.
pub struct PoolDir(PathBuf);

impl PoolDir {
    /// Create a new [`PoolDir`] with the given path.
    ///
    /// The path should point to the component's pool directory (e.g., `/var/lib/local-apt/pool/main`),
    /// not a package-specific subdirectory.
    ///
    /// - To use the standard apt-ftparchive structure, see [`from_apt_ftparchive_structure`][`Self::from_apt_ftparchive_structure`].
    /// - To use the default pool directory for this [`crate`], use [`PoolDir::default`].
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        PoolDir(path.as_ref().to_path_buf())
    }

    /// Create a [`PoolDir`] from the standard apt-ftparchive structure.
    ///
    /// This assumes the pool directory is located at `{repo_dir}/pool/{component}`.
    /// `component` is typically `main`, `contrib`, or `non-free`.
    ///
    /// - To use a completely custom path, use [`PoolDir::new`].
    /// - To use the default pool directory for this [`crate`], use [`PoolDir::default`].
    pub fn from_apt_ftparchive_structure<R: AsRef<Path>, C: AsRef<Path>>(
        repo_dir: R,
        component: C,
    ) -> Self {
        PoolDir(repo_dir.as_ref().join("pool").join(component))
    }

    /// Get the appropriate subdirectory for a package in the pool directory.
    ///
    /// The subdirectory is determined by the first letter of the package name.
    /// If the package name is empty, [`None`] is returned.
    pub fn package_dir(&self, package_name: &str) -> Option<PathBuf> {
        let first_letter = package_name.chars().next()?.to_ascii_lowercase();

        // avoid allocating a new String by encoding the char into a stack buffer
        // each char is up to 4 bytes in UTF-8
        let mut buf = [0; 4];
        let first_letter_str = first_letter.encode_utf8(&mut buf);
        Some(self.0.join(first_letter_str))
    }
}

impl Default for PoolDir {
    /// The default pool directory follows the apt-ftparchive structure at `/var/lib/local-apt/pool/main`.
    ///
    /// - To use a different component (e.g., `contrib` or `non-free`), use [`from_apt_ftparchive_structure`][`Self::from_apt_ftparchive_structure`] instead.
    /// - To use a completely custom path, use [`PoolDir::new`].
    fn default() -> Self {
        PoolDir::from_apt_ftparchive_structure("/var/lib/local-apt", "main")
    }
}
