use std::path::PathBuf;

use crate::paths::StateDir;

#[derive(clap::Args, Default, Debug)]

pub struct Args {
    /// The directory to store the downloaded packages and generated metadata. Defaults to /var/lib/local-apt/
    #[clap(long, short = 'd')]
    pub repository_directory: Option<PathBuf>,
}

impl Args {
    /// Get the state directory based on the provided repository directory or the default.
    pub fn state_dir(&self) -> StateDir {
        self.repository_directory
            .as_ref()
            .map(StateDir::new)
            .unwrap_or_default()
    }
}
