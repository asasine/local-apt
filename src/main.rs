use anyhow::{Context, anyhow};
use clap::Parser;
use local_apt::{
    cli::Cli,
    external::{get_deb_fields, update_repository_metadata},
    packages::{ProcessResult, UrlTimestamps},
    paths::{ConfigFile, StateDir, UnlockedLockFile},
};
use syslog_tracing::{Facility, Options, Syslog};
use tempfile::TempDir;
use tracing::{error, info, warn};
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

struct Paths {
    config_file: ConfigFile,
    state_dir: StateDir,
    lockfile: UnlockedLockFile,
}

impl Default for Paths {
    fn default() -> Self {
        Paths {
            config_file: ConfigFile::env_or_default(),
            state_dir: StateDir::default(),
            lockfile: UnlockedLockFile::default(),
        }
    }
}
/// Initialize tracing subscriber with both syslog and stderr outputs.
///
/// # Evironment Variables
/// - `RUST_LOG`: Set the log level (e.g., `info`, `debug`, `error`). Defaults to
///   `info` if not set.
fn init_logger() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let syslog = Syslog::new(c"apt-local", Options::LOG_PID, Facility::User).unwrap();
    tracing_subscriber::registry()
        .with(env_filter)
        .with(vec![
            tracing_subscriber::fmt::layer()
                .with_writer(syslog)
                .with_ansi(false)
                .without_time()
                .with_level(false)
                .with_target(false)
                .boxed(),
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .boxed(),
        ])
        .init();
}

fn main() -> anyhow::Result<()> {
    init_logger();

    let args = Cli::parse();
    match args {
        Cli::Update(update_args) => {
            info!("Running update command with args: {:?}", update_args);
            let config = Paths {
                state_dir: update_args.state_dir(),
                ..Default::default()
            };

            // Check if config file exists
            if !config.config_file.exists() {
                error!("Configuration file not found: {}", config.config_file);
                return Err(anyhow!("Configuration file not found"));
            }

            // Acquire lock to prevent concurrent runs, will automatically release when dropped
            let _lock = match config.lockfile.lock() {
                Ok(lock) => Some(lock),
                Err(local_apt::paths::LockError::PermissionDenied(e)) => {
                    info!(
                        "Could not acquire lock file (permission denied: {}), proceeding without lock",
                        e
                    );

                    None
                }
                Err(e) => return Err(e.into()),
            };

            info!("Starting package update process");

            // Create temporary directory
            let temp_dir = TempDir::new().context("Failed to create temporary directory")?;

            // Load URL timestamps mapping for conditional downloads
            let mut url_timestamps = UrlTimestamps::load(config.state_dir.url_timestamps_path())
                .context("Failed to load URL timestamps mapping")?;

            let pool_dir = config.state_dir.pool_dir();

            // Parse configuration
            let packages = config
                .config_file
                .read_packages()
                .context("Failed to parse configuration file")?;

            // Process each package
            let mut success_count = 0;
            let mut up_to_date_count = 0;
            let mut failure_count = 0;

            for package in packages.packages {
                match package.process(&pool_dir, &temp_dir, &mut url_timestamps) {
                    Ok(ProcessResult::Downloaded) => success_count += 1,
                    Ok(ProcessResult::AlreadyUpToDate) => up_to_date_count += 1,
                    Err(e) => {
                        warn!("Failed to process {:?}: {}", package, e);
                        failure_count += 1;
                    }
                }
            }

            // Save URL timestamps mapping
            if let Err(e) = url_timestamps.save() {
                warn!("Failed to save URL timestamps mapping: {}", e);
            }

            // Update repository metadata if any packages succeeded
            if success_count > 0 {
                match update_repository_metadata(config.state_dir.path()) {
                    Ok(()) => {
                        info!(
                            "Repository update complete: {} downloaded, {} up-to-date, {} failed",
                            success_count, up_to_date_count, failure_count
                        );
                    }
                    Err(e) => {
                        error!("Failed to update repository metadata: {}", e);
                        return Err(e.into());
                    }
                }
            } else if failure_count > 0 {
                warn!("No packages were successfully downloaded");
                return Err(anyhow!("No packages were successfully downloaded"));
            } else {
                info!(
                    "No packages configured, all packages disabled, or all packages already up-to-date"
                );
            }

            // Lock will be automatically released when the file is dropped
            Ok(())
        }
        Cli::Cleanup(cleanup_args) => {
            info!("Running cleanup command with args: {:?}", cleanup_args);
            let state_dir = cleanup_args.state_dir();
            let pool_dir = state_dir.pool_dir();
            let pool_path = pool_dir.path();

            if !pool_path.exists() {
                info!("Pool directory does not exist, nothing to clean up");
                return Ok(());
            }

            let mut deleted_count: u64 = 0;
            let mut kept_count: u64 = 0;

            // Iterate letter directories (e.g., pool/main/d/)
            let letter_dirs =
                std::fs::read_dir(pool_path).context("Failed to read pool directory")?;

            for letter_entry in letter_dirs {
                let letter_entry = letter_entry.context("Failed to read letter directory entry")?;
                if !letter_entry.file_type().map_or(false, |ft| ft.is_dir()) {
                    continue;
                }

                // Iterate package directories (e.g., pool/main/d/discord/)
                let pkg_dirs = std::fs::read_dir(letter_entry.path())
                    .context("Failed to read package directory")?;

                for pkg_entry in pkg_dirs {
                    let pkg_entry = pkg_entry.context("Failed to read package directory entry")?;
                    if !pkg_entry.file_type().map_or(false, |ft| ft.is_dir()) {
                        continue;
                    }

                    // Collect all .deb files in this package directory
                    let entries = std::fs::read_dir(pkg_entry.path())
                        .context("Failed to read package version directory")?;

                    let deb_files: Vec<std::path::PathBuf> = entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.path())
                        .filter(|p| p.extension().is_some_and(|ext| ext == "deb"))
                        .collect();

                    if deb_files.len() <= 1 {
                        kept_count += deb_files.len() as u64;
                        continue;
                    }

                    // Extract versions for each .deb file
                    let mut versioned_files: Vec<(String, std::path::PathBuf)> = Vec::new();
                    for deb_file in &deb_files {
                        match get_deb_fields(deb_file, &["Version"]) {
                            Ok([version]) => versioned_files.push((version, deb_file.clone())),
                            Err(e) => {
                                warn!("Failed to read version from {}: {}", deb_file.display(), e);
                            }
                        }
                    }

                    if versioned_files.len() <= 1 {
                        kept_count += versioned_files.len() as u64;
                        continue;
                    }

                    // Find the latest version using dpkg --compare-versions
                    let mut latest_idx = 0;
                    for i in 1..versioned_files.len() {
                        let status = std::process::Command::new("dpkg")
                            .args([
                                "--compare-versions",
                                &versioned_files[i].0,
                                "gt",
                                &versioned_files[latest_idx].0,
                            ])
                            .status()
                            .context("Failed to run dpkg --compare-versions")?;

                        if status.success() {
                            latest_idx = i;
                        }
                    }

                    // Delete all except the latest
                    for (i, (version, path)) in versioned_files.iter().enumerate() {
                        if i == latest_idx {
                            info!("Keeping {} (version {})", path.display(), version);
                            kept_count += 1;
                        } else {
                            info!("Deleting {} (version {})", path.display(), version);
                            std::fs::remove_file(path)
                                .with_context(|| format!("Failed to delete {}", path.display()))?;
                            deleted_count += 1;
                        }
                    }
                }
            }

            if deleted_count > 0 {
                update_repository_metadata(state_dir.path())
                    .map_err(|e| anyhow!("Failed to update repository metadata: {}", e))?;
            }

            info!(
                "Cleanup complete: {} deleted, {} kept",
                deleted_count, kept_count
            );
            Ok(())
        }
    }
}
