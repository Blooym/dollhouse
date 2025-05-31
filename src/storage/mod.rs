mod app_storage;
pub use app_storage::AppStorage;
mod backends;

use anyhow::Result;
use core::str::FromStr;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub trait StorageCapabilities {
    fn supports_expiry(&self) -> bool;
}

pub trait StorageOperations: StorageCapabilities {
    async fn read(&self, path: &Path) -> Result<Option<Vec<u8>>>;
    async fn write(&mut self, path: &Path, data: &[u8]) -> Result<()>;
    async fn delete(&mut self, path: &Path) -> Result<bool>;
    async fn exists(&self, path: &Path) -> Result<bool>;
    async fn list(&self, path: &Path) -> Result<Vec<PathBuf>>;
    async fn last_access(&self, path: &Path) -> Result<Option<SystemTime>>;
}

#[derive(Debug, Clone)]
pub enum StorageProvider {
    #[cfg(feature = "storage-memory")]
    Memory(backends::MemoryStorage),
    #[cfg(feature = "storage-filesystem")]
    Filesystem(backends::FilesystemStorage),
    #[cfg(feature = "storage-s3")]
    S3(backends::S3Storage),
}

impl StorageCapabilities for StorageProvider {
    fn supports_expiry(&self) -> bool {
        match self {
            #[cfg(feature = "storage-memory")]
            StorageProvider::Memory(storage) => storage.supports_expiry(),
            #[cfg(feature = "storage-filesystem")]
            StorageProvider::Filesystem(storage) => storage.supports_expiry(),
            #[cfg(feature = "storage-s3")]
            StorageProvider::S3(storage) => storage.supports_expiry(),
        }
    }
}

impl StorageOperations for StorageProvider {
    async fn read(&self, path: &Path) -> Result<Option<Vec<u8>>> {
        match self {
            #[cfg(feature = "storage-memory")]
            StorageProvider::Memory(storage) => storage.read(path).await,
            #[cfg(feature = "storage-filesystem")]
            StorageProvider::Filesystem(storage) => storage.read(path).await,
            #[cfg(feature = "storage-s3")]
            StorageProvider::S3(storage) => storage.read(path).await,
        }
    }

    async fn write(&mut self, path: &Path, data: &[u8]) -> Result<()> {
        match self {
            #[cfg(feature = "storage-memory")]
            StorageProvider::Memory(storage) => storage.write(path, data).await,
            #[cfg(feature = "storage-filesystem")]
            StorageProvider::Filesystem(storage) => storage.write(path, data).await,
            #[cfg(feature = "storage-s3")]
            StorageProvider::S3(storage) => storage.write(path, data).await,
        }
    }

    async fn delete(&mut self, path: &Path) -> Result<bool> {
        match self {
            #[cfg(feature = "storage-memory")]
            StorageProvider::Memory(storage) => storage.delete(path).await,
            #[cfg(feature = "storage-filesystem")]
            StorageProvider::Filesystem(storage) => storage.delete(path).await,
            #[cfg(feature = "storage-s3")]
            StorageProvider::S3(storage) => storage.delete(path).await,
        }
    }

    async fn exists(&self, path: &Path) -> Result<bool> {
        match self {
            #[cfg(feature = "storage-memory")]
            StorageProvider::Memory(storage) => storage.exists(path).await,
            #[cfg(feature = "storage-filesystem")]
            StorageProvider::Filesystem(storage) => storage.exists(path).await,
            #[cfg(feature = "storage-s3")]
            StorageProvider::S3(storage) => storage.exists(path).await,
        }
    }

    async fn list(&self, path: &Path) -> Result<Vec<PathBuf>> {
        match self {
            #[cfg(feature = "storage-memory")]
            StorageProvider::Memory(storage) => storage.list(path).await,
            #[cfg(feature = "storage-filesystem")]
            StorageProvider::Filesystem(storage) => storage.list(path).await,
            #[cfg(feature = "storage-s3")]
            StorageProvider::S3(storage) => storage.list(path).await,
        }
    }

    async fn last_access(&self, path: &Path) -> Result<Option<SystemTime>> {
        match self {
            #[cfg(feature = "storage-memory")]
            StorageProvider::Memory(storage) => storage.last_access(path).await,
            #[cfg(feature = "storage-filesystem")]
            StorageProvider::Filesystem(storage) => storage.last_access(path).await,
            #[cfg(feature = "storage-s3")]
            StorageProvider::S3(storage) => storage.last_access(path).await,
        }
    }
}

impl FromStr for StorageProvider {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            #[cfg(feature = "storage-memory")]
            "memory://" => Ok(Self::Memory(backends::MemoryStorage::new())),

            #[cfg(feature = "storage-filesystem")]
            _ if s.starts_with("fs://") => {
                use faccess::{AccessMode, PathExt};

                let s = PathBuf::from(s.trim_start_matches("fs://").trim());
                let _ = std::fs::create_dir_all(&s);
                if let Err(err) = s.access(AccessMode::WRITE | AccessMode::READ) {
                    return Err(format!(
                        "Path specified cannot be read from or written to by the current user\n\nError: {err}"
                    ));
                }
                Ok(Self::Filesystem(
                    backends::FilesystemStorage::new(s)
                        .map_err(|err| format!("Failed to create filesystem storage: {err:?}"))?,
                ))
            }

            #[cfg(feature = "storage-s3")]
            _ if s.starts_with("s3://") => {
                let bucket = s
                    .trim_start_matches("s3://")
                    .split('/')
                    .next()
                    .ok_or("S3 URL must include bucket: s3://bucket")?;

                if bucket.is_empty() {
                    return Err("S3 bucket name cannot be empty".to_string());
                }

                Ok(Self::S3(
                    backends::S3Storage::new(bucket.to_string())
                        .map_err(|err| format!("failed to create S3 client: {err:?}"))?,
                ))
            }

            _ => {
                let mut valid_sources = Vec::new();
                #[cfg(feature = "storage-memory")]
                valid_sources.push("'memory://'");
                #[cfg(feature = "storage-filesystem")]
                valid_sources.push("'fs://path'");
                #[cfg(feature = "storage-s3")]
                valid_sources.push("'s3://bucket'");

                if valid_sources.is_empty() {
                    Err("No storage backends are enabled".to_string())
                } else {
                    Err(format!("Valid sources are: {}", valid_sources.join(", ")))
                }
            }
        }
    }
}
