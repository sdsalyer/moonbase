use crate::bulletins::{Bulletin, BulletinRequest};
use crate::errors::{BbsError, BbsResult};

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Bulletin statistics for display
#[derive(Debug, Clone, Default)]
pub struct BulletinStats {
    pub total_bulletins: usize,
    pub unread_count: usize,
    pub recent_bulletins: Vec<BulletinSummary>,
}

/// Summary of a bulletin for menu display
#[derive(Debug, Clone)]
pub struct BulletinSummary {
    //pub id: u32,
    pub title: String,
    pub author: String,
    pub posted_display: String,
    pub is_sticky: bool,
    pub is_read: bool,
}

impl From<(&Bulletin, bool)> for BulletinSummary {
    fn from((bulletin, is_read): (&Bulletin, bool)) -> Self {
        Self {
            // id: bulletin.id,
            title: bulletin.title.clone(),
            author: bulletin.author.clone(),
            posted_display: bulletin.posted_display(),
            is_sticky: bulletin.is_sticky,
            is_read,
        }
    }
}

pub trait BulletinStorage {
    fn load_bulletin(&self, id: u32) -> BbsResult<Option<Bulletin>>;
    fn save_bulletin(&mut self, bulletin: &Bulletin) -> BbsResult<()>;
    fn post_bulletin(
        &mut self,
        request: &BulletinRequest,
        config: &crate::config::BbsConfig,
    ) -> BbsResult<u32>;
    fn mark_read(&mut self, bulletin_id: u32, username: &str) -> BbsResult<()>;
    // fn list_bulletins(&self) -> BbsResult<Vec<Bulletin>>;
    fn get_stats(&self, current_user: Option<&str>) -> BulletinStats;
    // fn get_recent_bulletins(&self, limit: usize) -> BbsResult<Vec<Bulletin>>;
    // fn get_unread_bulletins(&self, username: &str) -> BbsResult<Vec<Bulletin>>;
    // fn get_bulletin_count(&self) -> BbsResult<usize>;
    // fn delete_bulletin(&mut self, id: u32) -> BbsResult<bool>;
    // fn set_sticky(&mut self, id: u32, sticky: bool) -> BbsResult<bool>;
}

/// JSON file-based bulletin storage implementation
pub struct JsonBulletinStorage {
    bulletins_file: PathBuf,
    bulletins_cache: HashMap<u32, Bulletin>,
    next_id: u32,
}

impl JsonBulletinStorage {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> BbsResult<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        let bulletins_file = data_dir.join("bulletins.json");

        // Create data directory if it doesn't exist
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir).map_err(|e| {
                BbsError::Configuration(format!("Failed to create data directory: {}", e))
            })?;
        }

        let mut storage = Self {
            bulletins_file,
            bulletins_cache: HashMap::new(),
            next_id: 1,
        };

        storage.load_all_bulletins()?;
        Ok(storage)
    }

    /// Load all bulletins from the JSON file into the cache
    fn load_all_bulletins(&mut self) -> BbsResult<()> {
        if !self.bulletins_file.exists() {
            // Create empty bulletins file
            let empty_bulletins: HashMap<u32, Bulletin> = HashMap::new();
            self.save_all_bulletins(&empty_bulletins)?;
            return Ok(());
        }

        let content = fs::read_to_string(&self.bulletins_file).map_err(|e| {
            BbsError::Configuration(format!("Failed to read bulletins file: {}", e))
        })?;

        if content.trim().is_empty() {
            return Ok(()); // Empty file is OK
        }

        let data: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            BbsError::Configuration(format!("Failed to parse bulletins file: {}", e))
        })?;

        // Load bulletins
        if let Some(bulletins_obj) = data.get("bulletins").and_then(|v| v.as_object()) {
            for (id_str, bulletin_data) in bulletins_obj {
                let id: u32 = id_str.parse().map_err(|_| {
                    BbsError::Configuration("Invalid bulletin ID in storage".to_string())
                })?;

                let bulletin: Bulletin =
                    serde_json::from_value(bulletin_data.clone()).map_err(|e| {
                        BbsError::Configuration(format!("Failed to parse bulletin: {}", e))
                    })?;

                self.bulletins_cache.insert(id, bulletin);
                if id >= self.next_id {
                    self.next_id = id + 1;
                }
            }
        }

        // Load next_id
        if let Some(next_id) = data.get("next_id").and_then(|v| v.as_u64()) {
            self.next_id = next_id as u32;
        }

        Ok(())
    }

    /// Save all bulletins from cache to the JSON file
    fn save_all_bulletins(&self, bulletins: &HashMap<u32, Bulletin>) -> BbsResult<()> {
        let data = serde_json::json!({
            "bulletins": bulletins,
            "next_id": self.next_id
        });

        let content = serde_json::to_string_pretty(&data).map_err(|e| {
            BbsError::Configuration(format!("Failed to serialize bulletins: {}", e))
        })?;

        fs::write(&self.bulletins_file, content).map_err(|e| {
            BbsError::Configuration(format!("Failed to write bulletins file: {}", e))
        })?;

        Ok(())
    }

    /// Get statistics about bulletins
    pub fn get_stats(&self, current_user: Option<&str>) -> BulletinStats {
        let total_bulletins = self.bulletins_cache.len();

        let unread_count = if let Some(username) = current_user {
            self.bulletins_cache
                .values()
                .filter(|b| !b.is_read_by(username))
                .count()
        } else {
            total_bulletins // Anonymous users see all as unread
        };

        // Get recent bulletins for display
        let mut recent_bulletins: Vec<&Bulletin> = self.bulletins_cache.values().collect();

        // Sort: sticky posts first, then by posted_at (newest first)
        recent_bulletins.sort_by(|a, b| match (a.is_sticky, b.is_sticky) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => b.posted_at.cmp(&a.posted_at),
        });

        let recent_summaries: Vec<BulletinSummary> = recent_bulletins
            .into_iter()
            .take(10)
            .map(|bulletin| {
                let is_read = if let Some(username) = current_user {
                    bulletin.is_read_by(username)
                } else {
                    false
                };
                (bulletin, is_read).into()
            })
            .collect();

        BulletinStats {
            total_bulletins,
            unread_count,
            recent_bulletins: recent_summaries,
        }
    }
}

impl BulletinStorage for JsonBulletinStorage {
    fn load_bulletin(&self, id: u32) -> BbsResult<Option<Bulletin>> {
        Ok(self.bulletins_cache.get(&id).cloned())
    }

    fn save_bulletin(&mut self, bulletin: &Bulletin) -> BbsResult<()> {
        // Update cache
        // TODO: can clone be removed?
        self.bulletins_cache.insert(bulletin.id, bulletin.clone());

        // Save to file
        self.save_all_bulletins(&self.bulletins_cache)?;

        Ok(())
    }

    /// Post a new bulletin
    fn post_bulletin(
        &mut self,
        request: &BulletinRequest,
        config: &crate::config::BbsConfig,
    ) -> BbsResult<u32> {
        request.validate(config)?;

        let id = self.next_id;
        self.next_id += 1;

        let bulletin = Bulletin::new(
            id,
            request.title.trim().to_string(),
            request.content.trim().to_string(),
            request.author.clone(),
        );

        self.save_bulletin(&bulletin)?;

        Ok(id)
    }

    /// Mark bulletin as read by user
    fn mark_read(&mut self, bulletin_id: u32, username: &str) -> BbsResult<()> {
        if let Some(bulletin) = self.bulletins_cache.get_mut(&bulletin_id) {
            bulletin.mark_read_by(username);
            self.save_all_bulletins(&self.bulletins_cache)?;
            Ok(())
        } else {
            Err(BbsError::InvalidInput(format!(
                "Bulletin {} not found",
                bulletin_id
            )))
        }
    }

    // List all bulletins, sorted by post date (newest first)
    // fn list_bulletins(&self) -> BbsResult<Vec<Bulletin>> {
    //     let mut bulletins: Vec<Bulletin> = self.bulletins_cache.values().cloned().collect();
    //
    //     // Sort: sticky posts first, then by posted_at (newest first)
    //     bulletins.sort_by(|a, b| match (a.is_sticky, b.is_sticky) {
    //         (true, false) => std::cmp::Ordering::Less,
    //         (false, true) => std::cmp::Ordering::Greater,
    //         _ => b.posted_at.cmp(&a.posted_at),
    //     });
    //
    //     Ok(bulletins)
    // }
    //
    /*
    /// Get recent bulletins (limited count)
    fn get_recent_bulletins(&self, limit: usize) -> BbsResult<Vec<Bulletin>> {
        let mut bulletins = self.list_bulletins()?;
        bulletins.truncate(limit);
        Ok(bulletins)
    }

    /// Get unread bulletins for a user
    fn get_unread_bulletins(&self, username: &str) -> BbsResult<Vec<Bulletin>> {
        let bulletins: Vec<Bulletin> = self
            .bulletins_cache
            .values()
            .filter(|b| !b.is_read_by(username))
            .cloned()
            .collect();
        Ok(bulletins)
    }

    /// Get bulletin count
    fn get_bulletin_count(&self) -> BbsResult<usize> {
        Ok(self.bulletins_cache.len())
    }

    /// Delete a bulletin (admin function)
    fn delete_bulletin(&mut self, id: u32) -> BbsResult<bool> {
        if self.bulletins_cache.remove(&id).is_some() {
            self.save_all_bulletins(&self.bulletins_cache)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Make bulletin sticky (admin function)
    fn set_sticky(&mut self, id: u32, sticky: bool) -> BbsResult<bool> {
        if let Some(bulletin) = self.bulletins_cache.get_mut(&id) {
            bulletin.is_sticky = sticky;
            self.save_all_bulletins(&self.bulletins_cache)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    */

    fn get_stats(&self, current_user: Option<&str>) -> BulletinStats {
        self.get_stats(current_user)
    }
}
