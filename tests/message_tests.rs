mod common;

use moonbase::config::BbsConfig;
use moonbase::message_repository::{JsonMessageStorage, MessageStorage};
use moonbase::messages::{MessageRequest, PrivateMessage};
use tempfile::TempDir;

fn create_test_storage() -> (JsonMessageStorage, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let storage = JsonMessageStorage::new(temp_dir.path()).unwrap();
    (storage, temp_dir)
}

#[test]
fn test_message_creation() {
    let message = PrivateMessage::new(
        1,
        "alice".to_string(),
        "bob".to_string(),
        "Hello World".to_string(),
        "This is a test message.".to_string(),
    );

    assert_eq!(message.id, 1);
    assert_eq!(message.sender, "alice");
    assert_eq!(message.recipient, "bob");
    assert_eq!(message.subject, "Hello World");
    assert_eq!(message.content, "This is a test message.");
    assert!(message.is_unread());
    assert!(!message.is_deleted_by_sender);
    assert!(!message.is_deleted_by_recipient);
}

#[test]
fn test_message_read_marking() {
    let mut message = PrivateMessage::new(
        1,
        "alice".to_string(),
        "bob".to_string(),
        "Test Subject".to_string(),
        "Test content.".to_string(),
    );

    assert!(message.is_unread());

    message.mark_read();
    assert!(!message.is_unread());
    assert!(message.read_at.is_some());
}

#[test]
fn test_message_visibility() {
    let message = PrivateMessage::new(
        1,
        "alice".to_string(),
        "bob".to_string(),
        "Test Subject".to_string(),
        "Test content.".to_string(),
    );

    assert!(message.is_visible_to("alice"));
    assert!(message.is_visible_to("bob"));
    assert!(!message.is_visible_to("charlie"));
}

#[test]
fn test_message_deletion() {
    let mut message = PrivateMessage::new(
        1,
        "alice".to_string(),
        "bob".to_string(),
        "Test Subject".to_string(),
        "Test content.".to_string(),
    );

    // Delete for sender
    message.delete_for("alice");
    assert!(message.is_deleted_by_sender);
    assert!(!message.is_deleted_by_recipient);
    assert!(!message.is_visible_to("alice"));
    assert!(message.is_visible_to("bob"));

    // Delete for recipient
    message.delete_for("bob");
    assert!(message.is_deleted_by_sender);
    assert!(message.is_deleted_by_recipient);
    assert!(!message.is_visible_to("alice"));
    assert!(!message.is_visible_to("bob"));
}

#[test]
fn test_message_request_validation() {
    let config = BbsConfig::default();

    // Valid request
    let valid_request = MessageRequest::new(
        "bob".to_string(),
        "Hello".to_string(),
        "This is a valid message.".to_string(),
        "alice".to_string(),
    );
    assert!(valid_request.validate(&config).is_ok());

    // Empty recipient
    let invalid_request = MessageRequest::new(
        "".to_string(),
        "Hello".to_string(),
        "This is a message.".to_string(),
        "alice".to_string(),
    );
    assert!(invalid_request.validate(&config).is_err());

    // Empty subject
    let invalid_request = MessageRequest::new(
        "bob".to_string(),
        "".to_string(),
        "This is a message.".to_string(),
        "alice".to_string(),
    );
    assert!(invalid_request.validate(&config).is_err());

    // Empty content
    let invalid_request = MessageRequest::new(
        "bob".to_string(),
        "Hello".to_string(),
        "".to_string(),
        "alice".to_string(),
    );
    assert!(invalid_request.validate(&config).is_err());

    // Self-message
    let invalid_request = MessageRequest::new(
        "alice".to_string(),
        "Hello".to_string(),
        "This is a self message.".to_string(),
        "alice".to_string(),
    );
    assert!(invalid_request.validate(&config).is_err());
}

#[test]
fn test_message_storage_send_and_retrieve() {
    let (mut storage, _temp_dir) = create_test_storage();
    let config = BbsConfig::default();

    let request = MessageRequest::new(
        "bob".to_string(),
        "Test Subject".to_string(),
        "This is a test message content.".to_string(),
        "alice".to_string(),
    );

    // Send message
    let message_id = storage.send_message(&request, &config).unwrap();
    assert_eq!(message_id, 1);

    // Retrieve message as recipient
    let message = storage.get_message(message_id, "bob").unwrap();
    assert!(message.is_some());
    let message = message.unwrap();
    assert_eq!(message.sender, "alice");
    assert_eq!(message.recipient, "bob");
    assert_eq!(message.subject, "Test Subject");
    assert_eq!(message.content, "This is a test message content.");

    // Try to retrieve as unauthorized user
    let result = storage.get_message(message_id, "charlie").unwrap();
    assert!(result.is_none());
}

#[test]
fn test_message_storage_inbox_and_sent() {
    let (mut storage, _temp_dir) = create_test_storage();
    let config = BbsConfig::default();

    // Send message from alice to bob
    let request1 = MessageRequest::new(
        "bob".to_string(),
        "Message 1".to_string(),
        "First message content.".to_string(),
        "alice".to_string(),
    );
    storage.send_message(&request1, &config).unwrap();

    // Send message from bob to alice
    let request2 = MessageRequest::new(
        "alice".to_string(),
        "Message 2".to_string(),
        "Second message content.".to_string(),
        "bob".to_string(),
    );
    storage.send_message(&request2, &config).unwrap();

    // Check alice's inbox (should have 1 message from bob)
    let alice_inbox = storage.get_inbox("alice").unwrap();
    assert_eq!(alice_inbox.len(), 1);
    assert_eq!(alice_inbox[0].sender, "bob");
    assert_eq!(alice_inbox[0].subject, "Message 2");

    // Check alice's sent messages (should have 1 message to bob)
    let alice_sent = storage.get_sent("alice").unwrap();
    assert_eq!(alice_sent.len(), 1);
    assert_eq!(alice_sent[0].recipient, "bob");
    assert_eq!(alice_sent[0].subject, "Message 1");

    // Check bob's inbox (should have 1 message from alice)
    let bob_inbox = storage.get_inbox("bob").unwrap();
    assert_eq!(bob_inbox.len(), 1);
    assert_eq!(bob_inbox[0].sender, "alice");
    assert_eq!(bob_inbox[0].subject, "Message 1");

    // Check bob's sent messages (should have 1 message to alice)
    let bob_sent = storage.get_sent("bob").unwrap();
    assert_eq!(bob_sent.len(), 1);
    assert_eq!(bob_sent[0].recipient, "alice");
    assert_eq!(bob_sent[0].subject, "Message 2");
}

#[test]
fn test_message_mark_read() {
    let (mut storage, _temp_dir) = create_test_storage();
    let config = BbsConfig::default();

    let request = MessageRequest::new(
        "bob".to_string(),
        "Test Subject".to_string(),
        "Test content.".to_string(),
        "alice".to_string(),
    );

    let message_id = storage.send_message(&request, &config).unwrap();

    // Message should be unread initially
    let message = storage.get_message(message_id, "bob").unwrap().unwrap();
    assert!(message.is_unread());

    // Mark as read by recipient
    storage.mark_read(message_id, "bob").unwrap();

    // Message should now be read
    let message = storage.get_message(message_id, "bob").unwrap().unwrap();
    assert!(!message.is_unread());

    // Sender should not be able to mark as read
    let result = storage.mark_read(message_id, "alice");
    assert!(result.is_err());
}

#[test]
fn test_message_delete() {
    let (mut storage, _temp_dir) = create_test_storage();
    let config = BbsConfig::default();

    let request = MessageRequest::new(
        "bob".to_string(),
        "Test Subject".to_string(),
        "Test content.".to_string(),
        "alice".to_string(),
    );

    let message_id = storage.send_message(&request, &config).unwrap();

    // Both users should be able to see the message initially
    assert!(storage.get_message(message_id, "alice").unwrap().is_some());
    assert!(storage.get_message(message_id, "bob").unwrap().is_some());

    // Delete for sender
    storage.delete_message(message_id, "alice").unwrap();

    // Sender should no longer see it, recipient should still see it
    assert!(storage.get_message(message_id, "alice").unwrap().is_none());
    assert!(storage.get_message(message_id, "bob").unwrap().is_some());

    // Delete for recipient
    storage.delete_message(message_id, "bob").unwrap();

    // Both should no longer see it
    assert!(storage.get_message(message_id, "alice").unwrap().is_none());
    assert!(storage.get_message(message_id, "bob").unwrap().is_none());
}

#[test]
fn test_message_stats() {
    let (mut storage, _temp_dir) = create_test_storage();
    let config = BbsConfig::default();

    // Send multiple messages to alice
    for i in 1..=3 {
        let request = MessageRequest::new(
            "alice".to_string(),
            format!("Message {}", i),
            format!("Content {}", i),
            "bob".to_string(),
        );
        storage.send_message(&request, &config).unwrap();
    }

    // Send one message from alice to bob
    let request = MessageRequest::new(
        "bob".to_string(),
        "Reply".to_string(),
        "Reply content".to_string(),
        "alice".to_string(),
    );
    storage.send_message(&request, &config).unwrap();

    // Check alice's stats
    let alice_stats = storage.get_stats("alice");
    assert_eq!(alice_stats.total_received, 3);
    assert_eq!(alice_stats.total_sent, 1);
    assert_eq!(alice_stats.unread_count, 3); // All messages unread
    assert_eq!(alice_stats.recent_messages.len(), 3);

    // Mark one message as read
    storage.mark_read(1, "alice").unwrap();

    // Check updated stats
    let alice_stats = storage.get_stats("alice");
    assert_eq!(alice_stats.unread_count, 2); // One less unread
}

#[test]
fn test_message_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let config = BbsConfig::default();

    let message_id = {
        let mut storage = JsonMessageStorage::new(temp_dir.path()).unwrap();
        let request = MessageRequest::new(
            "bob".to_string(),
            "Persistent Message".to_string(),
            "This should persist.".to_string(),
            "alice".to_string(),
        );
        storage.send_message(&request, &config).unwrap()
    };

    // Create new storage instance and verify message exists
    {
        let storage = JsonMessageStorage::new(temp_dir.path()).unwrap();
        let message = storage.get_message(message_id, "bob").unwrap();
        assert!(message.is_some());
        let message = message.unwrap();
        assert_eq!(message.subject, "Persistent Message");
        assert_eq!(message.sender, "alice");
        assert_eq!(message.recipient, "bob");
    }
}
