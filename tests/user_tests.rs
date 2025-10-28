mod common;

use moonbase::config::BbsConfig;
use moonbase::user_repository::{JsonUserStorage, UserStorage};
use moonbase::users::{RegistrationRequest, User};
use tempfile::TempDir;

fn create_test_storage() -> (JsonUserStorage, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let storage = JsonUserStorage::new(temp_dir.path()).unwrap();
    (storage, temp_dir)
}

#[test]
fn test_user_creation() {
    let user = User::new(
        "testuser".to_string(),
        Some("test@example.com".to_string()),
        "password123",
    )
    .unwrap();

    assert_eq!(user.username, "testuser");
    assert_eq!(user.email, Some("test@example.com".to_string()));
    assert!(user.is_active());
    assert_eq!(user.login_count, 0);
}

#[test]
fn test_password_verification() {
    let user = User::new("testuser".to_string(), None, "password123").unwrap();

    assert!(user.verify_password("password123").unwrap());
    assert!(!user.verify_password("wrongpassword").unwrap());
}

#[test]
fn test_registration_validation() {
    let config = BbsConfig::default();

    // Valid registration
    let valid_req = RegistrationRequest::new(
        "validuser".to_string(),
        Some("valid@email.com".to_string()),
        "password123".to_string(),
    );
    assert!(valid_req.validate(&config).is_ok());

    // Empty username
    let invalid_req = RegistrationRequest::new("".to_string(), None, "password123".to_string());
    assert!(invalid_req.validate(&config).is_err());

    // Short password
    let invalid_req = RegistrationRequest::new("validuser".to_string(), None, "123".to_string());
    assert!(invalid_req.validate(&config).is_err());
}

#[test]
fn test_user_registration() {
    let (mut storage, _temp_dir) = create_test_storage();
    let config = BbsConfig::default();

    let request = RegistrationRequest::new(
        "testuser".to_string(),
        Some("test@example.com".to_string()),
        "password123".to_string(),
    );

    let user = storage.register_user(&request, &config).unwrap();
    assert_eq!(user.username, "testuser");
    assert!(storage.user_exists("testuser").unwrap());
}

#[test]
fn test_duplicate_username() {
    let (mut storage, _temp_dir) = create_test_storage();
    let config = BbsConfig::default();

    let request = RegistrationRequest::new("testuser".to_string(), None, "password123".to_string());

    // First registration should succeed
    storage.register_user(&request, &config).unwrap();

    // Second registration with same username should fail
    let result = storage.register_user(&request, &config);
    assert!(result.is_err());
}

#[test]
fn test_user_authentication() {
    let (mut storage, _temp_dir) = create_test_storage();
    let config = BbsConfig::default();

    let request = RegistrationRequest::new("testuser".to_string(), None, "password123".to_string());

    storage.register_user(&request, &config).unwrap();

    // Valid authentication
    let auth_result = storage
        .authenticate_user("testuser", "password123")
        .unwrap();
    assert!(auth_result.is_some());

    // Invalid password
    let auth_result = storage
        .authenticate_user("testuser", "wrongpassword")
        .unwrap();
    assert!(auth_result.is_none());

    // Non-existent user
    let auth_result = storage
        .authenticate_user("nonexistent", "password123")
        .unwrap();
    assert!(auth_result.is_none());
}

#[test]
fn test_user_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let config = BbsConfig::default();

    // Create storage and add user
    {
        let mut storage = JsonUserStorage::new(temp_dir.path()).unwrap();
        let request =
            RegistrationRequest::new("testuser".to_string(), None, "password123".to_string());
        storage.register_user(&request, &config).unwrap();
    }

    // Create new storage instance and verify user exists
    {
        let storage = JsonUserStorage::new(temp_dir.path()).unwrap();
        assert!(storage.user_exists("testuser").unwrap());
    }
}
