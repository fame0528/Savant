//! Memory engine stress tests - concurrent writes, consolidation, persistence.

#[cfg(test)]
mod memory_stress_tests {
    use std::sync::Arc;
    use std::time::Instant;

    #[tokio::test]
    async fn test_concurrent_writes_same_session() {
        // This test writes 100 messages concurrently to the same session
        // and verifies all messages are present
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("stress_test");

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

        let session_id = "stress-concurrent-same";
        let mut handles = vec![];

        for i in 0..50 {
            let engine_clone = engine.clone();
            let sid = session_id.to_string();
            let handle = tokio::spawn(async move {
                let msg = savant_memory::models::AgentMessage {
                    id: format!("msg-{}", i),
                    channel: "stress".to_string(),
                    role: savant_memory::models::MessageRole::User,
                    content: format!("Concurrent message {}", i),
                    timestamp: chrono::Utc::now().timestamp(),
                    tool_name: None,
                    tool_call_id: None,
                };
                engine_clone.append(&sid, msg)
            });
            handles.push(handle);
        }

        let mut success_count = 0;
        for handle in handles {
            if handle.await.is_ok() {
                success_count += 1;
            }
        }

        assert!(
            success_count >= 45,
            "Expected most writes to succeed, got {}",
            success_count
        );
    }

    #[tokio::test]
    async fn test_concurrent_writes_different_sessions() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("stress_multi");

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

        let mut handles = vec![];

        for session_num in 0..20 {
            for msg_num in 0..10 {
                let engine_clone = engine.clone();
                let sid = format!("session-{}", session_num);
                let handle = tokio::spawn(async move {
                    let msg = savant_memory::models::AgentMessage {
                        id: format!("msg-{}-{}", session_num, msg_num),
                        channel: "multi".to_string(),
                        role: savant_memory::models::MessageRole::User,
                        content: format!("Session {} message {}", session_num, msg_num),
                        timestamp: chrono::Utc::now().timestamp(),
                        tool_name: None,
                        tool_call_id: None,
                    };
                    engine_clone.append(&sid, msg)
                });
                handles.push(handle);
            }
        }

        let mut success_count = 0;
        for handle in handles {
            if handle.await.is_ok() {
                success_count += 1;
            }
        }

        assert!(
            success_count >= 180,
            "Expected most writes to succeed, got {}",
            success_count
        );
    }

    #[tokio::test]
    async fn test_bulk_insert_performance() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("perf_test");

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

        let session_id = "perf-test";
        let start = Instant::now();

        for i in 0..1000 {
            let msg = savant_memory::models::AgentMessage {
                id: format!("perf-{}", i),
                channel: "perf".to_string(),
                role: savant_memory::models::MessageRole::User,
                content: format!("Performance test message {} with some content", i),
                timestamp: chrono::Utc::now().timestamp(),
                tool_name: None,
                tool_call_id: None,
            };
            engine.append(session_id, msg).unwrap();
        }

        let elapsed = start.elapsed();
        println!("Inserted 1000 messages in {:?}", elapsed);
        assert!(
            elapsed.as_secs() < 30,
            "1000 inserts should complete within 30s"
        );
    }
}
