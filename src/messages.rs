use crate::errors::{BbsError, BbsResult};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};

/// A private message between users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateMessage {
    pub id: u32,
    pub sender: String,
    pub recipient: String,
    pub subject: String,
    pub content: String,
    pub sent_at: Timestamp,
    pub read_at: Option<Timestamp>,
    pub is_deleted_by_sender: bool,
    pub is_deleted_by_recipient: bool,
}

impl PrivateMessage {
    pub fn new(
        id: u32,
        sender: String,
        recipient: String,
        subject: String,
        content: String,
    ) -> Self {
        Self {
            id,
            sender,
            recipient,
            subject,
            content,
            sent_at: Timestamp::now(),
            read_at: None,
            is_deleted_by_sender: false,
            is_deleted_by_recipient: false,
        }
    }

    pub fn is_unread(&self) -> bool {
        self.read_at.is_none()
    }

    pub fn mark_read(&mut self) {
        if self.read_at.is_none() {
            self.read_at = Some(Timestamp::now());
        }
    }

    pub fn is_visible_to(&self, username: &str) -> bool {
        if username == self.sender {
            !self.is_deleted_by_sender
        } else if username == self.recipient {
            !self.is_deleted_by_recipient
        } else {
            false
        }
    }

    pub fn delete_for(&mut self, username: &str) {
        if username == self.sender {
            self.is_deleted_by_sender = true;
        } else if username == self.recipient {
            self.is_deleted_by_recipient = true;
        }
    }

    pub fn sent_display(&self) -> String {
        let now = Timestamp::now();
        let duration_since = now.duration_since(self.sent_at);
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

    /// Wrap content text for display with given width
    pub fn get_content_lines(&self, width: usize) -> Vec<String> {
        let text = &self.content;

        if width == 0 {
            return text.split("\\n").map(|s| s.to_string()).collect();
        }

        let mut out = Vec::new();

        for hard_line in text.split("\\n") {
            if hard_line.is_empty() {
                out.push(String::new());
                continue;
            }

            let mut current = String::new();

            for word in hard_line.split_whitespace() {
                let word_len = word.chars().count();

                if word_len > width {
                    if !current.is_empty() {
                        out.push(current);
                        current = String::new();
                    }

                    let char_iter = word.chars();
                    let mut chunk = String::new();
                    for ch in char_iter {
                        chunk.push(ch);
                        if chunk.chars().count() == width {
                            out.push(chunk);
                            chunk = String::new();
                        }
                    }
                    if !chunk.is_empty() {
                        out.push(chunk);
                    }
                } else {
                    if current.is_empty() {
                        current.push_str(word);
                    } else {
                        let new_len = current.chars().count() + 1 + word_len;
                        if new_len <= width {
                            current.push(' ');
                            current.push_str(word);
                        } else {
                            out.push(current);
                            current = String::new();
                            current.push_str(word);
                        }
                    }
                }
            }

            if !current.is_empty() {
                out.push(current);
            }
        }

        out
    }
}

/// Request to send a new private message
#[derive(Debug)]
pub struct MessageRequest {
    pub recipient: String,
    pub subject: String,
    pub content: String,
    pub sender: String,
}

impl MessageRequest {
    pub fn new(recipient: String, subject: String, content: String, sender: String) -> Self {
        Self {
            recipient,
            subject,
            content,
            sender,
        }
    }

    pub fn validate(&self, config: &crate::config::BbsConfig) -> BbsResult<()> {
        if self.recipient.trim().is_empty() {
            return Err(BbsError::InvalidInput("Recipient cannot be empty".to_string()));
        }

        if self.recipient.len() > config.features.max_username_length {
            return Err(BbsError::InvalidInput(format!(
                "Recipient name too long (max {} characters)",
                config.features.max_username_length
            )));
        }

        if self.subject.trim().is_empty() {
            return Err(BbsError::InvalidInput("Subject cannot be empty".to_string()));
        }

        if self.subject.len() > 100 {
            return Err(BbsError::InvalidInput(
                "Subject too long (max 100 characters)".to_string(),
            ));
        }

        if self.content.trim().is_empty() {
            return Err(BbsError::InvalidInput("Message content cannot be empty".to_string()));
        }

        if self.content.len() > config.features.max_message_length {
            return Err(BbsError::InvalidInput(format!(
                "Message too long (max {} characters)",
                config.features.max_message_length
            )));
        }

        if self.sender.trim().is_empty() {
            return Err(BbsError::InvalidInput("Sender cannot be empty".to_string()));
        }

        if self.sender == self.recipient {
            return Err(BbsError::InvalidInput(
                "Cannot send message to yourself".to_string(),
            ));
        }

        Ok(())
    }
}

/// Statistics about private messages for display
#[derive(Debug, Clone, Default)]
pub struct MessageStats {
    pub unread_count: usize,
    pub total_received: usize,
    pub total_sent: usize,
    pub recent_messages: Vec<MessageSummary>,
}

/// Summary of a message for menu display
#[derive(Debug, Clone)]
pub struct MessageSummary {
    pub id: u32,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub sent_display: String,
    pub is_read: bool,
}

impl From<&PrivateMessage> for MessageSummary {
    fn from(message: &PrivateMessage) -> Self {
        Self {
            id: message.id,
            from: message.sender.clone(),
            to: message.recipient.clone(),
            subject: message.subject.clone(),
            sent_display: message.sent_display(),
            is_read: !message.is_unread(),
        }
    }
}