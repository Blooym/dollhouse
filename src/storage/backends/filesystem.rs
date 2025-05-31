use crate::storage::{StorageCapabilities, StorageOperations};
use anyhow::{Context, Result};
use std::{
    fs::{self, File, FileTimes},
    io::{self, Read},
    time::SystemTime,
};
use tracing::debug;

#[derive(Debug, Clone)]
pub struct FilesystemStorage {
    base_path: std::path::PathBuf,
}

impl FilesystemStorage {
    pub fn new(base_path: std::path::PathBuf) -> Result<Self> {
        let _ = fs::create_dir_all(&base_path);
        Ok(Self {
            base_path: fs::canonicalize(base_path)?,
        })
    }
}

impl FilesystemStorage {
    fn join_to_base(&self, path: &std::path::Path) -> Result<std::path::PathBuf> {
        for component in path.components() {
            match component {
                std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        format!("Absolute paths are not allowed: {:?}", path),
                    )
                    .into());
                }
                std::path::Component::ParentDir => {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        format!("Paths cannot reference a parent directory: {:?}", path),
                    )
                    .into());
                }
                _ => {}
            }
        }
        Ok(self.base_path.join(path))
    }
}

impl StorageCapabilities for FilesystemStorage {
    fn supports_expiry(&self) -> bool {
        true
    }
}

impl StorageOperations for FilesystemStorage {
    async fn read(&self, path: &std::path::Path) -> Result<Option<Vec<u8>>> {
        let path = self.base_path.join(path);

        let metadata = match fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err.into()),
        };
        debug!("Updating access time for file {path:?}");
        let mut file = match File::options().read(true).write(true).open(&path) {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err.into()),
        };
        let _ = file.set_times(
            FileTimes::new()
                .set_accessed(SystemTime::now())
                .set_modified(metadata.modified()?),
        );
        debug!("Reading file at {path:?}");
        let mut buf = Vec::new();
        match file.read_to_end(&mut buf) {
            Ok(_) => Ok(Some(buf)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    async fn write(&mut self, path: &std::path::Path, data: &[u8]) -> Result<()> {
        let path = &self.join_to_base(path)?;
        debug!("Reading file at {path:?}");
        fs::create_dir_all(
            path.parent()
                .expect("path should always have parent when joined to base"),
        )
        .context(format!("failed to create directories for {path:?}"))?;
        Ok(fs::write(path, data)?)
    }

    async fn delete(&mut self, path: &std::path::Path) -> Result<bool> {
        let path = self.join_to_base(path)?;
        debug!("Deleting file at {path:?}");
        match fs::remove_file(path) {
            Ok(_) => Ok(true),
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    return Ok(false);
                }
                Err(err.into())
            }
        }
    }

    async fn exists(&self, path: &std::path::Path) -> Result<bool> {
        let path = self.join_to_base(path)?;
        debug!("Checking for file at {path:?}");
        Ok(fs::exists(path)?)
    }

    async fn list(&self, path: &std::path::Path) -> Result<Vec<std::path::PathBuf>> {
        let full_path = self.join_to_base(path)?;
        debug!("Listing all files inside of {full_path:?}");
        if !full_path.exists() {
            return Ok(Vec::new());
        }
        Ok(fs::read_dir(full_path)?
            .filter_map(Result::ok)
            .filter_map(|entry| {
                entry
                    .path()
                    .strip_prefix(&self.base_path)
                    .ok()
                    .map(|p| p.to_path_buf())
            })
            .collect())
    }

    async fn last_access(&self, path: &std::path::Path) -> Result<Option<std::time::SystemTime>> {
        let path = self.join_to_base(path)?;
        debug!("Obtaining last access time for {path:?}");
        let metadata = fs::metadata(path)?;
        Ok(Some(metadata.accessed()?))
    }
}
