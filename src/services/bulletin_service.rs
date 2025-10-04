use crate::bulletin_repository::{BulletinStats, BulletinStorage};
use crate::bulletins::{Bulletin, BulletinRequest};
use crate::config::BbsConfig;
use crate::errors::{BbsError, BbsResult};
use std::sync::{Arc, Mutex};

pub struct BulletinService {
    storage: Arc<Mutex<dyn BulletinStorage + Send>>,
}

impl BulletinService {
    pub fn new(storage: Arc<Mutex<dyn BulletinStorage + Send>>) -> Self {
        Self { storage }
    }

    pub fn post_bulletin(&self, request: BulletinRequest, config: &BbsConfig) -> BbsResult<u32> {
        let mut storage = self
            .storage
            .lock()
            .map_err(|_| BbsError::Configuration("Storage lock poisoned".to_string()))?;
        storage.post_bulletin(&request, config)
    }

    pub fn get_bulletin(&self, id: u32) -> BbsResult<Option<Bulletin>> {
        let storage = self
            .storage
            .lock()
            .map_err(|_| BbsError::Configuration("Storage lock poisoned".to_string()))?;
        storage.load_bulletin(id)
    }

    pub fn mark_read(&self, bulletin_id: u32, username: &str) -> BbsResult<()> {
        let mut storage = self
            .storage
            .lock()
            .map_err(|_| BbsError::Configuration("Storage lock poisoned".to_string()))?;
        storage.mark_read(bulletin_id, username)
    }

    pub fn get_stats(&self, current_user: Option<&str>) -> BbsResult<BulletinStats> {
        let storage = self
            .storage
            .lock()
            .map_err(|_| BbsError::Configuration("Storage lock poisoned".to_string()))?;
        Ok(storage.get_stats(current_user))
    }
}
