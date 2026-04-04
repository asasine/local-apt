use crate::paths::StateDir;
use clap::Parser;
use std::path::PathBuf;

/// Common arguments shared by subcommands that operate on the repository.
#[derive(clap::Args, Default, Debug)]
pub struct RepoArgs {
    /// The directory to store the downloaded packages and generated metadata. Defaults to /var/lib/local-apt/
    #[clap(long, short = 'd')]
    pub repository_directory: Option<PathBuf>,
}

impl RepoArgs {
    /// Get the state directory based on the provided repository directory or the default.
    pub fn state_dir(&self) -> StateDir {
        self.repository_directory
            .as_ref()
            .map(StateDir::new)
            .unwrap_or_default()
    }
}

#[derive(Parser, Debug)]
pub enum Cli {
    /// Update packages from configured URLs.
    Update(RepoArgs),

    /// Remove old package versions from the pool, keeping only the latest version of each package.
    Cleanup(RepoArgs),
}

impl Default for Cli {
    fn default() -> Self {
        Cli::Update(Default::default())
    }
}
