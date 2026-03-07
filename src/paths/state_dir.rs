//! [`StateDir`] is the root directory for all variable state managed by `local-apt`.

use super::PoolDir;
use std::path::{Path, PathBuf};

/// The root directory for all variable state managed by `local-apt`.
///
/// By default this is `/var/lib/local-apt/`. It contains:
/// - The APT repository structure (`pool/`, `dists/`, `cache/`)
/// - The URL timestamps mapping file (`url-timestamps.json`)
pub struct StateDir(PathBuf);

impl StateDir {
    const DEFAULT: &str = "/var/lib/local-apt";
    const URL_TIMESTAMPS_FILE: &str = "url-timestamps.json";

    /// Create a new [`StateDir`] from the given path.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        StateDir(path.into())
    }

    /// Get the [`PoolDir`] for the default component within this state directory.
    pub fn pool_dir(&self) -> PoolDir {
        PoolDir::from_repo_dir(&self.0)
    }

    /// Get the path to the URL timestamps mapping file.
    pub fn url_timestamps_path(&self) -> PathBuf {
        self.0.join(Self::URL_TIMESTAMPS_FILE)
    }

    /// Get the root path of the state directory.
    ///
    /// This is the root of the APT repository, containing the `pool` directory.
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Default for StateDir {
    fn default() -> Self {
        StateDir(PathBuf::from(Self::DEFAULT))
    }
}
