#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_bulletin_posting_and_reading() -> crate::errors::BbsResult<()> {
        // Create temporary directory for test
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();
        
        // Create bulletin storage
        let mut storage = crate::bulletin_repository::JsonBulletinStorage::new(temp_path)?;
        
        // Create a test bulletin
        let request = crate::bulletins::BulletinRequest::new(
            "Test Bulletin".to_string(),
            "This is test content for the bulletin.".to_string(),
            "TestUser".to_string()
        );
        
        // Mock config for validation
        let config = crate::config::BbsConfig::default();
        
        // Post the bulletin
        let bulletin_id = {
            use crate::bulletin_repository::BulletinStorage;
            storage.post_bulletin(&request, &config)?
        };
        
        // Verify it was created with ID 1
        assert_eq!(bulletin_id, 1);
        
        // Load the bulletin back
        let loaded_bulletin = {
            use crate::bulletin_repository::BulletinStorage;
            storage.load_bulletin(bulletin_id)?
        };
        
        // Verify the bulletin was loaded correctly
        assert!(loaded_bulletin.is_some());
        let bulletin = loaded_bulletin.unwrap();
        assert_eq!(bulletin.title, "Test Bulletin");
        assert_eq!(bulletin.content, "This is test content for the bulletin.");
        assert_eq!(bulletin.author, "TestUser");
        assert_eq!(bulletin.id, 1);
        assert!(!bulletin.is_sticky);
        assert!(bulletin.read_by.is_empty());
        
        // Test marking as read
        {
            use crate::bulletin_repository::BulletinStorage;
            storage.mark_read(bulletin_id, "TestUser")?;
        }
        
        // Verify it's marked as read
        let updated_bulletin = {
            use crate::bulletin_repository::BulletinStorage;
            storage.load_bulletin(bulletin_id)?.unwrap()
        };
        assert!(updated_bulletin.is_read_by("TestUser"));
        
        // Test statistics
        let stats = storage.get_stats(Some("TestUser"));
        assert_eq!(stats.total_bulletins, 1);
        assert_eq!(stats.unread_count, 0); // Read by TestUser
        assert_eq!(stats.recent_bulletins.len(), 1);
        
        let summary = &stats.recent_bulletins[0];
        assert_eq!(summary.id, bulletin_id);
        assert_eq!(summary.title, "Test Bulletin");
        assert_eq!(summary.author, "TestUser");
        assert!(summary.is_read);
        
        Ok(())
    }
}
