use thiserror::Error;

use crate::external::{CommandError, command_error};
use std::{io::BufRead, path::Path, process::Command};

#[derive(Debug, Error)]
pub enum Error {
    /// An error occurred while executing `dpkg-deb` command.
    #[error("dpkg-deb failed: {0}")]
    DpkgDebFailed(CommandError),

    /// Reading the output of `dpkg-deb` failed, which could indicate an issue with
    /// the command's output or an I/O error.
    #[error("failed to read dpkg-deb output: {0}")]
    CannotReadDpkgDebOutput(std::io::Error),

    /// The specified field was not found in the control file of the package.
    #[error("field not found in control file: {0}")]
    FieldNotFound(String),

    /// The specified field was found but the `dpkg-deb` output was empty for that
    /// field.
    #[error("field is empty in control file: {0}")]
    FieldEmpty(String),

    /// The `dpkg-deb` output did not contain the expected number of lines corresponding
    /// to the requested fields.
    #[error("unexpected dpkg-deb output: expected {expected} lines, got {actual}")]
    UnexpectedOutput { expected: usize, actual: usize },
}

/// Extract multiple fields from the control file of a binary package.
///
/// The return is an array of the same length and order as the input `fields`.
///
/// # Examples
/// The returned array can be combined with an slice pattern to assign to multiple
/// variables.
///
/// ```rust,no_run
/// # use local_apt::external::get_deb_fields;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let [name, version, arch] = get_deb_fields("package.deb", &["Package", "Version", "Architecture"])?;
/// # Ok(())
/// # }
/// ```
pub fn get_deb_fields<P: AsRef<Path>, const N: usize>(
    deb_file: P,
    fields: &[&str; N],
) -> Result<[String; N], Error> {
    let mut command = Command::new("dpkg-deb");
    command.arg("-f").arg(deb_file.as_ref()).args(fields);
    let output = command_error::run(&mut command).map_err(Error::DpkgDebFailed)?;

    /// Extract the value from the output of `dpkg-deb -f`.
    ///
    /// With multiple fields, the output uses `Field: value` format.
    /// With a single field, it outputs just the raw value.
    fn extract_value(line: &str, single_field: bool) -> Option<&str> {
        if single_field {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        } else {
            line.split_once(':').map(|(_, v)| v.trim())
        }
    }

    let single_field = N == 1;
    let values = fields
        .iter()
        .zip(output.stdout.lines())
        .map(|(field, line)| {
            let line = line.map_err(Error::CannotReadDpkgDebOutput)?;
            let value = extract_value(line.as_str(), single_field)
                .ok_or_else(|| Error::FieldNotFound(field.to_string()))?;

            if value.is_empty() {
                return Err(Error::FieldEmpty(field.to_string()));
            }

            Ok(value.to_string())
        })
        .collect::<Result<Vec<String>, Error>>()?;

    let values = values
        .try_into()
        .map_err(|values: Vec<String>| Error::UnexpectedOutput {
            expected: N,
            actual: values.len(),
        })?;

    Ok(values)
}
