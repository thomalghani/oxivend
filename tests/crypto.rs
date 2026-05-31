//! Unit tests for cryptographic operations.

use chrono::Utc;
use oxivend::crypto::{LicensePayload, generate_keypair, public_key, sign_license, verify_license};
use uuid::Uuid;

#[test]
fn test_keypair_generation() {
    let key = generate_keypair();
    let verifying_key = public_key(&key);
    assert_eq!(key.verifying_key(), verifying_key);
}

#[test]
fn test_sign_and_verify_roundtrip() {
    let key = generate_keypair();
    let verifying_key = public_key(&key);

    let payload = LicensePayload {
        format_version: 1,
        algorithm: "ed25519".into(),
        product_id: Uuid::new_v4(),
        user_email: "user@example.com".into(),
        issued_at: Utc::now(),
        expires_at: None,
        license_id: Uuid::new_v4(),
    };

    let token = sign_license(&key, &payload).expect("signing should succeed");
    let decoded = verify_license(&verifying_key, &token).expect("verification should succeed");

    assert_eq!(decoded.format_version, payload.format_version);
    assert_eq!(decoded.user_email, payload.user_email);
    assert_eq!(decoded.product_id, payload.product_id);
    assert_eq!(decoded.license_id, payload.license_id);
}

#[test]
fn test_verify_with_wrong_key() {
    let key = generate_keypair();
    let wrong_key = public_key(&generate_keypair());

    let payload = LicensePayload {
        format_version: 1,
        algorithm: "ed25519".into(),
        product_id: Uuid::new_v4(),
        user_email: "user@example.com".into(),
        issued_at: Utc::now(),
        expires_at: None,
        license_id: Uuid::new_v4(),
    };

    let token = sign_license(&key, &payload).expect("signing should succeed");
    let result = verify_license(&wrong_key, &token);

    assert!(result.is_err(), "verification with wrong key should fail");
}

#[test]
fn test_verify_malformed_token() {
    let key = public_key(&generate_keypair());

    assert!(verify_license(&key, "not-a-valid-token").is_err());
    assert!(verify_license(&key, "only-one-part").is_err());
    assert!(verify_license(&key, "too.many.dots").is_err());
}

#[test]
fn test_verify_tampered_payload() {
    let key = generate_keypair();
    let verifying_key = public_key(&key);

    let payload = LicensePayload {
        format_version: 1,
        algorithm: "ed25519".into(),
        product_id: Uuid::new_v4(),
        user_email: "user@example.com".into(),
        issued_at: Utc::now(),
        expires_at: None,
        license_id: Uuid::new_v4(),
    };

    let token = sign_license(&key, &payload).expect("signing should succeed");

    // Tamper by swapping the payload with a different one
    let different_payload = LicensePayload {
        user_email: "other@example.com".into(),
        ..payload
    };
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as BASE64;
    let tampered_payload = BASE64.encode(serde_json::to_vec(&different_payload).unwrap());
    let parts: Vec<&str> = token.splitn(2, '.').collect();
    let tampered_token = format!("{}.{}", tampered_payload, parts[1]);

    let result = verify_license(&verifying_key, &tampered_token);
    assert!(
        result.is_err(),
        "verification of tampered payload should fail"
    );
}

#[test]
fn test_empty_token() {
    let key = public_key(&generate_keypair());
    assert!(verify_license(&key, "").is_err());
}
