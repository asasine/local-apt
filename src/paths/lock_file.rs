//! [`LockedLockFile`] ensures only one instance of the application runs at a time.
//!
//! The application first creates an [`UnlockedLockFile`] at the default path (`/var/lock/local-apt.lock`)
//! and attempts to acquire an exclusive lock on it. If successful, the application
//! continues running and holds the lock until it exits. If another instance is
//! already running and holds the lock, the application will fail to acquire the
//! lock and exit with an error.

use core::fmt::Display;
use fs2::FileExt;
use std::{fs::File, path::PathBuf};
use tracing::{debug, error};

/// The lock file ensures only one instance of the application runs at a time.
///
/// It is created at the start of the application and deleted at the end. If another
/// instance is already running when the application attempts to acquire the lock,
/// the application will exit with an error.
#[derive(Debug, Clone)]
struct LockFile(PathBuf);

impl Default for LockFile {
    /// Creates an unlocked lock file at the default path (`/var/lock/local-apt.lock`).
    fn default() -> Self {
        Self(PathBuf::from("/var/lock/local-apt.lock"))
    }
}

/// An unlocked lock file that can be locked with [`lock`][`UnlockedLockFile::lock`]
/// to create a [`LockedLockFile`].
#[derive(Debug, Default, Clone)]
pub struct UnlockedLockFile(LockFile);

impl UnlockedLockFile {
    /// Attempt to acquire an exclusive lock on the lock file.
    ///
    /// If successful, returns a [`LockedLockFile`] that holds the lock until dropped.
    /// If another instance is already running and holds the lock, returns an error.
    /// If the file cannot be created or accessed due to permissions, returns an error.
    pub fn lock(self) -> Result<LockedLockFile, LockError> {
        let lock = File::create(&self.0.0).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                LockError::PermissionDenied(e)
            } else {
                LockError::Create(e)
            }
        })?;

        if lock.try_lock_exclusive().is_err() {
            error!("Another instance of local-apt is already running");
            return Err(LockError::AlreadyLocked);
        }

        Ok(LockedLockFile {
            lock_file: self.0,
            lock: Some(lock),
        })
    }
}

/// A locked lock file that indicates the application may continue running.
///
/// Acquired through [`UnlockedLockFile::lock`] and held until dropped.
#[derive(Debug)]
pub struct LockedLockFile {
    /// The path to the lock file.
    lock_file: LockFile,

    /// The file handle that holds the lock.
    ///
    /// This is unlocked when the [`LockedLockFile`] is dropped or explicitly unlocked with
    /// [`unlock`][`Self::unlock`]. When [`None`], the lock has already been released.
    lock: Option<File>,
}

impl LockedLockFile {
    /// Unlock the lock file and return the underlying [`UnlockedLockFile`].
    pub fn unlock(mut self) -> Result<UnlockedLockFile, std::io::Error> {
        if let Some(lock) = self.lock.take() {
            debug!("Lock file {} is being released", self.lock_file.0.display());
            lock.unlock()?;
        }

        Ok(UnlockedLockFile(self.lock_file.clone()))
    }
}

impl Drop for LockedLockFile {
    /// Automatically unlock the lock file when the [`LockedLockFile`] is dropped.
    ///
    /// Explicitly unlocking with [`unlock`][`Self::unlock`] is possible to handle
    /// any errors that may occur during unlocking. This [`Drop`] implementation
    /// is a safety net to ensure the lock is released even if the application panics
    /// or exits unexpectedly. It will log any errors that occur during unlocking.
    /// It will no-op if the lock has already been released.
    fn drop(&mut self) {
        if let Some(lock) = self.lock.take() {
            debug!(
                "Lock file {} is being automatically released",
                self.lock_file.0.display()
            );

            // can't panic in Drop, so just log any errors when unlocking
            if let Err(e) = lock.unlock() {
                error!("Failed to release lock: {}", e);
            }
        }
    }
}

/// An error that can occur when locking the lock file.
#[derive(Debug)]
pub enum LockError {
    /// Failed to create the lock file.
    Create(std::io::Error),

    /// Insufficient permissions to create or write to the lock file.
    PermissionDenied(std::io::Error),

    /// Failed to acquire an exclusive lock on the file.
    AlreadyLocked,
}

impl Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockError::Create(e) => write!(f, "Failed to create lock file: {}", e),
            LockError::PermissionDenied(e) => write!(f, "Permission denied on lock file: {}", e),
            LockError::AlreadyLocked => write!(f, "Another instance is already running"),
        }
    }
}

impl core::error::Error for LockError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            LockError::Create(e) => Some(e),
            LockError::PermissionDenied(e) => Some(e),
            LockError::AlreadyLocked => None,
        }
    }
}
