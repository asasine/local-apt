//! Executable external processes and commands.

mod command_error;
mod get_deb_fields;

pub use command_error::Error as CommandError;
pub use get_deb_fields::{Error as GetDebFieldsError, get_deb_fields};

use std::process::Command;
use tracing::info;

/// Update repository metadata by calling update-local-repo
pub fn update_repository_metadata() -> Result<(), CommandError> {
    info!("Updating repository metadata...");

    let status = Command::new("update-local-repo")
        .status()
        .map_err(CommandError::Spawn)?;

    if !status.success() {
        return Err(CommandError::NonZeroExitStatus(status));
    }

    Ok(())
}
