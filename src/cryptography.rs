use anyhow::{Result, bail};
use base64ct::Encoding;
use blake3::Hasher;
use chacha20poly1305::{
    AeadCore, KeyInit,
    aead::{Aead, OsRng, generic_array::typenum::Unsigned},
};
use rand::distr::{Alphanumeric, SampleString};
use std::{fs, path::PathBuf};

type CryptoImpl = chacha20poly1305::ChaCha20Poly1305;
type CryptoPayload<'a> = chacha20poly1305::aead::Payload<'a, 'a>;
type CryptoNonce = chacha20poly1305::Nonce;
const CRYPTO_NONCE_SIZE: usize = <CryptoImpl as AeadCore>::NonceSize::USIZE;

#[derive(Debug)]
pub struct Cryptography;

impl Cryptography {
    /// Encrypt a byte array using a random key & nonce.
    ///
    /// Upon success the decryption key and the encrypted bytes are provided.
    pub fn encrypt(bytes: &[u8], aad: &[u8]) -> Result<(String, Vec<u8>)> {
        let key = CryptoImpl::generate_key(&mut OsRng);
        let nonce = CryptoImpl::generate_nonce(&mut OsRng);
        let cipher = CryptoImpl::new(&key);
        let mut ciphered_bytes = match cipher.encrypt(&nonce, CryptoPayload { msg: bytes, aad }) {
            Ok(b) => b,
            Err(err) => {
                bail!("{err:?}");
            }
        };
        ciphered_bytes.splice(..0, nonce.iter().copied());
        Ok((
            base64ct::Base64UrlUnpadded::encode_string(&key),
            ciphered_bytes,
        ))
    }

    /// Decrypt a byte array with its decryption key.
    ///
    /// # Notes
    /// Should only be used on values encrypted by [`Cryptography::encrypt`].
    pub fn decrypt(bytes: &[u8], key: &str, aad: &[u8]) -> Result<Vec<u8>> {
        let (nonce, encrypted_bytes) = bytes.split_at(CRYPTO_NONCE_SIZE);
        let key = base64ct::Base64UrlUnpadded::decode_vec(key)?;
        let cipher = CryptoImpl::new_from_slice(&key)?;
        match cipher.decrypt(
            CryptoNonce::from_slice(nonce),
            CryptoPayload {
                msg: encrypted_bytes,
                aad,
            },
        ) {
            Ok(data) => Ok(data),
            Err(err) => bail!(err),
        }
    }

    /// Hash a byte array and add the provided salt.
    ///
    /// Will automatically use multiple threads when the provided
    /// byte array is beyond a certain length.
    pub fn hash_bytes(bytes: &[u8], salt: &str) -> Result<String> {
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
