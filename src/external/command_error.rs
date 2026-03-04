use core::fmt::Display;
use std::process::ExitStatus;

/// Errors that can occur when executing external commands.
#[derive(Debug)]
pub enum Error {
    /// An error occurred while spawning the command.
    Spawn(std::io::Error),

    /// The command returned a non-zero exit status.
    NonZeroExitStatus(ExitStatus),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Spawn(e) => write!(f, "Failed to spawn command: {}", e),
            Error::NonZeroExitStatus(status) => {
                write!(f, "Command exited with non-zero status: {}", status)
            }
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Error::Spawn(e) => Some(e),
            Error::NonZeroExitStatus(_) => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Spawn(e)
    }
}
