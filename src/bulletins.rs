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

    /// Wrap `text` into lines with maximum `width` glyphs (measured as Unicode scalar count).
    /// Hard newlines ('\n') are preserved. Words are not split; if a single word is longer than
    /// `width` it will be placed on its own line (no hyphenation).
    pub fn get_content_lines(&self, width: usize) -> Vec<String> {
        let text = &self.content;

        // width == 0: just return lines split on actual newlines
        if width == 0 {
            return text.split("\\n").map(|s| s.to_string()).collect();
        }

        let mut out = Vec::new();

        for hard_line in text.split("\\n") {
            // If hard_line is empty (i.e., consecutive '\n' or leading/trailing '\n'), preserve empty line.
            if hard_line.is_empty() {
                out.push(String::new());
                continue;
            }

            // Iterate over runs of whitespace-separated words in the hard line.
            // We need to preserve only words (split_whitespace), but ensure wrapping so no output line exceeds width.
            let mut current = String::new();

            for word in hard_line.split_whitespace() {
                let word_len = word.chars().count();

                // If word itself is longer than width, we must break it into chunks of size <= width.
                if word_len > width {
                    // First flush any pending current line.
                    if !current.is_empty() {
                        out.push(current);
                        current = String::new();
                    }

                    // Split the long word into char chunks of size `width`.
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
                    // After splitting the long word, continue to next word (no pending current).
                } else {
                    // Normal word fits within width.
                    if current.is_empty() {
                        // Start a new current with the word.
                        current.push_str(word);
                    } else {
                        // Would adding this word (with a space) exceed width?
                        let new_len = current.chars().count() + 1 + word_len;
                        if new_len <= width {
                            current.push(' ');
                            current.push_str(word);
                        } else {
                            // Flush current, start new current with word.
                            out.push(current);
                            current = String::new();
                            current.push_str(word);
                        }
                    }
                }
            }

            // After processing words in hard_line, flush any pending current.
            if !current.is_empty() {
                out.push(current);
            }
        }

        out
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
