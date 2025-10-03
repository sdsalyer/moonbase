use crate::errors::{BbsError, BbsResult};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};

/// A bulletin post
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bulletin {
    pub id: u32,
    pub title: String,
    pub content: String,
    pub author: String,
    pub posted_at: Timestamp,
    pub is_sticky: bool,
    pub read_by: Vec<String>, // usernames who have read this bulletin
}

impl Bulletin {
    pub fn new(id: u32, title: String, content: String, author: String) -> Self {
        Self {
            id,
            title,
            content,
            author,
            posted_at: Timestamp::now(),
            is_sticky: false,
            read_by: Vec::new(),
        }
    }

    pub fn mark_read_by(&mut self, username: &str) {
        if !self.read_by.contains(&username.to_string()) {
            self.read_by.push(username.to_string());
        }
    }

    pub fn is_read_by(&self, username: &str) -> bool {
        self.read_by.contains(&username.to_string())
    }

    pub fn posted_display(&self) -> String {
        let now = Timestamp::now();
        let duration_since = now.duration_since(self.posted_at);
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

/// Request to create a new bulletin
#[derive(Debug)]
pub struct BulletinRequest {
    pub title: String,
    pub content: String,
    pub author: String,
}

impl BulletinRequest {
    pub fn new(title: String, content: String, author: String) -> Self {
        Self {
            title,
            content,
            author,
        }
    }

    pub fn validate(&self, config: &crate::config::BbsConfig) -> BbsResult<()> {
        if self.title.trim().is_empty() {
            return Err(BbsError::InvalidInput("Title cannot be empty".to_string()));
        }

        if self.title.len() > 100 {
            return Err(BbsError::InvalidInput(
                "Title too long (max 100 characters)".to_string(),
            ));
        }

        if self.content.trim().is_empty() {
            return Err(BbsError::InvalidInput(
                "Content cannot be empty".to_string(),
            ));
        }

        if self.content.len() > config.features.max_message_length {
            return Err(BbsError::InvalidInput(format!(
                "Content too long (max {} characters)",
                config.features.max_message_length
            )));
        }

        if self.author.trim().is_empty() {
            return Err(BbsError::InvalidInput("Author cannot be empty".to_string()));
        }

        Ok(())
    }
}
