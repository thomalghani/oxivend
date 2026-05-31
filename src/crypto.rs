//! Ed25519 cryptographic utilities for license signing and verification.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
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
