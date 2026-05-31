//! Unit tests for configuration loading.

use oxivend::errors::ConfigError;

#[test]
fn test_config_from_env_with_all_vars() {
    temp_env::with_vars(
        [
            ("DATABASE_URL", Some("postgres://localhost/test")),
            ("OXIVEND_ADMIN_EMAIL", Some("admin@test.com")),
            ("OXIVEND_ADMIN_PASSWORD", Some("password123")),
            ("JWT_SECRET", Some("my-secret")),
            ("DATABASE_POOL_SIZE", Some("5")),
        ],
        || {
            let config = oxivend::config::Config::from_env().expect("should succeed");
            assert_eq!(config.database_url, "postgres://localhost/test");
            assert_eq!(config.admin_email, "admin@test.com");
            assert_eq!(config.admin_password, "password123");
            assert_eq!(config.jwt_secret, "my-secret");
            assert_eq!(config.database_pool_size, 5);
        },
    );
}

#[test]
fn test_config_missing_database_url() {
    temp_env::with_vars(
        [
            ("DATABASE_URL", None),
            ("OXIVEND_ADMIN_EMAIL", Some("admin@test.com")),
            ("OXIVEND_ADMIN_PASSWORD", Some("password123")),
        ],
        || {
            let result = oxivend::config::Config::from_env();
            assert!(result.is_err());
            match result.unwrap_err() {
                ConfigError::MissingEnvVar(name) => {
                    assert_eq!(name, "DATABASE_URL");
                }
                _ => panic!("expected MissingEnvVar"),
            }
        },
    );
}

#[test]
fn test_config_missing_admin_email() {
    temp_env::with_vars(
        [
            ("DATABASE_URL", Some("postgres://localhost/test")),
            ("OXIVEND_ADMIN_EMAIL", None),
            ("OXIVEND_ADMIN_PASSWORD", Some("password123")),
        ],
        || {
            let result = oxivend::config::Config::from_env();
            assert!(result.is_err());
            match result.unwrap_err() {
                ConfigError::MissingEnvVar(name) => {
                    assert_eq!(name, "OXIVEND_ADMIN_EMAIL");
                }
                _ => panic!("expected MissingEnvVar"),
            }
        },
    );
}

#[test]
fn test_config_default_pool_size() {
    temp_env::with_vars(
        [
            ("DATABASE_URL", Some("postgres://localhost/test")),
            ("OXIVEND_ADMIN_EMAIL", Some("admin@test.com")),
            ("OXIVEND_ADMIN_PASSWORD", Some("password123")),
            ("DATABASE_POOL_SIZE", None),
        ],
        || {
            let config = oxivend::config::Config::from_env().expect("should succeed");
            assert_eq!(config.database_pool_size, 10);
        },
    );
}

#[test]
fn test_config_default_jwt_secret() {
    temp_env::with_vars(
        [
            ("DATABASE_URL", Some("postgres://localhost/test")),
            ("OXIVEND_ADMIN_EMAIL", Some("admin@test.com")),
            ("OXIVEND_ADMIN_PASSWORD", Some("password123")),
            ("JWT_SECRET", None),
        ],
        || {
            let config = oxivend::config::Config::from_env().expect("should succeed");
            assert_eq!(config.jwt_secret, "changeme-dev-secret");
        },
    );
}
