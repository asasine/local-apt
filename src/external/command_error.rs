use std::process::{Command, ExitStatus, Output};

use thiserror::Error;

/// Errors that can occur when executing external commands.
#[derive(Debug, Error)]
pub enum Error {
    /// An error occurred while spawning the command.
    #[error("failed to run {program}: {source}")]
    Spawn {
        program: String,
        source: std::io::Error,
    },

    /// The command returned a non-zero exit status.
    #[error("{program} exited with {status}: {stderr}")]
    NonZeroExitStatus {
        program: String,
        status: ExitStatus,
        stderr: String,
    },
}

pub(crate) fn run(command: &mut Command) -> Result<Output, Error> {
    const MAX_STDERR_BYTES: usize = 16 * 1024;

    let program = command.get_program().to_string_lossy().into_owned();
    let output = command.output().map_err(|source| Error::Spawn {
        program: program.clone(),
        source,
    })?;
    if output.status.success() {
        return Ok(output);
    }

    let truncated = output.stderr.len() > MAX_STDERR_BYTES;
    let stderr = &output.stderr[..output.stderr.len().min(MAX_STDERR_BYTES)];
    let mut stderr = String::from_utf8_lossy(stderr).trim().to_owned();
    if stderr.is_empty() {
        stderr = "no diagnostic output".to_owned();
    } else if truncated {
        stderr.push_str(" [truncated]");
    }

    Err(Error::NonZeroExitStatus {
        program,
        status: output.status,
        stderr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonzero_exit_preserves_stderr() {
        let mut command = Command::new("sh");
        command.args(["-c", "printf 'actionable diagnostic' >&2; exit 7"]);
        let error = run(&mut command).unwrap_err();
        assert!(error.to_string().contains("actionable diagnostic"));
        assert!(error.to_string().contains("exit status: 7"));
    }
}
