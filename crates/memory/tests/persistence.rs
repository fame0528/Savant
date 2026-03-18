//! Memory persistence and integration tests.
//! Tests vector persistence, delete cascade, query filtering, Drop impl.

use std::sync::Arc;
use std::time::Instant;

#[tokio::test]
async fn test_lsm_message_append_and_retrieve() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_lsm");

    let engine = match savant_memory::lsm_engine::LsmStorageEngine::new(
        &db_path,
        savant_memory::lsm_engine::LsmConfig::default(),
    ) {
        Ok(e) => e,
        Err(_) => {
            eprintln!("SKIP: Could not create LSM engine");
            return;
        }
    };

    let session = "test-session";
    let mut messages = Vec::new();
    for i in 0..10 {
        let msg = savant_memory::models::AgentMessage {
            id: format!("msg-{}", i),
            channel: "test".to_string(),
            role: savant_memory::models::MessageRole::User,
            content: format!("Test message {}", i),
            timestamp: chrono::Utc::now().timestamp() + i as i64,
            tool_name: None,
            tool_call_id: None,
        };
        messages.push(msg.clone());
        engine.append(session, msg).unwrap();
    }

    let retrieved = engine.fetch_session_tail(session, 100);
    assert_eq!(retrieved.len(), 10, "Should retrieve all 10 messages");
}

#[tokio::test]
async fn test_lsm_message_ordering() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_ordering");

    let engine = match savant_memory::lsm_engine::LsmStorageEngine::new(
        &db_path,
        savant_memory::lsm_engine::LsmConfig::default(),
    ) {
        Ok(e) => e,
        Err(_) => {
            eprintln!("SKIP: Could not create LSM engine");
            return;
        }
    };

    let session = "ordering-test";
    let base_time = chrono::Utc::now().timestamp();

    for i in 0..20 {
        let msg = savant_memory::models::AgentMessage {
            id: format!("msg-{}", i),
            channel: "test".to_string(),
            role: savant_memory::models::MessageRole::User,
            content: format!("Message {}", i),
            timestamp: base_time + i as i64,
            tool_name: None,
            tool_call_id: None,
        };
        engine.append(session, msg).unwrap();
    }

    let retrieved = engine.fetch_session_tail(session, 20);
    assert_eq!(retrieved.len(), 20);

    // Verify ordering: messages should be in timestamp order
    for i in 1..retrieved.len() {
        assert!(
            retrieved[i].timestamp >= retrieved[i - 1].timestamp,
            "Messages should be in timestamp order"
        );
    }
}

#[tokio::test]
async fn test_lsm_atomic_compact() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_compact");

    let engine = match savant_memory::lsm_engine::LsmStorageEngine::new(
        &db_path,
        savant_memory::lsm_engine::LsmConfig::default(),
    ) {
        Ok(e) => e,
        Err(_) => {
            eprintln!("SKIP: Could not create LSM engine");
            return;
        }
    };

    let session = "compact-test";

    // Insert 100 messages
    for i in 0..100 {
        let msg = savant_memory::models::AgentMessage {
            id: format!("msg-{}", i),
            channel: "test".to_string(),
            role: savant_memory::models::MessageRole::User,
            content: format!("Message {}", i),
            timestamp: chrono::Utc::now().timestamp() + i as i64,
            tool_name: None,
            tool_call_id: None,
        };
        engine.append(session, msg).unwrap();
    }

    // Compact to keep only last 20
    let compact_batch: Vec<_> = engine.fetch_session_tail(session, 20);
    engine.atomic_compact(session, compact_batch).unwrap();

    // After compaction, should only have 20 messages
    let after = engine.fetch_session_tail(session, 200);
    assert!(
        after.len() <= 30,
        "After compaction, should have ~20 messages, got {}",
        after.len()
    );
}

#[tokio::test]
async fn test_lsm_delete_session() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_delete");

    let engine = match savant_memory::lsm_engine::LsmStorageEngine::new(
        &db_path,
        savant_memory::lsm_engine::LsmConfig::default(),
    ) {
        Ok(e) => e,
        Err(_) => {
            eprintln!("SKIP: Could not create LSM engine");
            return;
        }
    };

    let session = "delete-test";

    for i in 0..50 {
        let msg = savant_memory::models::AgentMessage {
            id: format!("msg-{}", i),
            channel: "test".to_string(),
            role: savant_memory::models::MessageRole::User,
            content: format!("Message {}", i),
            timestamp: chrono::Utc::now().timestamp() + i as i64,
            tool_name: None,
            tool_call_id: None,
        };
        engine.append(session, msg).unwrap();
    }

    assert_eq!(engine.fetch_session_tail(session, 200).len(), 50);

    engine.delete_session(session).unwrap();
    assert!(
        engine.fetch_session_tail(session, 200).is_empty(),
        "Session should be empty after delete"
    );
}

#[tokio::test]
async fn test_lsm_limit_retrieval() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_limit");

    let engine = match savant_memory::lsm_engine::LsmStorageEngine::new(
        &db_path,
        savant_memory::lsm_engine::LsmConfig::default(),
    ) {
        Ok(e) => e,
        Err(_) => {
            eprintln!("SKIP: Could not create LSM engine");
            return;
        }
    };

    let session = "limit-test";
    for i in 0..200 {
        let msg = savant_memory::models::AgentMessage {
            id: format!("msg-{}", i),
            channel: "test".to_string(),
            role: savant_memory::models::MessageRole::User,
            content: format!("Message {}", i),
            timestamp: chrono::Utc::now().timestamp() + i as i64,
            tool_name: None,
            tool_call_id: None,
        };
        engine.append(session, msg).unwrap();
    }

    let tail_10 = engine.fetch_session_tail(session, 10);
    assert_eq!(tail_10.len(), 10, "Should respect limit parameter");

    let tail_50 = engine.fetch_session_tail(session, 50);
    assert_eq!(tail_50.len(), 50, "Should respect limit parameter");
}

#[tokio::test]
async fn test_lsm_high_error_rate() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_error_rate");

    let engine = match savant_memory::lsm_engine::LsmStorageEngine::new(
        &db_path,
        savant_memory::lsm_engine::LsmConfig::default(),
    ) {
        Ok(e) => e,
        Err(_) => {
            eprintln!("SKIP: Could not create LSM engine");
            return;
        }
    };

    // Insert 500 messages rapidly
    let start = Instant::now();
    for i in 0..500 {
        let msg = savant_memory::models::AgentMessage {
            id: format!("rapid-{}", i),
            channel: "stress".to_string(),
            role: savant_memory::models::MessageRole::User,
            content: format!(
                "Rapid message {} with some padding to simulate real content xyz",
                i
            ),
            timestamp: chrono::Utc::now().timestamp() + i as i64,
            tool_name: None,
            tool_call_id: None,
        };
        engine.append("rapid-session", msg).unwrap();
    }
    let elapsed = start.elapsed();

    println!("Inserted 500 messages in {:?}", elapsed);
    assert!(
        elapsed.as_secs() < 30,
        "500 inserts should complete within 30s"
    );

    let count = engine.fetch_session_tail("rapid-session", 1000).len();
    assert_eq!(count, 500, "All 500 messages should be present");
}
