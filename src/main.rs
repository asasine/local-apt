use anyhow::{Context, Result, anyhow};
use clap::Parser;
use fs2::FileExt;
use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use syslog_tracing::{Facility, Options, Syslog};
use tempfile::TempDir;
use tracing::{error, info, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

mod cli;

use cli::Cli;

const DEFAULT_CONFIG_FILE: &str = "/etc/local-apt/packages.conf";
const REPO_DIR: &str = "/var/lib/local-apt";
const LOCKFILE: &str = "/run/lock/local-apt.lock";

struct Config {
    config_file: PathBuf,
    pool_dir: PathBuf,
    lockfile: PathBuf,
}

impl Config {
    fn new() -> Self {
        let config_file = env::var("LOCAL_APT_CONFIG")
            .unwrap_or_else(|_| DEFAULT_CONFIG_FILE.to_string())
            .into();
        let repo_dir = PathBuf::from(REPO_DIR);
        let pool_dir = repo_dir.join("pool/main");
        let lockfile = PathBuf::from(LOCKFILE);

        Config {
            config_file,
            pool_dir,
            lockfile,
        }
    }
}

/// Initialize tracing subscriber
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

/// Parse configuration file and return list of package URLs
fn parse_packages_config<P: AsRef<Path>>(config_file: P) -> Result<Vec<String>> {
    let file = File::open(config_file.as_ref()).context("Failed to open configuration file")?;
    let reader = BufReader::new(file);

    let mut urls = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        urls.push(trimmed.to_string());
    }

    Ok(urls)
}

/// Download and process a single package
fn process_package(url: &str, pool_dir: &Path, temp_dir: &Path) -> Result<()> {
    info!("Processing package from: {}", url);

    // Download to temp directory
    let download_file = temp_dir.join(format!("package-{}.deb", std::process::id()));

    let response = reqwest::blocking::get(url).context("Failed to download package")?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Download failed with status: {}",
            response.status()
        ));
    }

    let mut file = File::create(&download_file).context("Failed to create temporary file")?;
    let content = response
        .bytes()
        .context("Failed to read response content")?;
    file.write_all(&content)
        .context("Failed to write downloaded content")?;
    drop(file);

    // Verify it's a valid .deb file
    let verify_output = Command::new("dpkg-deb")
        .arg("-I")
        .arg(&download_file)
        .output()
        .context("Failed to run dpkg-deb")?;

    if !verify_output.status.success() {
        fs::remove_file(&download_file).ok();
        return Err(anyhow!("Downloaded file is not a valid .deb package"));
    }

    // Extract package metadata
    let pkg_name =
        get_deb_field(&download_file, "Package").context("Could not extract package name")?;
    let pkg_version =
        get_deb_field(&download_file, "Version").context("Could not extract package version")?;
    let pkg_arch =
        get_deb_field(&download_file, "Architecture").unwrap_or_else(|_| "all".to_string());

    // Construct standard Debian package filename
    let std_filename = format!("{}_{}__{}.deb", pkg_name, pkg_version, pkg_arch);

    // Auto-generate target path following Debian pool convention
    // pool/main/<first-letter>/<package-name>/
    let first_letter = pkg_name
        .chars()
        .next()
        .ok_or_else(|| anyhow!("Package name is empty"))?;
    let target_dir = pool_dir.join(first_letter.to_string()).join(&pkg_name);

    // Create target directory if it doesn't exist
    fs::create_dir_all(&target_dir).context("Failed to create target directory")?;

    // Move the .deb file to target directory with standard naming
    let target_path = target_dir.join(&std_filename);
    fs::rename(&download_file, &target_path)
        .context("Failed to move package to target directory")?;

    info!(
        "Successfully installed {} to {}",
        pkg_name,
        target_path.display()
    );

    Ok(())
}

/// Extract a field from a .deb package
fn get_deb_field<P: AsRef<Path>>(deb_file: P, field: &str) -> Result<String> {
    let output = Command::new("dpkg-deb")
        .arg("-f")
        .arg(deb_file.as_ref())
        .arg(field)
        .output()
        .context("Failed to run dpkg-deb")?;

    if !output.status.success() {
        return Err(anyhow!("dpkg-deb failed to extract field {}", field));
    }

    let value = String::from_utf8(output.stdout)
        .context("Invalid UTF-8 in dpkg-deb output")?
        .trim()
        .to_string();

    if value.is_empty() {
        return Err(anyhow!("Field {} is empty", field));
    }

    Ok(value)
}

/// Update repository metadata by calling update-local-repo
fn update_repository_metadata() -> Result<()> {
    info!("Updating repository metadata...");

    let status = Command::new("update-local-repo")
        .status()
        .context("Failed to run update-local-repo")?;

    if !status.success() {
        return Err(anyhow!("update-local-repo failed"));
    }

    Ok(())
}

fn main() -> Result<()> {
    init_logger();

    let args = Cli::parse();
    match args {
        Cli::Update(update_args) => {
            info!("Running update command with args: {:?}", update_args);
        }
    }

    return Ok(());

    let config = Config::new();

    // Check if config file exists
    if !config.config_file.exists() {
        error!(
            "Configuration file not found: {}",
            config.config_file.display()
        );
        return Err(anyhow!("Configuration file not found"));
    }

    // Acquire lock to prevent concurrent runs
    let lockfile = File::create(&config.lockfile).context("Failed to create lockfile")?;

    if lockfile.try_lock_exclusive().is_err() {
        error!("Another instance of update-packages is already running");
        return Err(anyhow!("Another instance is already running"));
    }

    info!("Starting package update process");

    // Create temporary directory
    let temp_dir = TempDir::new().context("Failed to create temporary directory")?;

    // Parse configuration
    let urls =
        parse_packages_config(&config.config_file).context("Failed to parse configuration file")?;

    // Process each package
    let mut success_count = 0;
    let mut failure_count = 0;

    for url in urls {
        match process_package(&url, &config.pool_dir, temp_dir.path()) {
            Ok(_) => success_count += 1,
            Err(e) => {
                warn!("Failed to process {}: {}", url, e);
                failure_count += 1;
            }
        }
    }

    // Update repository metadata if any packages succeeded
    if success_count > 0 {
        match update_repository_metadata() {
            Ok(_) => {
                info!(
                    "Repository update complete: {} successful, {} failed",
                    success_count, failure_count
                );
            }
            Err(e) => {
                error!("Failed to update repository metadata: {}", e);
                return Err(e);
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
