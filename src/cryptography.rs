use anyhow::{Result, bail};
use blake3::Hasher;
use chacha20poly1305::{
    AeadCore, KeyInit,
    aead::{Aead, OsRng, generic_array::typenum::Unsigned},
};
use hex::ToHex;
use rand::distr::{Alphanumeric, SampleString};
use std::{fs, path::PathBuf};

type CryptoImpl = chacha20poly1305::ChaCha20Poly1305;
type CryptoNonce = chacha20poly1305::Nonce;
type CryptoNonceSize = <CryptoImpl as AeadCore>::NonceSize;

#[derive(Debug)]
pub struct Cryptography;

impl Cryptography {
    /// Encrypt a byte array using a random key & nonce.
    ///
    /// Upon success the decryption key and the encrypted bytes are provided.
    pub fn encrypt(bytes: &[u8]) -> Result<(String, Vec<u8>)> {
        let key = CryptoImpl::generate_key(&mut OsRng);
        let nonce = CryptoImpl::generate_nonce(&mut OsRng);
        let cipher = CryptoImpl::new(&key);
        let mut ciphered_bytes = match cipher.encrypt(&nonce, bytes) {
            Ok(b) => b,
            Err(err) => {
                bail!("{err:?}");
            }
        };
        ciphered_bytes.splice(..0, nonce.iter().copied());
        Ok((key.encode_hex_upper(), ciphered_bytes))
    }

    /// Decrypt a byte array with its decryption key.
    ///
    /// # Notes
    /// Should only be used on values encrypted by [`Cryptography::encrypt`].
    pub fn decrypt(bytes: &[u8], key: &str) -> Result<Vec<u8>> {
        let (nonce, encrypted_bytes) = bytes.split_at(CryptoNonceSize::to_usize());
        let key = hex::decode(key)?;
        let cipher = CryptoImpl::new_from_slice(&key)?;
        match cipher.decrypt(CryptoNonce::from_slice(nonce), encrypted_bytes) {
            Ok(data) => Ok(data),
            Err(err) => bail!(err),
        }
    }

    /// Hash a byte array and add the provided salt.
    ///
    /// Will automatically use multiple threads when the provided
    /// byte array is beyond a certain length.
    pub fn hash_from_bytes(bytes: &[u8], salt: &str) -> Result<String> {
        let mut hasher = Hasher::new();

        // 100 MB
        if bytes.len() < 0x100000 {
            hasher.update_rayon(bytes);
        } else {
            hasher.update(bytes);
        }

        hasher.update(salt.as_bytes());
        Ok(hasher.finalize().to_hex().to_string())
    }

    /// Retrieve a saved salt string from the given path.
    pub fn get_persisted_salt(path: &PathBuf) -> Result<Option<String>> {
        // Check for existing salt.
        if fs::exists(path)? {
            let data = fs::read_to_string(path)?;
            if !data.trim().is_empty() {
                return Ok(Some(data));
            }
        }
        Ok(None)
    }

    // Generate and save a persisted salt value at the given path.
    pub fn create_persisted_salt(path: &PathBuf) -> Result<String> {
        let salt = Alphanumeric.sample_string(&mut rand::rng(), 64);
        fs::write(path, &salt)?;
        Ok(salt)
    }
}
