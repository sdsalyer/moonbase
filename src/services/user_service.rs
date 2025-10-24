use crate::errors::{BbsError, BbsResult};
use crate::menu::UserStats;
use crate::user_repository::UserStorage;
use crate::users::{RegistrationRequest, User};
use std::sync::{Arc, Mutex};

pub struct UserService {
    storage: Arc<Mutex<dyn UserStorage + Send>>,
}

impl UserService {
    pub fn new(storage: Arc<Mutex<dyn UserStorage + Send>>) -> Self {
        Self { storage }
    }

    pub fn authenticate(&self, username: &str, password: &str) -> BbsResult<Option<User>> {
        let mut storage = self
            .storage
            .lock()
            .map_err(|_| BbsError::Configuration("Storage lock poisoned".to_string()))?;
        storage.authenticate_user(username, password)
    }

    pub fn register(
        &self,
        request: RegistrationRequest,
        config: &crate::config::BbsConfig,
    ) -> BbsResult<User> {
        let mut storage = self
            .storage
            .lock()
            .map_err(|_| BbsError::Configuration("Storage lock poisoned".to_string()))?;
        storage.register_user(&request, config)
    }

    // pub fn get_user(&self, username: &str) -> BbsResult<Option<User>> {
    //     let storage = self.storage.lock()
    //         .map_err(|_| BbsError::Configuration("Storage lock poisoned".to_string()))?;
    //     storage.load_user(username)
    // }

    pub fn get_stats(&self) -> BbsResult<UserStats> {
        let storage = self
            .storage
            .lock()
            .map_err(|_| BbsError::Configuration("Storage lock poisoned".to_string()))?;
        storage.get_stats()
    }
}
