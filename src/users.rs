use crate::errors::{BbsError, BbsResult};

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// User account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    // TODO: Id number
    pub username: String,
    pub email: Option<String>,
    pub password_hash: String,
    pub created_at: Timestamp,
    pub last_login: Timestamp,
    pub login_count: u32,
    pub is_active: bool,
}

impl User {
    /// Create a new user with the given username and password
    pub fn new(username: String, email: Option<String>, password: &str) -> BbsResult<Self> {
        let password_hash = PasswordHasher::hash_password(password)?;
        let now = Timestamp::now();

        Ok(User {
            username,
            email,
            password_hash,
            created_at: now,
            last_login: now,
            login_count: 0,
            is_active: true,
        })
    }

    // TODO: investigate secure string implementation like secrecy or zeroize
    //       for passing around secrets like passwords
    /// Verify a password against this user's stored hash
    pub fn verify_password(&self, password: &str) -> BbsResult<bool> {
        PasswordHasher::verify_password(password, &self.password_hash)
    }

    /// Update the last login time and increment login count
    pub fn record_login(&mut self) {
        self.last_login = Timestamp::now();
        self.login_count += 1;
    }

    /// Check if the user account is active
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    // Get a display-friendly string for when the user was created
    // pub fn created_display(&self) -> String {
    //     // Format timestamp for display
    //     format!("{}", self.created_at.strftime("%Y-%m-%d"))
    // }

    /// Get a display-friendly string for last login time
    pub fn last_login_display(&self) -> String {
        let now = Timestamp::now();
        let duration_since = now.duration_since(self.last_login);

        // Simple relative time display
        let seconds = duration_since.as_secs();
        if seconds < 60 {
            "just now".to_string()
        } else if seconds < 3600 {
            format!("{} minutes ago", seconds / 60)
        } else if seconds < 86400 {
            format!("{} hours ago", seconds / 3600)
        } else {
            format!("{} days ago", seconds / 86400)
        }
    }
}

/// Registration request data
#[derive(Debug)]
pub struct RegistrationRequest {
    pub username: String,
    pub email: Option<String>,
    pub password: String,
}

impl RegistrationRequest {
    pub fn new(username: String, email: Option<String>, password: String) -> Self {
        Self {
            username,
            email,
            password,
        }
    }

    /// Validate the registration request
    pub fn validate(&self, config: &crate::config::BbsConfig) -> BbsResult<()> {
        // Validate username
        if self.username.is_empty() {
            return Err(BbsError::InvalidInput(
                "Username cannot be empty".to_string(),
            ));
        }

        if self.username.len() > config.features.max_username_length {
            return Err(BbsError::InvalidInput(format!(
                "Username too long (max {} characters)",
                config.features.max_username_length
            )));
        }

        // Check for valid characters (alphanumeric and underscore only)
        if !self
            .username
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_')
        {
            return Err(BbsError::InvalidInput(
                "Username can only contain letters, numbers, and underscores".to_string(),
            ));
        }

        // Validate password
        if self.password.is_empty() {
            return Err(BbsError::InvalidInput(
                "Password cannot be empty".to_string(),
            ));
        }

        if self.password.len() < 4 {
            return Err(BbsError::InvalidInput(
                "Password must be at least 4 characters".to_string(),
            ));
        }

        // Validate email if provided
        if let Some(ref email) = self.email
            && !email.is_empty()
            && !email.contains('@')
        {
            return Err(BbsError::InvalidInput("Invalid email address".to_string()));
        }

        Ok(())
    }
}

/// Password hashing trait - allows easy swapping of hash algorithms
pub trait PasswordHash {
    fn hash_password(password: &str) -> BbsResult<String>;
    fn verify_password(password: &str, hash: &str) -> BbsResult<bool>;
}

/// Simple password hasher
/// This can be easily replaced with bcrypt or others later
pub struct PasswordHasher;

impl PasswordHash for PasswordHasher {
    fn hash_password(password: &str) -> BbsResult<String> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Generate a simple salt based on current timestamp
        let salt = Timestamp::now().as_second();
        let salted_password = format!("{}{}", password, salt);

        // TODO: add a proper crypto package to handle this
        // Use DefaultHasher for simplicity (not cryptographically secure, but good for demo)
        // In production, you'd use a proper crypto hash like SHA-256
        let mut hasher = DefaultHasher::new();
        salted_password.hash(&mut hasher);
        let hash = hasher.finish();

        // Store hash and salt together
        Ok(format!("{}:{}", hash, salt))
    }

    fn verify_password(password: &str, stored_hash: &str) -> BbsResult<bool> {
        // Split stored hash into hash and salt
        let parts: Vec<&str> = stored_hash.split(':').collect();
        if parts.len() != 2 {
            return Err(BbsError::AuthenticationFailed(
                "Invalid hash format".to_string(),
            ));
        }

        let stored_hash_value = parts[0];
        let salt = parts[1];

        // Hash the provided password with the same salt
        let salted_password = format!("{}{}", password, salt);
        let mut hasher = DefaultHasher::new();
        salted_password.hash(&mut hasher);
        let computed_hash = hasher.finish().to_string();

        Ok(computed_hash == stored_hash_value)
    }
}

// TODO:
// Future bcrypt implementation would look like:
// pub struct BcryptHasher;
// impl PasswordHash for BcryptHasher {
//     fn hash_password(password: &str) -> BbsResult<String> {
//         bcrypt::hash(password, bcrypt::DEFAULT_COST)
//             .map_err(|e| BbsError::AuthenticationFailed(e.to_string()))
//     }
//
//     fn verify_password(password: &str, hash: &str) -> BbsResult<bool> {
//         bcrypt::verify(password, hash)
//             .map_err(|e| BbsError::AuthenticationFailed(e.to_string()))
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BbsConfig;

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
        let invalid_req =
            RegistrationRequest::new("validuser".to_string(), None, "123".to_string());
        assert!(invalid_req.validate(&config).is_err());
    }
}
