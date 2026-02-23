use clap::Parser;

pub mod update;

#[derive(Parser, Debug)]
pub enum Cli {
    /// Update packages from configured URLs.
    Update(update::Args),
}

impl Default for Cli {
    fn default() -> Self {
        Cli::Update(Default::default())
    }
}
