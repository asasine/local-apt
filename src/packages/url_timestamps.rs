//! [`UrlTimestamps`] maps download URLs to their pool file paths.
//!
//! The HTTP `Last-Modified` timestamp is stored as the file's modification time,
//! avoiding a separate timestamp store.

use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};
use tracing::{debug, warn};

/// A mapping of download URLs to their downloaded pool file paths.
///
/// On download, the pool file's modification time is set to the HTTP `Last-Modified`
/// value. On subsequent requests, the file's mtime is read and sent as
/// `If-Modified-Since`, allowing servers to respond with `304 Not Modified`.
pub struct UrlTimestamps {
    path: PathBuf,
    entries: HashMap<String, PathBuf>,
}

impl UrlTimestamps {
    /// Load a timestamp mapping from the given file path.
    ///
    /// If the file doesn't exist, an empty mapping is returned.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, io::Error> {
        let path = path.as_ref().to_path_buf();
        let entries = match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => HashMap::new(),
            Err(e) => return Err(e),
        };

        debug!(
            "Loaded timestamp mapping with {} entries from {}",
            entries.len(),
            path.display()
        );
        Ok(UrlTimestamps { path, entries })
    }

    /// Get the `If-Modified-Since` value for a URL by reading the pool file's mtime.
    ///
    /// Returns [`None`] if the URL is not mapped or the file no longer exists.
    pub fn get_if_modified_since(&self, url: &str) -> Option<String> {
        let file_path = self.entries.get(url)?;
        let metadata = fs::metadata(file_path).ok()?;
        let mtime = metadata.modified().ok()?;
        Some(httpdate::fmt_http_date(mtime))
    }

    /// Record that a URL was downloaded to the given pool file path.
    ///
    /// If `last_modified` is provided, the file's modification time is set to match.
    pub fn set(&mut self, url: String, file_path: PathBuf, last_modified: Option<&str>) {
        if let Some(last_modified) = last_modified {
            if let Ok(mtime) = httpdate::parse_http_date(last_modified) {
                let times = fs::FileTimes::new().set_modified(mtime);
                match fs::File::options().write(true).open(&file_path) {
                    Ok(file) => {
                        if let Err(e) = file.set_times(times) {
                            warn!("Failed to set mtime on {}: {}", file_path.display(), e);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to open {} to set mtime: {}", file_path.display(), e);
                    }
                }
            } else {
                warn!("Failed to parse Last-Modified header: {:?}", last_modified);
            }
        }

        self.entries.insert(url, file_path);
    }

    /// Save the mapping to disk.
    pub fn save(&self) -> Result<(), io::Error> {
        let content = serde_json::to_string_pretty(&self.entries)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(&self.path, content)?;
        debug!(
            "Saved timestamp mapping with {} entries to {}",
            self.entries.len(),
            self.path.display()
        );
        Ok(())
    }
}
