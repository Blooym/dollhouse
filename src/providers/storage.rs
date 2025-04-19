use crate::cryptography::Cryptography;
use anyhow::Result;
use std::{
    fs::{self, File, FileTimes},
    io::Read,
    path::PathBuf,
    time::{Duration, SystemTime},
};
use tracing::{debug, info};

#[derive(Debug)]
pub struct StorageProvider {
    base_path: PathBuf,
}

impl StorageProvider {
    pub fn new(base_path: PathBuf) -> Result<Self> {
        fs::create_dir_all(&base_path)?;
        Ok(Self { base_path })
    }

    fn is_file_expired(&self, filename: &str, expire_after: Duration) -> Result<bool> {
        let metadata = fs::metadata(self.base_path.join(filename))?;
        let last_access = metadata.accessed()?;
        Ok(last_access + expire_after <= SystemTime::now())
    }

    pub fn remove_all_expired_files(&self, expire_after: Duration) -> Result<()> {
        fs::read_dir(&self.base_path)
            .unwrap()
            .filter_map(|f| f.ok())
            .for_each(|file| {
                let Ok(file_name) = file.file_name().into_string() else {
                    return;
                };
                if self.is_file_expired(&file_name, expire_after).unwrap() {
                    info!("file '{}' expired - deleting from storage.", file_name);
                    self.delete_file(&file_name).unwrap();
                }
            });
        Ok(())
    }

    pub fn get_file(&self, filename: &str, key: &str) -> Result<Vec<u8>> {
        debug!("Decrypting and fetching {filename} from storage");
        let file_path = self.base_path.join(filename);

        // Update access time.
        let metadata = fs::metadata(&file_path)?;
        let mut file = File::options().read(true).write(true).open(&file_path)?;
        let _ = file.set_times(
            FileTimes::new()
                .set_accessed(SystemTime::now())
                .set_modified(metadata.modified()?),
        );

        // Read and decrypt.
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Cryptography::decrypt(&buf, key, filename.as_bytes())
    }

    pub fn save_file(&self, filename: &str, bytes: &[u8]) -> Result<String> {
        debug!("Encrypting and saving {filename} to storage");
        let (key, bytes) = Cryptography::encrypt(bytes, filename.as_bytes())?;
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
