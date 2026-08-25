use crate::paths::StateDir;
use clap::{ArgAction, Parser, ValueEnum};
use std::path::PathBuf;

/// Common arguments shared by subcommands that operate on the repository.
#[derive(clap::Args, Default, Debug)]
pub struct RepoArgs {
    /// The directory to store the downloaded packages and generated metadata. Defaults to /var/lib/local-apt/
    #[clap(long, short = 'd')]
    pub repository_directory: Option<PathBuf>,

    /// Write a stable machine-readable report to stdout.
    #[clap(long, value_enum)]
    pub output: Option<OutputFormat>,

    /// Show informational progress on stderr; repeat for debug details.
    #[clap(long, short = 'v', action = ArgAction::Count, conflicts_with = "quiet")]
    pub verbose: u8,

    /// Only write fatal errors to stderr.
    #[clap(long, short = 'q', conflicts_with = "verbose")]
    pub quiet: bool,

    /// Return a failure status when any package fails.
    #[clap(long)]
    pub fail_on_partial: bool,
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

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum OutputFormat {
    Json,
    Ndjson,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandName {
    Update,
    Cleanup,
}

#[derive(Parser, Debug)]
pub enum Cli {
    /// Update packages from configured URLs.
    Update(RepoArgs),

    /// Remove old package versions from the pool, keeping only the latest version of each package.
    Cleanup(RepoArgs),
}

impl Cli {
    pub fn parts(&self) -> (&RepoArgs, CommandName) {
        match self {
            Self::Update(args) => (args, CommandName::Update),
            Self::Cleanup(args) => (args, CommandName::Cleanup),
        }
    }
}

impl Default for Cli {
    fn default() -> Self {
        Cli::Update(Default::default())
    }
}
