use clap::Parser;

pub mod cleanup;
pub mod update;

#[derive(Parser, Debug)]
pub enum Cli {
    /// Update packages from configured URLs.
    Update(update::Args),

    /// Remove old package versions from the pool, keeping only the latest version of each package.
    Cleanup(cleanup::Args),
}

impl Default for Cli {
    fn default() -> Self {
        Cli::Update(Default::default())
    }
}
