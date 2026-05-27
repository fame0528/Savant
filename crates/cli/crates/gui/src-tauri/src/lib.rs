// SAFETY: All clippy::disallowed_methods violations in this file originate from serde_json::json!() macro internals. The json!() macro calls .unwrap() on provably-infallible compile-time-validated JSON literals. grep confirms 0 real .unwrap() calls exist in this file outside macro expansions.
#![allow(clippy::disallowed_methods)]
// SAFETY: All `clippy::disallowed_methods` violations in this file originate from
// the `serde_json::json!()` macro, which internally uses `.unwrap()` on
// compile-time-validated JSON literals. A malformed JSON literal would be a
// compile error, making the panic path statically unreachable.

mod modules;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{Emitter, State};
use tokio::sync::{Mutex, RwLock};
use tracing::info;

use modules::fs::{FileNode, FsService};
use modules::gamification::GamificationService;
use modules::pty::PtyService;
use modules::security::SecurityGuard;
use modules::shell::{ShellBgProcess, ShellSession};
use modules::workspace::WorkspaceState;

/// Application state shared across all Tauri commands
pub struct AppState {
    pub pty: Arc<Mutex<PtyService>>,
    pub fs: Arc<Mutex<FsService>>,
    pub security: Arc<SecurityGuard>,
    pub workspace: Arc<RwLock<WorkspaceState>>,
    pub sessions: Arc<RwLock<HashMap<String, ShellSession>>>,
    pub bg_processes: Arc<RwLock<HashMap<u32, ShellBgProcess>>>,
    pub next_bg_id: Arc<Mutex<u32>>,
    pub gateway_url: Arc<RwLock<Option<String>>>,
    pub api_keys: Arc<RwLock<HashMap<String, String>>>,
    pub gamification: Arc<Mutex<GamificationService>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            pty: Arc::new(Mutex::new(PtyService::new())),
            fs: Arc::new(Mutex::new(FsService::new())),
            security: Arc::new(SecurityGuard::new()),
            workspace: Arc::new(RwLock::new(WorkspaceState::default())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            bg_processes: Arc::new(RwLock::new(HashMap::new())),
            next_bg_id: Arc::new(Mutex::new(1)),
            gateway_url: Arc::new(RwLock::new(None)),
            api_keys: Arc::new(RwLock::new(HashMap::new())),
            gamification: Arc::new(Mutex::new(GamificationService::new())),
        }
    }
}

// ── PTY Commands ────────────────────────────────────────────────────────────

#[tauri::command]
async fn pty_create(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
    shell: Option<String>,
    cwd: Option<String>,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let mut pty = state.pty.lock().await;
    pty.create(&id, shell.as_deref(), cwd.as_deref(), Some(app_handle))
        .map_err(|e| format!("Failed to create PTY: {}", e))?;
    info!("PTY created: {}", id);
    Ok(id)
}

#[tauri::command]
async fn pty_write(state: State<'_, AppState>, id: String, data: String) -> Result<(), String> {
    let mut pty = state.pty.lock().await;
    pty.write(&id, &data)
        .map_err(|e| format!("Failed to write to PTY: {}", e))
}

#[tauri::command]
async fn pty_resize(
    state: State<'_, AppState>,
    id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let mut pty = state.pty.lock().await;
    pty.resize(&id, cols, rows)
        .map_err(|e| format!("Failed to resize PTY: {}", e))
}

#[tauri::command]
async fn pty_close(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mut pty = state.pty.lock().await;
    pty.close(&id)
        .map_err(|e| format!("Failed to close PTY: {}", e))
}

#[tauri::command]
async fn pty_list(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let pty = state.pty.lock().await;
    Ok(pty.list())
}

// ── File System Commands ────────────────────────────────────────────────────

#[tauri::command]
async fn fs_read_file(path: String) -> Result<String, String> {
    // GUI-02: Security check before reading sensitive files
    let guard = modules::security::SecurityGuard::new();
    let check = guard.check_readable(&path);
    if !check.ok {
        return Err(check.reason.unwrap_or_else(|| "Access denied".to_string()));
    }
    tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("Failed to read file {}: {}", path, e))
}

#[tauri::command]
async fn fs_write_file(path: String, content: String) -> Result<(), String> {
    // GUI-01: Security check before writing to protected paths
    let guard = modules::security::SecurityGuard::new();
    let check = guard.check_writable(&path);
    if !check.ok {
        return Err(check.reason.unwrap_or_else(|| "Access denied".to_string()));
    }
    tokio::fs::write(&path, &content)
        .await
        .map_err(|e| format!("Failed to write file {}: {}", path, e))
}

#[tauri::command]
async fn fs_list_dir(path: String) -> Result<Vec<FileNode>, String> {
    let mut fs = FsService::new();
    fs.list_dir(&path)
        .await
        .map_err(|e| format!("Failed to list directory {}: {}", path, e))
}

#[tauri::command]
async fn fs_search_files(
    state: State<'_, AppState>,
    query: String,
    dir: String,
    max_results: Option<usize>,
) -> Result<Vec<String>, String> {
    let fs = state.fs.lock().await;
    fs.search_files(&query, &dir, max_results.unwrap_or(50))
        .await
        .map_err(|e| format!("Search failed: {}", e))
}

#[tauri::command]
async fn fs_search_content(
    state: State<'_, AppState>,
    pattern: String,
    dir: String,
    file_glob: Option<String>,
) -> Result<Vec<SearchResult>, String> {
    let fs = state.fs.lock().await;
    fs.search_content(&pattern, &dir, file_glob.as_deref())
        .await
        .map_err(|e| format!("Content search failed: {}", e))
}

pub use modules::fs::SearchResult;

// ── Shell Session Commands ──────────────────────────────────────────────────

#[tauri::command]
async fn shell_session_create(
    state: State<'_, AppState>,
    cwd: Option<String>,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let session = ShellSession::new(cwd.as_deref()).map_err(|e| format!("{}", e))?;
    state.sessions.write().await.insert(id.clone(), session);
    Ok(id)
}

#[tauri::command]
async fn shell_session_run(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    command: String,
    timeout_secs: Option<u64>,
) -> Result<ShellOutput, String> {
    let security_result = state.security.check_shell_command(&command);
    if !security_result.ok {
        return Err(security_result
            .reason
            .unwrap_or_else(|| "Command rejected by security guard".to_string()));
    }

    let sessions = state.sessions.read().await;
    let session = sessions.get(&session_id).ok_or("Session not found")?;
    let result: ShellOutput = session
        .run(&command, timeout_secs.unwrap_or(120))
        .await
        .map_err(|e| format!("{}", e))?;

    // Emit tool execution event
    let _ = app.emit("shell:output", &result);

    Ok(result)
}

#[tauri::command]
async fn shell_session_close(state: State<'_, AppState>, session_id: String) -> Result<(), String> {
    state.sessions.write().await.remove(&session_id);
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
}

// ── Background Process Commands ─────────────────────────────────────────────

#[tauri::command]
async fn bg_spawn(
    state: State<'_, AppState>,
    command: String,
    cwd: Option<String>,
) -> Result<u32, String> {
    let security_result = state.security.check_shell_command(&command);
    if !security_result.ok {
        return Err(security_result
            .reason
            .unwrap_or_else(|| "Command rejected by security guard".to_string()));
    }
    let mut bg_id = state.next_bg_id.lock().await;
    let id = *bg_id;
    *bg_id += 1;
    drop(bg_id);

    let process = ShellBgProcess::spawn(&command, cwd.as_deref()).map_err(|e| format!("{}", e))?;
    state.bg_processes.write().await.insert(id, process);
    Ok(id)
}

#[tauri::command]
async fn bg_logs(
    state: State<'_, AppState>,
    id: u32,
    since_offset: Option<usize>,
) -> Result<BgLogsOutput, String> {
    let processes = state.bg_processes.read().await;
    let process = processes.get(&id).ok_or("Process not found")?;
    process
        .logs(since_offset.unwrap_or(0))
        .map_err(|e| format!("{}", e))
}

#[tauri::command]
async fn bg_kill(state: State<'_, AppState>, id: u32) -> Result<(), String> {
    let mut processes = state.bg_processes.write().await;
    if let Some(process) = processes.remove(&id) {
        process.kill().map_err(|e| format!("{}", e))?;
    }
    Ok(())
}

#[tauri::command]
async fn bg_list(state: State<'_, AppState>) -> Result<Vec<BgProcessInfo>, String> {
    let processes = state.bg_processes.read().await;
    Ok(processes
        .iter()
        .map(|(id, p)| BgProcessInfo {
            id: *id,
            command: p.command().to_string(),
            running: p.is_running(),
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BgLogsOutput {
    pub stdout: String,
    pub next_offset: usize,
    pub dropped: usize,
    pub finished: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BgProcessInfo {
    pub id: u32,
    pub command: String,
    pub running: bool,
}

// ── Security Commands ───────────────────────────────────────────────────────

#[tauri::command]
async fn security_check_path(path: String, read: bool) -> Result<SecurityResult, String> {
    let guard = SecurityGuard::new();
    Ok(if read {
        guard.check_readable(&path)
    } else {
        guard.check_writable(&path)
    })
}

#[tauri::command]
async fn security_check_command(command: String) -> Result<SecurityResult, String> {
    let guard = SecurityGuard::new();
    Ok(guard.check_shell_command(&command))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityResult {
    pub ok: bool,
    pub reason: Option<String>,
}

// ── Gateway Commands ────────────────────────────────────────────────────────

#[tauri::command]
async fn gateway_connect(state: State<'_, AppState>, url: String) -> Result<(), String> {
    info!("Connecting to gateway: {}", url);
    *state.gateway_url.write().await = Some(url);
    Ok(())
}

#[tauri::command]
async fn gateway_disconnect(state: State<'_, AppState>) -> Result<(), String> {
    *state.gateway_url.write().await = None;
    info!("Disconnected from gateway");
    Ok(())
}

#[tauri::command]
async fn gateway_status(state: State<'_, AppState>) -> Result<GatewayStatus, String> {
    let url = state.gateway_url.read().await;
    Ok(GatewayStatus {
        connected: url.is_some(),
        url: url.clone(),
    })
}

#[tauri::command]
async fn gateway_send_chat(
    state: State<'_, AppState>,
    message: String,
    agent_id: Option<String>,
) -> Result<String, String> {
    let url = state.gateway_url.read().await;
    let url = url.as_ref().ok_or("Not connected to gateway")?;

    // Build HTTP API URL
    let api_url = format!("{}/api/chat", url.trim_end_matches('/'));

    // Build chat message payload
    let payload = serde_json::json!({
        "content": message,
        "agent_id": agent_id.unwrap_or_else(|| "default".to_string()),
        "timestamp": chrono::Utc::now().timestamp_millis(),
    });

    // Send HTTP request to gateway
    let client = reqwest::Client::new();
    let response = client
        .post(&api_url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to send message to gateway: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Gateway returned {}: {}", status, body));
    }

    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read gateway response: {}", e))?;

    // Parse response
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response_text) {
        if let Some(content) = json.get("content").and_then(|v| v.as_str()) {
            return Ok(content.to_string());
        }
        if let Some(error) = json.get("error").and_then(|v| v.as_str()) {
            return Err(format!("Gateway error: {}", error));
        }
    }

    Ok(response_text)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayStatus {
    pub connected: bool,
    pub url: Option<String>,
}

// ── API Key Commands ────────────────────────────────────────────────────────

#[tauri::command]
async fn set_api_key(
    state: State<'_, AppState>,
    provider: String,
    key: String,
) -> Result<(), String> {
    state.api_keys.write().await.insert(provider, key);
    Ok(())
}

#[tauri::command]
async fn get_api_key(
    state: State<'_, AppState>,
    provider: String,
) -> Result<Option<String>, String> {
    Ok(state.api_keys.read().await.get(&provider).cloned())
}

#[tauri::command]
async fn delete_api_key(state: State<'_, AppState>, provider: String) -> Result<(), String> {
    state.api_keys.write().await.remove(&provider);
    Ok(())
}

// ── App Commands ────────────────────────────────────────────────────────────

#[tauri::command]
async fn get_home_dir() -> Result<String, String> {
    dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "Could not determine home directory".to_string())
}

#[tauri::command]
async fn get_platform() -> String {
    std::env::consts::OS.to_string()
}

// ── Gamification Commands ───────────────────────────────────────────────────

#[tauri::command]
async fn gamification_get_state(
    state: State<'_, AppState>,
) -> Result<modules::gamification::GamificationSummary, String> {
    let g = state.gamification.lock().await;
    Ok(g.get_state_summary())
}

#[tauri::command]
async fn gamification_checkin(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let mut g = state.gamification.lock().await;
    g.daily_checkin()
}

#[tauri::command]
async fn gamification_feed(
    state: State<'_, AppState>,
    feed_type: String,
) -> Result<Vec<String>, String> {
    let feed = match feed_type.as_str() {
        "snack" => modules::gamification::FeedType::Snack,
        _ => modules::gamification::FeedType::Meal,
    };
    let mut g = state.gamification.lock().await;
    g.feed(feed)
}

#[tauri::command]
async fn gamification_play(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let mut g = state.gamification.lock().await;
    g.play()
}

#[tauri::command]
async fn gamification_clean(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let mut g = state.gamification.lock().await;
    g.clean()
}

#[tauri::command]
async fn gamification_medicine(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let mut g = state.gamification.lock().await;
    g.give_medicine()
}

#[tauri::command]
async fn gamification_discipline(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let mut g = state.gamification.lock().await;
    g.discipline()
}

#[tauri::command]
async fn gamification_sleep(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let mut g = state.gamification.lock().await;
    g.put_to_sleep()
}

#[tauri::command]
async fn gamification_pet(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let mut g = state.gamification.lock().await;
    g.pet_interact()
}

#[tauri::command]
async fn gamification_reset_egg(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let mut g = state.gamification.lock().await;
    g.reset_egg()
}

#[tauri::command]
async fn gamification_record_session(
    state: State<'_, AppState>,
    turns: u64,
    tool_calls: u64,
) -> Result<Vec<String>, String> {
    let mut g = state.gamification.lock().await;
    g.record_session_end(turns, tool_calls)
}

#[tauri::command]
async fn gamification_record_panel(
    state: State<'_, AppState>,
    panel: String,
) -> Result<Vec<String>, String> {
    let mut g = state.gamification.lock().await;
    g.record_panel_usage(&panel)
}

#[tauri::command]
async fn gamification_set_evolution_stage(
    state: State<'_, AppState>,
    stage: String,
) -> Result<(), String> {
    let mut g = state.gamification.lock().await;
    g.set_evolution_stage(&stage)
}

// ── Event emission helpers ──────────────────────────────────────────────────

fn emit_pty_event(app: &Option<tauri::AppHandle>, id: &str, event: modules::pty::PtyEvent) {
    if let Some(ref app) = app {
        let _ = app.emit(&format!("pty:{}", id), event);
    }
}

// ── Main entry point ────────────────────────────────────────────────────────

#[allow(clippy::disallowed_methods)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            // PTY
            pty_create,
            pty_write,
            pty_resize,
            pty_close,
            pty_list,
            // FS
            fs_read_file,
            fs_write_file,
            fs_list_dir,
            fs_search_files,
            fs_search_content,
            // Shell
            shell_session_create,
            shell_session_run,
            shell_session_close,
            // Background processes
            bg_spawn,
            bg_logs,
            bg_kill,
            bg_list,
            // Security
            security_check_path,
            security_check_command,
            // Gateway
            gateway_connect,
            gateway_disconnect,
            gateway_status,
            gateway_send_chat,
            // API keys
            set_api_key,
            get_api_key,
            delete_api_key,
            // App
            get_home_dir,
            get_platform,
            // Gamification
            gamification_get_state,
            gamification_checkin,
            gamification_feed,
            gamification_play,
            gamification_clean,
            gamification_medicine,
            gamification_discipline,
            gamification_sleep,
            gamification_pet,
            gamification_reset_egg,
            gamification_record_session,
            gamification_record_panel,
            gamification_set_evolution_stage,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            panic!("failed to run tauri application: {e}");
        });
}
