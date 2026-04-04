//! [`PoolDir`] contains the downloaded packages.

use std::{
    io,
    path::{Path, PathBuf},
};

/// The package pool directory where downloaded packages are sorted and stored.
///
/// Packages are moved here after being downloaded and verified by [`ConfiguredPackage::process`][`local_apt::packages::ConfiguredPackage::process`]
/// in a temporary directory.
///
/// Use [`package_dir`][`Self::package_dir`] to get the appropriate subdirectory
/// for a package in this directory.
pub struct PoolDir(PathBuf);

impl PoolDir {
    /// The default component for the pool directory is `main`.
    pub const COMPONENT: &'static str = "main";

    /// Create a [`PoolDir`] from a repository root directory using the default component.
    ///
    /// The pool directory is located at `{repo_dir}/pool/main`.
    pub fn from_repo_dir(repo_dir: &Path) -> Self {
        PoolDir(repo_dir.join("pool").join(Self::COMPONENT))
    }

    /// Get the appropriate subdirectory for a package in the pool directory.
    ///
    /// The subdirectory is determined by the first letter of the package name.
    /// If the package name is empty, [`None`] is returned.
    pub fn package_dir(&self, package_name: &str) -> Option<PathBuf> {
        let first_letter = package_name.chars().next()?.to_ascii_lowercase();

        // avoid allocating a new String by encoding the char into a stack buffer
        // each char is at most 4 bytes in UTF-8
        let mut buf = [0; 4];
        let first_letter_str = first_letter.encode_utf8(&mut buf);
        Some(self.0.join(first_letter_str).join(package_name))
    }

    /// Get the root path of the pool directory.
    pub fn path(&self) -> &Path {
        &self.0
    }

    /// Iterate over all `.deb` files in the pool, grouped by package directory.
    ///
    /// Each item is a [`Vec`] of `.deb` file paths from the same package directory.
    /// Directories that contain no `.deb` files are skipped.
    pub fn deb_files_by_package(&self) -> io::Result<Vec<Vec<PathBuf>>> {
        let mut result = Vec::new();

        if !self.0.exists() {
            return Ok(result);
        }

        for letter_entry in std::fs::read_dir(&self.0)? {
            let letter_entry = letter_entry?;
            if !letter_entry.file_type().is_ok_and(|ft| ft.is_dir()) {
                continue;
            }

            for pkg_entry in std::fs::read_dir(letter_entry.path())? {
                let pkg_entry = pkg_entry?;
                if !pkg_entry.file_type().is_ok_and(|ft| ft.is_dir()) {
                    continue;
                }

                let deb_files: Vec<PathBuf> = std::fs::read_dir(pkg_entry.path())?
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|ext| ext == "deb"))
                    .collect();

                if !deb_files.is_empty() {
                    result.push(deb_files);
                }
            }
        }

        Ok(result)
    }
}
