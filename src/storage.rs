use crate::cryptography::Cryptography;
use anyhow::Result;
use std::{
    fs::{self},
    path::PathBuf,
    time::{Duration, SystemTime},
};
use tracing::{debug, info};

#[derive(Debug)]
pub struct Storage {
    base_path: PathBuf,
    expire_after: Duration,
}

impl Storage {
    pub fn new(base_path: PathBuf, expire_after: Duration) -> Result<Self> {
        fs::create_dir_all(&base_path)?;
        Ok(Self {
            base_path,
            expire_after,
        })
    }

    fn is_file_expired(&self, filename: &str) -> Result<bool> {
        let metadata = fs::metadata(self.base_path.join(filename))?;
        let last_access = metadata.accessed()?;
        if last_access + self.expire_after <= SystemTime::now() {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn remove_all_expired_files(&self) -> Result<()> {
        fs::read_dir(&self.base_path)
            .unwrap()
            .filter_map(|f| f.ok())
            .for_each(|file| {
                let Ok(file_name) = file.file_name().into_string() else {
                    return;
                };
                if self.is_file_expired(&file_name).unwrap() {
                    info!("'{}' has expired - deleting from storage.", file_name);
                    self.delete_file(&file_name).unwrap();
                }
            });
        Ok(())
    }

    pub fn get_file(&self, filename: &str, key: &str) -> Result<Vec<u8>> {
        debug!("Decrypting and fetching {filename} from storage");
        Cryptography::decrypt(&fs::read(self.base_path.join(filename))?, key)
    }

    pub fn save_file(&self, filename: &str, bytes: &[u8]) -> Result<String> {
        debug!("Encrypting and saving {filename} to storage");
        let (key, bytes) = Cryptography::encrypt(bytes)?;
        fs::write(self.base_path.join(filename), bytes)?;
        Ok(key)
    }

    pub fn delete_file(&self, filename: &str) -> Result<()> {
        debug!("Deleting {filename} from storage");
        fs::remove_file(self.base_path.join(filename))?;
        Ok(())
    }

    pub fn file_exists(&self, filename: &str) -> Result<bool> {
        debug!("Checking if {filename} exists in storage");
        Ok(fs::exists(
            self.base_path.join(self.base_path.join(filename)),
        )?)
    }

    pub fn file_count(&self) -> Result<usize> {
        Ok(fs::read_dir(&self.base_path)?.count())
    }
}
