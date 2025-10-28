use crate::errors::{BbsError, BbsResult};
use crate::messages::{MessageRequest, MessageStats, MessageSummary, PrivateMessage};

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub trait MessageStorage {
    fn send_message(
        &mut self,
        request: &MessageRequest,
        config: &crate::config::BbsConfig,
    ) -> BbsResult<u32>;
    fn get_message(&self, id: u32, username: &str) -> BbsResult<Option<PrivateMessage>>;
    fn get_inbox(&self, username: &str) -> BbsResult<Vec<PrivateMessage>>;
    fn get_sent(&self, username: &str) -> BbsResult<Vec<PrivateMessage>>;
    fn mark_read(&mut self, id: u32, username: &str) -> BbsResult<()>;
    fn delete_message(&mut self, id: u32, username: &str) -> BbsResult<()>;
    fn get_stats(&self, username: &str) -> MessageStats;
}

/// JSON file-based private message storage implementation
pub struct JsonMessageStorage {
    messages_file: PathBuf,
    messages_cache: HashMap<u32, PrivateMessage>,
    next_id: u32,
}

impl JsonMessageStorage {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> BbsResult<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        let messages_file = data_dir.join("messages.json");

        // Create data directory if it doesn't exist
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir).map_err(|e| {
                BbsError::Configuration(format!("Failed to create data directory: {}", e))
            })?;
        }

        let mut storage = Self {
            messages_file,
            messages_cache: HashMap::new(),
            next_id: 1,
        };

        storage.load_all_messages()?;
        Ok(storage)
    }

    /// Load all messages from the JSON file into the cache
    fn load_all_messages(&mut self) -> BbsResult<()> {
        if !self.messages_file.exists() {
            // Create empty messages file
            let empty_messages: HashMap<u32, PrivateMessage> = HashMap::new();
            self.save_all_messages(&empty_messages)?;
            return Ok(());
        }

        let content = fs::read_to_string(&self.messages_file)
            .map_err(|e| BbsError::Configuration(format!("Failed to read messages file: {}", e)))?;

        if content.trim().is_empty() {
            let empty_messages: HashMap<u32, PrivateMessage> = HashMap::new();
            self.save_all_messages(&empty_messages)?;
            return Ok(());
        }

        let messages: HashMap<u32, PrivateMessage> =
            serde_json::from_str(&content).map_err(|e| {
                BbsError::Configuration(format!("Failed to parse messages file: {}", e))
            })?;

        self.next_id = messages.keys().max().unwrap_or(&0) + 1;
        self.messages_cache = messages;

        Ok(())
    }

    /// Save all messages from the cache to the JSON file
    fn save_all_messages(&self, messages: &HashMap<u32, PrivateMessage>) -> BbsResult<()> {
        let content = serde_json::to_string_pretty(messages)
            .map_err(|e| BbsError::Configuration(format!("Failed to serialize messages: {}", e)))?;

        fs::write(&self.messages_file, content).map_err(|e| {
            BbsError::Configuration(format!("Failed to write messages file: {}", e))
        })?;

        Ok(())
    }

    /// Save a single message to the cache and file
    fn save_message(&mut self, message: &PrivateMessage) -> BbsResult<()> {
        self.messages_cache.insert(message.id, message.clone());
        self.save_all_messages(&self.messages_cache)
    }

    /// Check if a user exists (placeholder - would need user storage reference)
    fn user_exists(&self, _username: &str) -> bool {
        // TODO: This should check against user storage
        // For now, assume all usernames are valid
        true
    }
}

impl MessageStorage for JsonMessageStorage {
    fn send_message(
        &mut self,
        request: &MessageRequest,
        config: &crate::config::BbsConfig,
    ) -> BbsResult<u32> {
        // Validate the request
        request.validate(config)?;

        // Check if recipient exists
        if !self.user_exists(&request.recipient) {
            return Err(BbsError::InvalidInput(format!(
                "User '{}' does not exist",
                request.recipient
            )));
        }

        // Create the message
        let message = PrivateMessage::new(
            self.next_id,
            request.sender.clone(),
            request.recipient.clone(),
            request.subject.clone(),
            request.content.clone(),
        );

        let message_id = message.id;
        self.next_id += 1;

        // Save the message
        self.save_message(&message)?;

        Ok(message_id)
    }

    fn get_message(&self, id: u32, username: &str) -> BbsResult<Option<PrivateMessage>> {
        if let Some(message) = self.messages_cache.get(&id) {
            // Check if user has permission to read this message
            if message.is_visible_to(username) {
                Ok(Some(message.clone()))
            } else {
                // Message exists but user can't see it (wrong user or deleted)
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn get_inbox(&self, username: &str) -> BbsResult<Vec<PrivateMessage>> {
        let mut inbox: Vec<PrivateMessage> = self
            .messages_cache
            .values()
            .filter(|msg| msg.recipient == username && !msg.is_deleted_by_recipient)
            .cloned()
            .collect();

        // Sort by sent_at descending (newest first)
        inbox.sort_by(|a, b| b.sent_at.cmp(&a.sent_at));

        Ok(inbox)
    }

    fn get_sent(&self, username: &str) -> BbsResult<Vec<PrivateMessage>> {
        let mut sent: Vec<PrivateMessage> = self
            .messages_cache
            .values()
            .filter(|msg| msg.sender == username && !msg.is_deleted_by_sender)
            .cloned()
            .collect();

        // Sort by sent_at descending (newest first)
        sent.sort_by(|a, b| b.sent_at.cmp(&a.sent_at));

        Ok(sent)
    }

    fn mark_read(&mut self, id: u32, username: &str) -> BbsResult<()> {
        if let Some(message) = self.messages_cache.get_mut(&id) {
            // Only the recipient can mark a message as read
            if message.recipient == username {
                message.mark_read();
                self.save_all_messages(&self.messages_cache)?;
                Ok(())
            } else {
                Err(BbsError::InvalidInput(
                    "Only the recipient can mark a message as read".to_string(),
                ))
            }
        } else {
            Err(BbsError::InvalidInput("Message not found".to_string()))
        }
    }

    fn delete_message(&mut self, id: u32, username: &str) -> BbsResult<()> {
        if let Some(message) = self.messages_cache.get_mut(&id) {
            // Check if user has permission to delete this message
            if message.is_visible_to(username) {
                message.delete_for(username);

                // If both sender and recipient have deleted, remove from cache
                if message.is_deleted_by_sender && message.is_deleted_by_recipient {
                    self.messages_cache.remove(&id);
                }

                self.save_all_messages(&self.messages_cache)?;
                Ok(())
            } else {
                Err(BbsError::InvalidInput(
                    "You don't have permission to delete this message".to_string(),
                ))
            }
        } else {
            Err(BbsError::InvalidInput("Message not found".to_string()))
        }
    }

    fn get_stats(&self, username: &str) -> MessageStats {
        let inbox_messages: Vec<&PrivateMessage> = self
            .messages_cache
            .values()
            .filter(|msg| msg.recipient == username && !msg.is_deleted_by_recipient)
            .collect();

        let sent_messages: Vec<&PrivateMessage> = self
            .messages_cache
            .values()
            .filter(|msg| msg.sender == username && !msg.is_deleted_by_sender)
            .collect();

        let unread_count = inbox_messages.iter().filter(|msg| msg.is_unread()).count();
        let total_received = inbox_messages.len();
        let total_sent = sent_messages.len();

        // Get recent messages (last 10 from inbox)
        let mut recent_inbox: Vec<&PrivateMessage> = inbox_messages;
        recent_inbox.sort_by(|a, b| b.sent_at.cmp(&a.sent_at));
        recent_inbox.truncate(10);

        let recent_messages: Vec<MessageSummary> = recent_inbox
            .iter()
            .map(|msg| MessageSummary::from(*msg))
            .collect();

        MessageStats {
            unread_count,
            total_received,
            total_sent,
            recent_messages,
        }
    }
}
