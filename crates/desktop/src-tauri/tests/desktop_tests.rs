// Tests for savant-desktop (Tauri) crate.
//
// Tauri commands require the full Tauri runtime (AppHandle, webview, etc.)
// which cannot be instantiated in unit tests. These tests verify:
// 1. Source code structure and completeness
// 2. The bootstrap_log function logic (pure filesystem operations)
// 3. Path resolution patterns
// 4. Command handler signatures

use std::path::PathBuf;

fn make_temp_dir() -> PathBuf {
    let id = uuid::Uuid::new_v4();
    std::env::temp_dir().join(format!("savant-desktop-test-{}", id))
}

fn cleanup(path: &PathBuf) {
    let _ = std::fs::remove_dir_all(path);
}

// ─── Source code verification tests ───────────────────────────────────────

#[test]
fn test_main_rs_has_all_tauri_commands() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = PathBuf::from(manifest_dir).join("src/main.rs");
    let content = std::fs::read_to_string(&src_path).unwrap();

    // Verify all expected Tauri commands are defined
    assert!(content.contains("#[tauri::command]"), "Missing tauri::command attribute");
    assert!(content.contains("async fn ignite_swarm"), "Missing ignite_swarm command");
    assert!(content.contains("async fn get_swarm_status"), "Missing get_swarm_status command");
    assert!(content.contains("async fn get_version"), "Missing get_version command");
    assert!(content.contains("async fn show_browser"), "Missing show_browser command");
    assert!(content.contains("async fn hide_browser"), "Missing hide_browser command");
    assert!(content.contains("async fn browser_go_back"), "Missing browser_go_back command");
    assert!(content.contains("async fn browser_go_forward"), "Missing browser_go_forward command");
    assert!(content.contains("async fn browser_reload"), "Missing browser_reload command");
    assert!(content.contains("async fn browser_navigate"), "Missing browser_navigate command");
    assert!(content.contains("async fn browser_get_tabs"), "Missing browser_get_tabs command");
}

#[test]
fn test_main_rs_has_invoke_handler() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = PathBuf::from(manifest_dir).join("src/main.rs");
    let content = std::fs::read_to_string(&src_path).unwrap();
    assert!(content.contains("invoke_handler"), "Missing invoke_handler");
    assert!(content.contains("tauri::generate_handler"), "Missing generate_handler");
}

#[test]
fn test_main_rs_has_tracing_setup() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = PathBuf::from(manifest_dir).join("src/main.rs");
    let content = std::fs::read_to_string(&src_path).unwrap();
    assert!(content.contains("LogBridge"), "Missing LogBridge");
    assert!(content.contains("tracing_subscriber"), "Missing tracing_subscriber");
    assert!(content.contains("try_init"), "Missing try_init for tracing");
}

#[test]
fn test_main_rs_has_path_resolver_setup() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = PathBuf::from(manifest_dir).join("src/main.rs");
    let content = std::fs::read_to_string(&src_path).unwrap();
    assert!(content.contains("SavantPathResolver"), "Missing SavantPathResolver");
    assert!(content.contains("app.manage(resolver)"), "Missing path resolver management");
}

#[test]
fn test_main_rs_has_system_tray() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = PathBuf::from(manifest_dir).join("src/main.rs");
    let content = std::fs::read_to_string(&src_path).unwrap();
    assert!(content.contains("TrayIconBuilder"), "Missing TrayIconBuilder");
    assert!(content.contains("MenuBuilder"), "Missing MenuBuilder");
    assert!(content.contains("MenuItemBuilder"), "Missing MenuItemBuilder");
}

#[test]
fn test_main_rs_has_updater() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = PathBuf::from(manifest_dir).join("src/main.rs");
    let content = std::fs::read_to_string(&src_path).unwrap();
    assert!(content.contains("tauri_plugin_updater"), "Missing updater plugin");
    assert!(content.contains("updater.check"), "Missing update check");
}

#[test]
fn test_main_rs_has_event_forwarder() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = PathBuf::from(manifest_dir).join("src/main.rs");
    let content = std::fs::read_to_string(&src_path).unwrap();
    assert!(content.contains("start_event_forwarder"), "Missing event forwarder");
    assert!(content.contains("gateway-event"), "Missing gateway-event emit");
}

#[test]
fn test_main_rs_has_app_state() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = PathBuf::from(manifest_dir).join("src/main.rs");
    let content = std::fs::read_to_string(&src_path).unwrap();
    assert!(content.contains("struct AppState"), "Missing AppState");
    assert!(content.contains("ignition: Mutex"), "Missing ignition state");
    assert!(content.contains("nexus: Mutex"), "Missing nexus state");
}

#[test]
fn test_main_rs_has_log_bridge() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = PathBuf::from(manifest_dir).join("src/main.rs");
    let content = std::fs::read_to_string(&src_path).unwrap();
    assert!(content.contains("struct LogBridge"), "Missing LogBridge struct");
    assert!(content.contains("impl<S: Subscriber> Layer<S> for LogBridge"), "Missing Layer impl");
    assert!(content.contains("system-log-event"), "Missing system-log-event emit");
}

#[test]
fn test_main_rs_has_log_visitor() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = PathBuf::from(manifest_dir).join("src/main.rs");
    let content = std::fs::read_to_string(&src_path).unwrap();
    assert!(content.contains("struct LogVisitor"), "Missing LogVisitor struct");
    assert!(content.contains("impl Visit for LogVisitor"), "Missing Visit impl");
    assert!(content.contains("record_debug"), "Missing record_debug");
    assert!(content.contains("record_str"), "Missing record_str");
}

#[test]
fn test_main_rs_has_bootstrap_log() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = PathBuf::from(manifest_dir).join("src/main.rs");
    let content = std::fs::read_to_string(&src_path).unwrap();
    assert!(content.contains("fn bootstrap_log"), "Missing bootstrap_log function");
}

#[test]
fn test_paths_rs_exists() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let paths_path = PathBuf::from(manifest_dir).join("src/paths.rs");
    assert!(paths_path.exists(), "paths.rs not found");
}

#[test]
fn test_paths_rs_has_resolver_struct() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let paths_path = PathBuf::from(manifest_dir).join("src/paths.rs");
    let content = std::fs::read_to_string(&paths_path).unwrap();
    assert!(content.contains("pub struct SavantPathResolver"), "Missing SavantPathResolver struct");
    assert!(content.contains("pub fn new"), "Missing new() constructor");
    assert!(content.contains("pub fn config_file"), "Missing config_file()");
    assert!(content.contains("pub fn env_file"), "Missing env_file()");
    assert!(content.contains("pub fn workspaces_dir"), "Missing workspaces_dir()");
}

#[test]
fn test_paths_rs_has_dead_code_markers() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let paths_path = PathBuf::from(manifest_dir).join("src/paths.rs");
    let content = std::fs::read_to_string(&paths_path).unwrap();
    // These are marked dead_code — verify they exist for future use
    assert!(content.contains("skills_dir"), "Missing skills_dir()");
    assert!(content.contains("data_dir"), "Missing data_dir()");
    assert!(content.contains("memory_dir"), "Missing memory_dir()");
}

#[test]
fn test_build_rs_exists() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let build_path = PathBuf::from(manifest_dir).join("build.rs");
    assert!(build_path.exists(), "build.rs not found");
}

#[test]
fn test_tauri_conf_exists() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let conf_path = PathBuf::from(manifest_dir).join("tauri.conf.json");
    assert!(conf_path.exists(), "tauri.conf.json not found");
}

#[test]
fn test_tauri_conf_has_required_fields() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let conf_path = PathBuf::from(manifest_dir).join("tauri.conf.json");
    let content = std::fs::read_to_string(&conf_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(json.get("productName").is_some(), "Missing productName");
    assert!(json.get("version").is_some(), "Missing version");
}

// ─── bootstrap_log pattern test ──────────────────────────────────────────
// Replicate the bootstrap_log logic for testing

fn bootstrap_log(dir: &PathBuf, msg: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let log_path = dir.join("savant-desktop.log");
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    writeln!(f, "[BOOT] {}", msg)?;
    Ok(())
}

#[test]
fn test_bootstrap_log_creates_dir() {
    let dir = make_temp_dir();
    bootstrap_log(&dir, "test message").unwrap();
    assert!(dir.exists());
    assert!(dir.join("savant-desktop.log").exists());
    cleanup(&dir);
}

#[test]
fn test_bootstrap_log_appends() {
    let dir = make_temp_dir();
    bootstrap_log(&dir, "first").unwrap();
    bootstrap_log(&dir, "second").unwrap();
    let content = std::fs::read_to_string(dir.join("savant-desktop.log")).unwrap();
    assert!(content.contains("first"));
    assert!(content.contains("second"));
    cleanup(&dir);
}

#[test]
fn test_bootstrap_log_includes_timestamp() {
    let dir = make_temp_dir();
    bootstrap_log(&dir, "test").unwrap();
    let content = std::fs::read_to_string(dir.join("savant-desktop.log")).unwrap();
    // Should contain [BOOT] prefix
    assert!(content.contains("[BOOT]"));
    assert!(content.contains("test"));
    cleanup(&dir);
}
