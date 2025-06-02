use crate::storage::{StorageCapabilities, StorageOperations};
use anyhow::{Context, Result, anyhow, bail};
use aws_sdk_s3::{Client, primitives::ByteStream};
use std::path::PathBuf;
use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub struct S3Storage {
    client: Client,
    bucket: String,
}

impl S3Storage {
    pub fn new(bucket: String) -> Result<Self> {
        let bucket_clone = bucket.clone();
        let client = match std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let config = aws_config::from_env().load().await;
                let client = Client::new(&config);
                if let Err(err) = client.head_bucket().bucket(&bucket_clone).send().await {
                    if err.as_service_error().map(|e| e.is_not_found()) == Some(true) {
                        client
                            .create_bucket()
                            .bucket(&bucket_clone)
                            .send()
                            .await
                            .unwrap();
                    } else {
                        bail!("Error while initialing S3 bucket for storage: {err:?}");
                    }
                }
                debug!(
                    "Initialised S3 client with endpoint {:?}",
                    config.endpoint_url()
                );
                Ok(client)
            })
        })
        .join()
        {
            Ok(result) => result,
            Err(panic_err) => {
                return Err(anyhow!("S3 client creation thread error: {:?}", panic_err));
            }
        }?;

        Ok(Self { client, bucket })
    }
}

impl StorageCapabilities for S3Storage {
    fn supports_expiry(&self) -> bool {
        false
    }
}

impl StorageOperations for S3Storage {
    async fn read(&self, path: &std::path::Path) -> Result<Option<Vec<u8>>> {
        debug!("Reading {path:?} from bucket {}", self.bucket);
        match self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(path.to_str().context("failed to convert path to str")?)
            .send()
            .await
        {
            Ok(output) => {
                let data = output.body.collect().await?.into_bytes().to_vec();
                Ok(Some(data))
            }
            Err(err) => {
                if err.as_service_error().map(|e| e.is_no_such_key()) == Some(true) {
                    Ok(None)
                } else {
                    Err(err.into())
                }
            }
        }
    }

    async fn write(&mut self, path: &std::path::Path, data: &[u8]) -> Result<()> {
        debug!("Writing {path:?} to bucket {}", self.bucket);
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(path.to_str().context("failed to convert path to str")?)
            .body(ByteStream::from(data.to_vec()))
            .send()
            .await?;
        Ok(())
    }

    async fn delete(&mut self, path: &std::path::Path) -> Result<bool> {
        debug!("Deleting {path:?} from bucket {}", self.bucket);
        if !self.exists(path).await? {
            return Ok(false);
        }
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(path.to_str().context("failed to convert path to str")?)
            .send()
            .await?;
        Ok(true)
    }

    async fn exists(&self, path: &std::path::Path) -> Result<bool> {
        debug!("Checking if {path:?} exists in bucket {}", self.bucket);
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(path.to_str().context("failed to convert path to str")?)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(err) => {
                if err.as_service_error().map(|e| e.is_not_found()) == Some(true) {
                    Ok(false)
                } else {
                    Err(err.into())
                }
            }
        }
    }

    async fn list(&self, path: &std::path::Path) -> Result<Vec<std::path::PathBuf>> {
        // FIXME: This needs work as it's highly unoptimal if storing a large amount of files
        // as it will only return up to 1000.
        debug!("Listing files inside of {path:?} in bucket {}", self.bucket);
        let output = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(path.to_str().context("failed to convert path to str")?)
            .send()
            .await?;
        let mut paths = Vec::new();
        if let Some(objects) = output.contents {
            for object in objects {
                if let Some(key) = object.key {
                    paths.push(PathBuf::from(key));
                }
            }
        }
        Ok(paths)
    }

    async fn last_access(&self, _path: &std::path::Path) -> Result<Option<std::time::SystemTime>> {
        // Use Lifecycle Configuration instead
        warn!("last_access is an unsupported operation that will always return Err");
        bail!("Unsupported operation");

        // // S3 doesn't track access times, so this falls back to last modified instead.
        // match self
        //     .client
        //     .head_object()
        //     .bucket(&self.bucket)
        //     .key(path.to_str().context("failed to convert path to str")?)
        //     .send()
        //     .await
        // {
        //     Ok(output) => {
        //         if let Some(last_modified) = output.last_modified {
        //             Ok(Some(
        //                 UNIX_EPOCH + Duration::from_secs(last_modified.secs() as u64),
        //             ))
        //         } else {
        //             Ok(None)
        //         }
        //     }
        //     Err(err) => {
        //         if err.as_service_error().map(|e| e.is_not_found()) == Some(true) {
        //             Ok(None)
        //         } else {
        //             Err(err.into())
        //         }
        //     }
        // }
    }
}
