//! Ed25519 cryptographic utilities for license signing and verification.
//! Also provides AES-256-GCM private key encryption.

use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit},
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::RngCore;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::errors::CryptoError;

/// Versioned license payload. The `format_version` field enables forward
/// compatibility when the payload format or signing algorithm changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicensePayload {
    pub format_version: u8,
    pub algorithm: String,
    pub product_id: Uuid,
    pub user_email: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub license_id: Uuid,
}

/// Generate a new Ed25519 key pair.
pub fn generate_keypair() -> SigningKey {
    SigningKey::generate(&mut OsRng)
}

/// Get the verifying key from a signing key.
pub fn public_key(signing_key: &SigningKey) -> VerifyingKey {
    signing_key.verifying_key()
}

/// Sign a license payload and return a signed token in the format:
/// `base64(payload).base64(signature)`
pub fn sign_license(key: &SigningKey, payload: &LicensePayload) -> Result<String, CryptoError> {
    let payload_json =
        serde_json::to_vec(payload).map_err(|e| CryptoError::InvalidKey(e.to_string()))?;
    let signature = key.sign(&payload_json);
    let encoded_payload = BASE64.encode(&payload_json);
    let encoded_sig = BASE64.encode(signature.to_bytes());
    Ok(format!("{encoded_payload}.{encoded_sig}"))
}

/// Verify a signed license token and return the decoded payload.
pub fn verify_license(key: &VerifyingKey, token: &str) -> Result<LicensePayload, CryptoError> {
    let parts: Vec<&str> = token.splitn(2, '.').collect();
    if parts.len() != 2 {
        tracing::warn!(
            "Malformed license token: expected 2 parts, got {}",
            parts.len()
        );
        return Err(CryptoError::VerificationFailed);
    }

    let payload_bytes = BASE64.decode(parts[0]).map_err(|e| {
        tracing::warn!(error = %e, "Failed to base64-decode license payload");
        CryptoError::VerificationFailed
    })?;
    let sig_bytes = BASE64.decode(parts[1]).map_err(|e| {
        tracing::warn!(error = %e, "Failed to base64-decode license signature");
        CryptoError::VerificationFailed
    })?;

    let signature = Signature::from_slice(&sig_bytes).map_err(|e| {
        tracing::warn!(error = %e, "Failed to parse license signature");
        CryptoError::VerificationFailed
    })?;

    key.verify(&payload_bytes, &signature).map_err(|e| {
        tracing::warn!(error = %e, "License signature verification failed");
        CryptoError::VerificationFailed
    })?;

    let payload: LicensePayload = serde_json::from_slice(&payload_bytes).map_err(|e| {
        tracing::warn!(error = %e, "Failed to deserialize license payload");
        CryptoError::InvalidKey("Invalid payload format".into())
    })?;

    Ok(payload)
}

// ─── Private key encryption ───────────────────────────────────────────────────

/// Encrypt an Ed25519 private key using AES-256-GCM.
///
/// The 256-bit key is derived from `encryption_key` via SHA-256.
/// Output format: base64(12-byte-nonce || ciphertext)
#[allow(deprecated)]
pub fn encrypt_private_key(
    private_key: &[u8],
    encryption_key: &str,
) -> Result<String, CryptoError> {
    let key_bytes = Sha256::digest(encryption_key.as_bytes());
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, private_key)
        .map_err(|e| CryptoError::InvalidKey(format!("Encryption failed: {e}")))?;

    let mut out = Vec::with_capacity(12 + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);

    Ok(BASE64.encode(&out))
}

/// Decrypt an Ed25519 private key that was encrypted with [`encrypt_private_key`].
#[allow(deprecated)]
pub fn decrypt_private_key(encrypted: &str, encryption_key: &str) -> Result<Vec<u8>, CryptoError> {
    let data = BASE64
        .decode(encrypted)
        .map_err(|_| CryptoError::InvalidKey("Invalid base64 in encrypted key".into()))?;

    if data.len() < 12 {
        return Err(CryptoError::InvalidKey("Encrypted key too short".into()));
    }

    let (nonce_bytes, ciphertext) = data.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let key_bytes = Sha256::digest(encryption_key.as_bytes());
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    cipher.decrypt(nonce, ciphertext).map_err(|_| {
        CryptoError::InvalidKey("Decryption failed — wrong key or corrupted data".into())
    })
}
