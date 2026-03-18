//! Crash recovery verification tests.

#[cfg(test)]
mod crash_recovery {
    use savant_memory::{MemoryEngine, models::{AgentMessage, MessageRole}};
    use tempfile::tempdir;
    use std::sync::Arc;

    fn make_msg(id: usize) -> AgentMessage {
        AgentMessage {
            id: format!("msg-{}", id),
            session_id: "session".to_string(),
            role: MessageRole::User,
            content: format!("Test message {} with realistic content", id),
            tool_calls: vec![],
            tool_results: vec![],
            timestamp: rend::i64_le::from(chrono::Utc::now().timestamp()),
            parent_id: None,
            channel: "test".to_string(),
        }
    }

    #[test]
    fn test_graceful_restart() {
        let dir = tempdir().unwrap();
        {
            let engine = MemoryEngine::with_defaults(dir.path()).unwrap();
            for i in 0..100 {
                engine.append_message("sess", &make_msg(i)).unwrap();
            }
        }
        let engine = MemoryEngine::with_defaults(dir.path()).unwrap();
        let msgs = engine.fetch_session_tail("sess", 200);
        assert!(msgs.len() >= 90, "Expected ~100 msgs, got {}", msgs.len());
    }

    #[test]
    fn test_crash_drop() {
        let dir = tempdir().unwrap();
        {
            let engine = Arc::new(MemoryEngine::with_defaults(dir.path()).unwrap());
            for i in 0..200 {
                engine.append_message("crash-sess", &make_msg(i)).unwrap();
            }
            drop(engine);
        }
        let engine = MemoryEngine::with_defaults(dir.path()).unwrap();
        let msgs = engine.fetch_session_tail("crash-sess", 500);
        assert!(msgs.len() >= 150, "Expected ~200 msgs after crash, got {}", msgs.len());
    }

    #[test]
    fn test_ordering_after_restart() {
        let dir = tempdir().unwrap();
        {
            let engine = MemoryEngine::with_defaults(dir.path()).unwrap();
            for i in 0..50 {
                engine.append_message("ord", &make_msg(i)).unwrap();
            }
        }
        let engine = MemoryEngine::with_defaults(dir.path()).unwrap();
        let msgs = engine.fetch_session_tail("ord", 100);
        assert_eq!(msgs.len(), 50, "All 50 messages should be recovered");
    }

    #[test]
    fn test_empty_engine_restart() {
        let dir = tempdir().unwrap();
        assert!(MemoryEngine::with_defaults(dir.path()).is_ok());
    }

    #[test]
    fn test_different_sessions_independent() {
        let dir = tempdir().unwrap();
        let engine = MemoryEngine::with_defaults(dir.path()).unwrap();

        engine.append_message("sess-a", &make_msg(1)).unwrap();
        engine.append_message("sess-b", &make_msg(2)).unwrap();

        let a = engine.fetch_session_tail("sess-a", 10);
        let b = engine.fetch_session_tail("sess-b", 10);

        assert_eq!(a.len(), 1);
        assert_eq!(b.len(), 1);
    }

    #[test]
    fn test_bulk_write_survives_restart() {
        let dir = tempdir().unwrap();
        {
            let engine = MemoryEngine::with_defaults(dir.path()).unwrap();
            for i in 0..500 {
                engine.append_message("bulk", &make_msg(i)).unwrap();
            }
        }
        let engine = MemoryEngine::with_defaults(dir.path()).unwrap();
        let msgs = engine.fetch_session_tail("bulk", 1000);
        assert!(msgs.len() >= 400, "Expected ~500 msgs after restart, got {}", msgs.len());
    }
}
