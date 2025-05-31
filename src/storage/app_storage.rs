use std::{
    path::Path,
    time::{Duration, SystemTime},
};

use super::{StorageCapabilities, StorageOperations, StorageProvider};
use crate::cryptography::Cryptography;
use anyhow::{Context, Result, bail};
use tracing::{debug, info};

pub struct AppStorage {
    provider: StorageProvider,
}

impl AppStorage {
    pub fn new(provider: StorageProvider) -> Self {
        Self { provider }
    }

    fn upload_path() -> &'static Path {
        Path::new("uploads/")
    }

    pub fn provider_supports_expiry(&self) -> bool {
        self.provider.supports_expiry()
    }

    pub async fn remove_all_expired_uploads(&mut self, expire_after: Duration) -> Result<()> {
        if !self.provider.supports_expiry() {
            return Ok(());
        }

        let paths = self.provider.list(Self::upload_path()).await?;
        for path in paths.iter() {
            if self.is_upload_expired(path, expire_after).await? {
                info!("file '{}' expired - deleting from storage.", path.display());
                self.provider.delete(path).await?;
            }
        }
        Ok(())
    }

    async fn is_upload_expired(&self, file: &Path, expire_after: Duration) -> Result<bool> {
        if !self.provider.supports_expiry() {
            return Ok(false);
        }
        let Some(last_access) = self.provider.last_access(file).await? else {
            bail!("File does not have a last access time");
        };
        Ok(last_access + expire_after <= SystemTime::now())
    }

    pub async fn get_upload(&self, id: &str, key: &str) -> Result<Vec<u8>> {
        debug!("Decrypting and fetching {id} from storage");
        let file = self
            .provider
            .read(&Self::upload_path().join(Path::new(id)))
            .await?
            .context("file does not exist")?;
        Cryptography::decrypt(&file, key, id.as_bytes())
    }

    pub async fn upload_exists(&self, id: &str) -> Result<bool> {
        debug!("Checking if {id} exists in storage");
        self.provider
            .exists(&Self::upload_path().join(Path::new(id)))
            .await
    }

    pub async fn save_upload(&mut self, id: &str, bytes: &[u8]) -> Result<String> {
        debug!("Encrypting and saving {id} to storage");
        let (key, bytes) = Cryptography::encrypt(bytes, id.as_bytes())?;
        self.provider
            .write(&Self::upload_path().join(id), &bytes)
            .await?;
        Ok(key)
    }

    pub async fn delete_upload(&mut self, id: &str) -> Result<()> {
        debug!("Deleting {id} from storage");
        self.provider.delete(&Self::upload_path().join(id)).await?;
        Ok(())
    }
}
