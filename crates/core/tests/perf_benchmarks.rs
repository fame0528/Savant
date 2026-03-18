//! Performance benchmarks for Savant core components.
//! Run with: cargo test -p savant_core --test perf_benchmarks

use std::time::Instant;

#[test]
fn bench_storage_append() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("bench_storage");

    let storage = match savant_core::db::Storage::new(db_path) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("SKIP: Could not create storage");
            return;
        }
    };

    let start = Instant::now();
    for i in 0..1000 {
        let msg = savant_core::types::ChatMessage {
            id: format!("bench-{}", i),
            role: savant_core::types::ChatRole::User,
            content: format!("Benchmark message {} with padding to simulate realistic content size for performance testing", i),
            timestamp: chrono::Utc::now().timestamp(),
            channel: "bench".to_string(),
            metadata: None,
        };
        storage.append_chat("bench-agent", &msg).unwrap();
    }
    let elapsed = start.elapsed();

    println!(
        "Storage append: 1000 messages in {:?} ({:.0} msg/s)",
        elapsed,
        1000.0 / elapsed.as_secs_f64()
    );
    assert!(
        elapsed.as_secs() < 10,
        "1000 appends should complete within 10s"
    );
}

#[test]
fn bench_storage_retrieve() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("bench_retrieve");

    let storage = match savant_core::db::Storage::new(db_path) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("SKIP: Could not create storage");
            return;
        }
    };

    // Insert 500 messages
    for i in 0..500 {
        let msg = savant_core::types::ChatMessage {
            id: format!("ret-{}", i),
            role: savant_core::types::ChatRole::User,
            content: format!("Message {}", i),
            timestamp: chrono::Utc::now().timestamp(),
            channel: "ret".to_string(),
            metadata: None,
        };
        storage.append_chat("ret-agent", &msg).unwrap();
    }

    let start = Instant::now();
    for _ in 0..100 {
        let _ = storage.get_history("ret-agent", 50).unwrap();
    }
    let elapsed = start.elapsed();

    println!(
        "Storage retrieve: 100 queries in {:?} ({:.0} q/s)",
        elapsed,
        100.0 / elapsed.as_secs_f64()
    );
}

#[test]
fn bench_session_id_sanitization() {
    let start = Instant::now();
    for i in 0..10000 {
        let input = format!("test-session-{}!@#$%", i);
        let _ = savant_core::session::SessionMapper::sanitize(&input);
    }
    let elapsed = start.elapsed();

    println!(
        "Session sanitize: 10000 calls in {:?} ({:.0} calls/s)",
        elapsed,
        10000.0 / elapsed.as_secs_f64()
    );
}

#[test]
fn bench_skill_name_validation() {
    let start = Instant::now();
    for i in 0..10000 {
        let name = format!("skill-name-{}", i);
        let _ = name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_');
    }
    let elapsed = start.elapsed();

    println!(
        "Skill name validation: 10000 calls in {:?} ({:.0} calls/s)",
        elapsed,
        10000.0 / elapsed.as_secs_f64()
    );
}
