use std::path::PathBuf;

use crate::paths::PoolDir;

#[derive(clap::Args, Default, Debug)]

pub struct Args {
    /// The directory to store the downloaded packages and generated metadata. Defaults to /var/lib/local-apt/
    #[clap(long, short = 'd')]
    pub repository_directory: Option<PathBuf>,
}

impl Args {
    /// Get the pool directory based on the provided repository directory or default to [`PoolDir::default`].
    pub fn pool_dir(&self) -> PoolDir {
        self.repository_directory
            .as_ref()
            .map(|dir| PoolDir::from_apt_ftparchive_structure(dir, PoolDir::COMPONENT))
            .unwrap_or_default()
    }
}
