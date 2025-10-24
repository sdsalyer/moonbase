use crate::errors::BbsResult;
use crate::message_repository::MessageStorage;
use crate::messages::{MessageRequest, MessageStats, PrivateMessage};

use std::sync::{Arc, Mutex};

pub struct MessageService {
    storage: Arc<Mutex<dyn MessageStorage + Send>>,
}

impl MessageService {
    pub fn new(storage: Arc<Mutex<dyn MessageStorage + Send>>) -> Self {
        Self { storage }
    }

    pub fn send_message(
        &self,
        request: MessageRequest,
        config: &crate::config::BbsConfig,
    ) -> BbsResult<u32> {
        let mut storage = self.storage.lock().unwrap();
        storage.send_message(&request, config)
    }

    pub fn get_inbox(&self, username: &str) -> BbsResult<Vec<PrivateMessage>> {
        let storage = self.storage.lock().unwrap();
        storage.get_inbox(username)
    }

    pub fn get_sent(&self, username: &str) -> BbsResult<Vec<PrivateMessage>> {
        let storage = self.storage.lock().unwrap();
        storage.get_sent(username)
    }

    pub fn read_message(&self, id: u32, username: &str) -> BbsResult<Option<PrivateMessage>> {
        let mut storage = self.storage.lock().unwrap();
        
        // Get the message first
        let message = storage.get_message(id, username)?;
        
        // If it exists and the user is the recipient, mark it as read
        if let Some(ref msg) = message {
            if msg.recipient == username && msg.is_unread() {
                storage.mark_read(id, username)?;
                // Return the updated message
                return storage.get_message(id, username);
            }
        }
        
        Ok(message)
    }

    pub fn delete_message(&self, id: u32, username: &str) -> BbsResult<()> {
        let mut storage = self.storage.lock().unwrap();
        storage.delete_message(id, username)
    }

    pub fn get_stats(&self, username: &str) -> BbsResult<MessageStats> {
        let storage = self.storage.lock().unwrap();
        Ok(storage.get_stats(username))
    }

    pub fn get_unread_count(&self, username: &str) -> BbsResult<usize> {
        let stats = self.get_stats(username)?;
        Ok(stats.unread_count)
    }
}