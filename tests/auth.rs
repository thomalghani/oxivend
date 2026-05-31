//! Unit tests for authentication functions.

use oxivend::auth::{create_token, hash_password, validate_token, verify_password};
use uuid::Uuid;

#[test]
fn test_password_hash_and_verify_roundtrip() {
    let password = "my-secure-password-123!";
    let hash = hash_password(password).expect("hashing should succeed");
    assert!(verify_password(password, &hash).expect("verification should succeed"));
}

#[test]
fn test_password_verify_wrong_password() {
    let hash = hash_password("correct-password").expect("hashing should succeed");
    let result = verify_password("wrong-password", &hash).expect("verify should not error");
    assert!(!result, "wrong password should not match");
}

#[test]
fn test_password_verify_invalid_hash() {
    let result = verify_password("password", "not-a-valid-hash");
    assert!(
        result.is_err(),
        "verifying against invalid hash should error"
    );
}

#[test]
fn test_token_create_and_validate_roundtrip() {
    let user_id = Uuid::new_v4();
    let secret = "my-secret-key";

    let token = create_token(user_id, secret).expect("token creation should succeed");
    let claims = validate_token(&token, secret).expect("token validation should succeed");

    assert_eq!(claims.sub, user_id.to_string());
    assert!(claims.iat > 0);
    assert!(claims.exp > claims.iat);
}

#[test]
fn test_token_validate_wrong_secret() {
    let user_id = Uuid::new_v4();
    let token = create_token(user_id, "correct-secret").expect("token creation should succeed");
    let result = validate_token(&token, "wrong-secret");
    assert!(result.is_err(), "validation with wrong secret should fail");
}

#[test]
fn test_token_validate_malformed() {
    let result = validate_token("not-a-jwt-token", "secret");
    assert!(result.is_err(), "malformed token should fail validation");
}

#[test]
fn test_token_validate_empty() {
    let result = validate_token("", "secret");
    assert!(result.is_err(), "empty token should fail validation");
}
