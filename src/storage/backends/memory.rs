use crate::storage::{StorageCapabilities, StorageOperations};
use anyhow::Result;
use dashmap::DashMap;
use std::{path::PathBuf, time::SystemTime};

#[derive(Debug, Clone)]
pub struct MemoryStorage {
    memory: DashMap<PathBuf, (Vec<u8>, SystemTime)>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        MemoryStorage {
            memory: DashMap::new(),
        }
    }
}

impl StorageCapabilities for MemoryStorage {
    fn supports_expiry(&self) -> bool {
        true
    }
}

impl StorageOperations for MemoryStorage {
    async fn read(&self, path: &std::path::Path) -> Result<Option<Vec<u8>>> {
        if let Some(mut entry) = self.memory.get_mut(path) {
            let (data, access_time) = entry.value_mut();
            let data = data.clone();
            *access_time = SystemTime::now();
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    async fn write(&mut self, path: &std::path::Path, data: &[u8]) -> Result<()> {
        self.memory
            .insert(path.to_path_buf(), (data.to_vec(), SystemTime::now()));
        Ok(())
    }

    async fn delete(&mut self, path: &std::path::Path) -> Result<bool> {
        Ok(self.memory.remove(path).is_some())
    }

    async fn exists(&self, path: &std::path::Path) -> Result<bool> {
        Ok(self.memory.contains_key(path))
    }

    async fn list(&self, path: &std::path::Path) -> Result<Vec<PathBuf>> {
        Ok(self
            .memory
            .iter()
            .filter(|entry| entry.key().starts_with(path))
            .map(|entry| entry.key().clone())
            .collect())
    }

    async fn last_access(&self, path: &std::path::Path) -> Result<Option<SystemTime>> {
        Ok(self.memory.get(path).map(|entry| entry.value().1))
    }
}
