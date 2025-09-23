use crate::errors::{BbsError, BbsResult};
use crate::users::{RegistrationRequest, User};

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Storage backend trait - allows easy swapping between file and database storage
pub trait UserStorage {
    fn load_user(&self, username: &str) -> BbsResult<Option<User>>;
    fn save_user(&mut self, user: &User) -> BbsResult<()>;
    fn user_exists(&self, username: &str) -> BbsResult<bool>;
    // fn list_users(&self) -> BbsResult<Vec<String>>;
    // fn get_user_count(&self) -> BbsResult<usize>;
}

/// JSON file-based user storage implementation
pub struct JsonUserStorage {
    //data_dir: PathBuf,
    users_file: PathBuf,
    users_cache: HashMap<String, User>,
}

impl JsonUserStorage {
    /// Create a new JSON storage backend with the specified data directory
    pub fn new<P: AsRef<Path>>(data_dir: P) -> BbsResult<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        let users_file = data_dir.join("users.json");

        // Create data directory if it doesn't exist
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir).map_err(|e| {
                BbsError::Configuration(format!("Failed to create data directory: {}", e))
            })?;
        }

        let mut storage = Self {
            // data_dir,
            users_file,
            users_cache: HashMap::new(),
        };

        // Load existing users into cache
        storage.load_all_users()?;

        Ok(storage)
    }

    /// Load all users from the JSON file into the cache
    fn load_all_users(&mut self) -> BbsResult<()> {
        if !self.users_file.exists() {
            // Create empty users file
            let empty_users: HashMap<String, User> = HashMap::new();
            self.save_all_users(&empty_users)?;
            return Ok(());
        }

        let content = fs::read_to_string(&self.users_file)
            .map_err(|e| BbsError::Configuration(format!("Failed to read users file: {}", e)))?;

        if content.trim().is_empty() {
            return Ok(()); // Empty file is OK
        }

        let users: HashMap<String, User> = serde_json::from_str(&content)
            .map_err(|e| BbsError::Configuration(format!("Failed to parse users file: {}", e)))?;

        self.users_cache = users;
        Ok(())
    }

    /// Save all users from cache to the JSON file
    fn save_all_users(&self, users: &HashMap<String, User>) -> BbsResult<()> {
        let content = serde_json::to_string_pretty(users)
            .map_err(|e| BbsError::Configuration(format!("Failed to serialize users: {}", e)))?;

        fs::write(&self.users_file, content)
            .map_err(|e| BbsError::Configuration(format!("Failed to write users file: {}", e)))?;

        Ok(())
    }

    /// Register a new user
    pub fn register_user(
        &mut self,
        request: &RegistrationRequest,
        config: &crate::config::BbsConfig,
    ) -> BbsResult<User> {
        // Validate the request
        request.validate(config)?;

        // Check if user already exists
        if self.user_exists(&request.username)? {
            return Err(BbsError::InvalidInput(format!(
                "Username '{}' is already taken",
                request.username
            )));
        }

        // Create new user
        let user = User::new(
            request.username.clone(),
            request.email.clone(),
            &request.password,
        )?;

        // Save user
        self.save_user(&user)?;

        Ok(user)
    }

    /// Authenticate a user with username and password
    pub fn authenticate_user(&mut self, username: &str, password: &str) -> BbsResult<Option<User>> {
        if let Some(mut user) = self.load_user(username)? {
            if !user.is_active() {
                return Err(BbsError::AuthenticationFailed(
                    "Account is disabled".to_string(),
                ));
            }

            if user.verify_password(password)? {
                // Update login information
                user.record_login();
                self.save_user(&user)?;
                return Ok(Some(user));
            }
        }

        Ok(None) // User not found or password incorrect
    }

    // Get statistics about users
    // pub fn get_stats(&self) -> UserStats {
    //     let total_users = self.users_cache.len();
    //     let active_users = self.users_cache.values().filter(|u| u.is_active()).count();
    //
    //     UserStats {
    //         total_users,
    //         active_users,
    //         inactive_users: total_users - active_users,
    //     }
    // }
}

impl UserStorage for JsonUserStorage {
    fn load_user(&self, username: &str) -> BbsResult<Option<User>> {
        // TODO: why clone?
        Ok(self.users_cache.get(username).cloned())
    }

    fn save_user(&mut self, user: &User) -> BbsResult<()> {
        // Update cache
        self.users_cache.insert(user.username.clone(), user.clone());

        // Save to file
        self.save_all_users(&self.users_cache)?;

        Ok(())
    }

    fn user_exists(&self, username: &str) -> BbsResult<bool> {
        Ok(self.users_cache.contains_key(username))
    }

    // fn list_users(&self) -> BbsResult<Vec<String>> {
    //     let mut usernames: Vec<String> = self.users_cache.keys().cloned().collect();
    //     usernames.sort();
    //     Ok(usernames)
    // }
    //
    // fn get_user_count(&self) -> BbsResult<usize> {
    //     Ok(self.users_cache.len())
    // }
}

// User statistics
// #[derive(Debug)]
// pub struct UserStats {
//     pub total_users: usize,
//     pub active_users: usize,
//     pub inactive_users: usize,
// }
//
// Future database storage implementation would look like:
// ```rust
// pub struct DatabaseUserStorage {
//     connection: Database,
// }
//
// impl UserStorage for DatabaseUserStorage {
//     fn load_user(&self, username: &str) -> BbsResult<Option<User>> {
//         // SELECT * FROM users WHERE username = ?
//     }
//     // ... other methods
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BbsConfig;
    use tempfile::TempDir;

    fn create_test_storage() -> (JsonUserStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = JsonUserStorage::new(temp_dir.path()).unwrap();
        (storage, temp_dir)
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

        let request =
            RegistrationRequest::new("testuser".to_string(), None, "password123".to_string());

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

        let request =
            RegistrationRequest::new("testuser".to_string(), None, "password123".to_string());

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
    fn test_persistence() {
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
}
