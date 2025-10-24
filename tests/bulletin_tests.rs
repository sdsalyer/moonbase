mod common;

use moonbase::bulletin_repository::{BulletinStorage, JsonBulletinStorage};
use moonbase::bulletins::BulletinRequest;
use moonbase::config::BbsConfig;
use moonbase::errors::BbsResult;
use tempfile::TempDir;

#[test]
fn test_bulletin_posting_and_reading() -> BbsResult<()> {
    // Create temporary directory for test
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_str().unwrap();

    // Create bulletin storage
    let mut storage = JsonBulletinStorage::new(temp_path)?;

    // Create a test bulletin
    let request = BulletinRequest::new(
        "Test Bulletin".to_string(),
        "This is test content for the bulletin.".to_string(),
        "TestUser".to_string(),
    );

    // Mock config for validation
    let config = BbsConfig::default();

    // Post the bulletin
    let bulletin_id = storage.post_bulletin(&request, &config)?;

    // Verify it was created with ID 1
    assert_eq!(bulletin_id, 1);

    // Load the bulletin back
    let loaded_bulletin = storage.load_bulletin(bulletin_id)?;

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
    storage.mark_read(bulletin_id, "TestUser")?;

    // Verify it's marked as read
    let updated_bulletin = storage.load_bulletin(bulletin_id)?.unwrap();
    assert!(updated_bulletin.is_read_by("TestUser"));

    // Test statistics
    let stats = storage.get_stats(Some("TestUser"));
    assert_eq!(stats.total_bulletins, 1);
    assert_eq!(stats.unread_count, 0); // Read by TestUser
    assert_eq!(stats.recent_bulletins.len(), 1);

    let summary = &stats.recent_bulletins[0];
    assert_eq!(summary.title, "Test Bulletin");
    assert_eq!(summary.author, "TestUser");
    assert!(summary.is_read);

    Ok(())
}

