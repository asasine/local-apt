mod logging;
mod output;

use clap::Parser;
use local_apt::{
    cli::{Cli, RepoArgs},
    external::{
        CommandError, Error as MetadataError, GetDebFieldsError, dpkg_version_is_greater,
        get_deb_fields, update_repository_metadata,
    },
    packages::{ProcessResult, UrlTimestamps},
    paths::{ConfigFile, LockError, LockedLockFile, ReadPackagesError, StateDir, UnlockedLockFile},
};
use output::{Event, Outcome, Reporter, Summary};
use std::{io, path::PathBuf, process::ExitCode};
use tempfile::TempDir;
use tracing::{error, info, warn};

struct Paths {
    config_file: ConfigFile,
    state_dir: StateDir,
    lockfile: UnlockedLockFile,
}

impl Default for Paths {
    fn default() -> Self {
        Self {
            config_file: ConfigFile::env_or_default(),
            state_dir: StateDir::default(),
            lockfile: UnlockedLockFile::default(),
        }
    }
}

fn main() -> ExitCode {
    human_panic::setup_panic!();

    let cli = Cli::parse();
    let (args, command) = cli.parts();

    if let Err(e) = logging::init(args) {
        eprintln!("local-apt: {e}");
        return ExitCode::FAILURE;
    }

    let stdout = io::stdout();
    let mut reporter = Reporter::new(args.output, stdout.lock(), command.clone());
    if let Err(e) = reporter.emit(Event::Started { command }) {
        error!("{e}");
        return ExitCode::FAILURE;
    }

    let result = match &cli {
        Cli::Update(args) => run_update(args, &mut reporter),
        Cli::Cleanup(args) => run_cleanup(args, &mut reporter),
    };

    match result {
        Ok((outcome, summary)) => {
            let strict_partial = partial_is_failure(&outcome, args.fail_on_partial);
            if let Err(e) = reporter.finish(outcome, summary) {
                error!("{e}");
                return ExitCode::FAILURE;
            }

            if strict_partial {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            error!("{e}");
            if let Err(output_error) = reporter
                .emit(Event::Fatal {
                    error: e.to_string(),
                })
                .and_then(|_| reporter.finish(Outcome::Failure, Summary::default()))
            {
                error!("{output_error}");
            }
            ExitCode::FAILURE
        }
    }
}

fn partial_is_failure(outcome: &Outcome, fail_on_partial: bool) -> bool {
    matches!(outcome, Outcome::PartialSuccess) && fail_on_partial
}

fn run_update<W: io::Write>(
    args: &RepoArgs,
    reporter: &mut Reporter<W>,
) -> Result<(Outcome, Summary), AppError> {
    let config = Paths {
        state_dir: args.state_dir(),
        ..Default::default()
    };
    info!(repository = %config.state_dir.path().display(), "starting package update");

    let _lock = acquire_lock(config.lockfile, reporter)?;
    let temp_dir = TempDir::new().map_err(AppError::CreateTemporaryDirectory)?;
    let mut url_timestamps = UrlTimestamps::load(config.state_dir.url_timestamps_path())
        .map_err(AppError::LoadTimestamps)?;
    let pool_dir = config.state_dir.pool_dir();
    let config_path = config.config_file.to_string();
    let packages =
        config
            .config_file
            .read_packages()
            .map_err(|source| AppError::ReadConfiguration {
                path: config_path,
                source: Box::new(source),
            })?;

    let mut downloaded = 0_u64;
    let mut unchanged = 0_u64;
    let mut failed = 0_u64;

    for package in packages.packages {
        let source = package.source_label();
        match package.process(&pool_dir, &temp_dir, &mut url_timestamps) {
            Ok(ProcessResult::Downloaded {
                source,
                package,
                version,
                path,
            }) => {
                downloaded += 1;
                reporter.emit(Event::Downloaded {
                    source,
                    package,
                    version,
                    path: path.display().to_string(),
                })?;
            }
            Ok(ProcessResult::AlreadyUpToDate { source }) => {
                unchanged += 1;
                reporter.emit(Event::Unchanged { source })?;
            }
            Err(e) => {
                warn!(source, error = %e, "package processing failed");
                failed += 1;
                reporter.emit(Event::PackageFailed {
                    source,
                    error: e.to_string(),
                })?;
            }
        }
    }

    if let Err(e) = url_timestamps.save() {
        let message = format!("failed to save URL timestamp state: {e}");
        warn!("{message}");
        reporter.emit(Event::Warning { message })?;
    }

    if downloaded > 0 {
        update_repository_metadata(config.state_dir.path())?;
        reporter.emit(Event::MetadataUpdated)?;
    }

    if failed > 0 && downloaded + unchanged == 0 {
        return Err(AppError::AllPackagesFailed);
    }

    let summary = Summary {
        downloaded: Some(downloaded),
        unchanged: Some(unchanged),
        failed: Some(failed),
        ..Summary::default()
    };
    let outcome = if failed > 0 {
        Outcome::PartialSuccess
    } else {
        Outcome::Success
    };
    info!(downloaded, unchanged, failed, "package update completed");
    Ok((outcome, summary))
}

fn run_cleanup<W: io::Write>(
    args: &RepoArgs,
    reporter: &mut Reporter<W>,
) -> Result<(Outcome, Summary), AppError> {
    let config = Paths {
        state_dir: args.state_dir(),
        ..Default::default()
    };
    info!(repository = %config.state_dir.path().display(), "starting package cleanup");
    let _lock = acquire_lock(config.lockfile, reporter)?;
    let state_dir = config.state_dir;
    let packages = state_dir
        .pool_dir()
        .deb_files_by_package()
        .map_err(AppError::ReadPool)?;
    let mut deleted = 0_u64;
    let mut kept = 0_u64;

    for deb_files in packages {
        if deb_files.len() <= 1 {
            kept += deb_files.len() as u64;
            continue;
        }

        let mut versioned_files: Vec<(String, String, PathBuf)> = Vec::new();
        for deb_file in &deb_files {
            match get_deb_fields(deb_file, &["Package", "Version"]) {
                Ok([package, version]) => {
                    versioned_files.push((package, version, deb_file.clone()));
                }
                Err(e) => {
                    let message = format!(
                        "failed to read package version from {}: {e}",
                        deb_file.display()
                    );
                    warn!("{message}");
                    reporter.emit(Event::Warning { message })?;
                    kept += 1;
                }
            }
        }

        if versioned_files.len() <= 1 {
            kept += versioned_files.len() as u64;
            continue;
        }

        let mut latest_idx = 0;
        for i in 1..versioned_files.len() {
            if dpkg_version_is_greater(&versioned_files[i].1, &versioned_files[latest_idx].1)? {
                latest_idx = i;
            }
        }

        for (i, (package, version, path)) in versioned_files.iter().enumerate() {
            if i == latest_idx {
                kept += 1;
                reporter.emit(Event::Kept {
                    package: package.clone(),
                    version: version.clone(),
                    path: path.display().to_string(),
                })?;
            } else {
                std::fs::remove_file(path).map_err(|source| AppError::DeletePackage {
                    path: path.clone(),
                    source,
                })?;
                deleted += 1;
                reporter.emit(Event::Deleted {
                    package: package.clone(),
                    version: version.clone(),
                    path: path.display().to_string(),
                })?;
            }
        }
    }

    if deleted > 0 {
        update_repository_metadata(state_dir.path())?;
        reporter.emit(Event::MetadataUpdated)?;
    }

    let summary = Summary {
        deleted: Some(deleted),
        kept: Some(kept),
        ..Summary::default()
    };
    info!(deleted, kept, "package cleanup completed");
    Ok((Outcome::Success, summary))
}

fn acquire_lock<W: io::Write>(
    lockfile: UnlockedLockFile,
    reporter: &mut Reporter<W>,
) -> Result<Option<LockedLockFile>, AppError> {
    match lockfile.lock() {
        Ok(lock) => Ok(Some(lock)),
        Err(LockError::PermissionDenied(source)) => {
            let message = format!(
                "lock unavailable due to permission denial; proceeding without it: {source}"
            );
            warn!("{message}");
            reporter.emit(Event::Warning { message })?;
            Ok(None)
        }
        Err(source) => Err(AppError::AcquireLock(source)),
    }
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("failed to acquire repository lock: {0}")]
    AcquireLock(LockError),
    #[error("failed to read configuration {path}: {source}")]
    ReadConfiguration {
        path: String,
        source: Box<ReadPackagesError>,
    },
    #[error("failed to create temporary directory: {0}")]
    CreateTemporaryDirectory(io::Error),
    #[error("failed to load URL timestamp state: {0}")]
    LoadTimestamps(io::Error),
    #[error("failed to read package pool: {0}")]
    ReadPool(io::Error),
    #[error("failed to compare package versions: {0}")]
    CompareVersions(#[from] CommandError),
    #[error("failed to inspect package: {0}")]
    InspectPackage(#[from] GetDebFieldsError),
    #[error("failed to delete {path}: {source}")]
    DeletePackage { path: PathBuf, source: io::Error },
    #[error(transparent)]
    UpdateMetadata(#[from] MetadataError),
    #[error(transparent)]
    Output(#[from] output::Error),
    #[error("all configured packages failed")]
    AllPackagesFailed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_success_is_strict_only_when_requested() {
        assert!(!partial_is_failure(&Outcome::PartialSuccess, false));
        assert!(partial_is_failure(&Outcome::PartialSuccess, true));
        assert!(!partial_is_failure(&Outcome::Success, true));
    }
}
