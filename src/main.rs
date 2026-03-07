use anyhow::{Context, anyhow};
use clap::Parser;
use local_apt::{
    cli::Cli,
    external::update_repository_metadata,
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
            let _lock = config.lockfile.lock()?;

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
                info!("No packages configured or all packages disabled");
            }

            // Lock will be automatically released when the file is dropped
            Ok(())
        }
    }
}
