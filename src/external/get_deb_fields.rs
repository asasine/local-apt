use crate::external::CommandError;
use core::fmt::Display;
use std::{io::BufRead, path::Path, process::Command};

#[derive(Debug)]
pub enum Error {
    /// An error occurred while executing `dpkg-deb` command.
    DpkgDebFailed(CommandError),

    /// Reading the output of `dpkg-deb` failed, which could indicate an issue with
    /// the command's output or an I/O error.
    CannotReadDpkgDebOutput(std::io::Error),

    /// The specified field was not found in the control file of the package.
    FieldNotFound(String),

    /// The specified field was found but the `dpkg-deb` output was empty for that
    /// field.
    FieldEmpty(String),

    /// The `dpkg-deb` output did not contain the expected number of lines corresponding
    /// to the requested fields.
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
/// # use local_apt::get_deb_fields;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let [name, version, arch] = get_deb_fields("package.deb", &["Package", "Version", "Architecture"])?;
/// # Ok(())
/// # }
/// ```
pub fn get_deb_fields<P: AsRef<Path>, const N: usize>(
    deb_file: P,
    fields: &[&str; N],
) -> Result<[String; N], Error> {
    let output = Command::new("dpkg-deb")
        .arg("-f")
        .arg(deb_file.as_ref())
        .args(fields)
        .output()
        .map_err(|e| Error::DpkgDebFailed(CommandError::Spawn(e)))?;

    if !output.status.success() {
        return Err(Error::DpkgDebFailed(CommandError::NonZeroExitStatus(
            output.status,
        )));
    }

    /// Extract the value from a line of the form `Field: value`
    fn extract_value(line: &str) -> Option<&str> {
        line.split_once(':').map(|(_, v)| v).map(|v| v.trim())
    }

    let values = fields
        .iter()
        .zip(output.stdout.lines())
        .map(|(field, line)| {
            let line = line.map_err(Error::CannotReadDpkgDebOutput)?;
            let value = extract_value(line.as_str())
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

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DpkgDebFailed(e) => write!(f, "dpkg-deb failed: {e}"),
            Error::CannotReadDpkgDebOutput(e) => {
                write!(f, "Failed to read dpkg-deb output: {e}")
            }
            Error::FieldNotFound(field) => write!(f, "Field not found in control file: {field}"),
            Error::FieldEmpty(field) => write!(f, "Field is empty in control file: {field}"),
            Error::UnexpectedOutput { expected, actual } => write!(
                f,
                "Unexpected number of lines in dpkg-deb output: expected {expected}, got {actual}"
            ),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Error::DpkgDebFailed(e) => Some(e),
            Error::CannotReadDpkgDebOutput(e) => Some(e),
            _ => None,
        }
    }
}
