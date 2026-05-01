#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod paths;

use paths::SavantPathResolver;
use savant_agent::orchestration::ignition::{IgnitionService, SwarmIgnition};
use savant_browser::{BrowserConfig, BrowserEngine};
use savant_core::bus::NexusBridge;
use serde_json;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, State,
};
use tauri_plugin_updater::UpdaterExt;
use tokio::sync::Mutex;
use tracing::{error, field::Visit, info, warn, Subscriber};
use tracing_subscriber::layer::{Context, Layer};

/// LogBridge: emits tracing events to both the Tauri frontend (splash screen)
/// AND a log file for post-mortem debugging.
struct LogBridge {
    app_handle: tauri::AppHandle,
    log_file: std::sync::Arc<std::sync::Mutex<std::fs::File>>,
}

impl LogBridge {
    fn new(app_handle: AppHandle) -> Self {
        // Create logs directory next to the exe
        let log_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("logs")))
            .unwrap_or_else(|| std::path::PathBuf::from("logs"));
        let _ = std::fs::create_dir_all(&log_dir);
        let log_path = log_dir.join("savant-desktop.log");

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .unwrap_or_else(|_e| {
                // No eprintln! in release — it spawns a console window on Windows
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("savant-desktop.log")
                    .expect("CRITICAL: Cannot create any log file")
            });

        Self {
            app_handle,
            log_file: std::sync::Arc::new(std::sync::Mutex::new(file)),
        }
    }
}

impl<S: Subscriber> Layer<S> for LogBridge {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = LogVisitor::default();
        event.record(&mut visitor);

        let level = event.metadata().level();
        let msg = format!("[{}] {}", level, visitor.message);

        // Write to log file
        if let Ok(mut f) = self.log_file.lock() {
            let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let _ = writeln!(f, "{} {}", ts, msg);
            let _ = f.flush();
        }

        // Emit to frontend (splash screen)
        if let Err(e) = self.app_handle.emit("system-log-event", &msg) {
            eprintln!("[desktop] Failed to emit system-log-event: {}", e);
        }
    }
}

#[derive(Default)]
struct LogVisitor {
    message: String,
}

impl Visit for LogVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }
}

struct AppState {
    ignition: Mutex<Option<Arc<SwarmIgnition>>>,
    nexus: Mutex<Option<Arc<NexusBridge>>>,
    browser: Mutex<Option<Arc<BrowserEngine>>>,
}

#[tauri::command]
async fn ignite_swarm(state: State<'_, AppState>, app_handle: AppHandle) -> Result<String, String> {
    info!("=== SWARM IGNITION STARTED ===");

    let mut lock = state.ignition.lock().await;
    if lock.is_some() {
        let msg = "Swarm is already active";
        info!("{}", msg);
        let _ = app_handle.emit("system-log-event", msg);
        return Ok(msg.into());
    }

    // Step 1: Resolve paths using SavantPathResolver (set up in main.rs setup hook)
    info!("[1/5] Resolving project paths...");
    let _ = app_handle.emit("system-log-event", "[1/5] Resolving project paths...");

    let resolver = match app_handle.try_state::<SavantPathResolver>() {
        Some(r) => r,
        None => {
            let msg = "CRITICAL: SavantPathResolver not initialized".to_string();
            error!("{}", msg);
            let _ = app_handle.emit("system-log-event", &msg);
            return Err(msg);
        }
    };

    let config_path = resolver.config_file();
    info!("  config_file: {:?}", config_path);
    info!("  env_file: {:?}", resolver.env_file());
    info!("  workspaces_dir: {:?}", resolver.workspaces_dir());
    let _ = app_handle.emit("system-log-event", &format!("  config: {:?}", config_path));

    let config_path_str = if config_path.exists() {
        Some(config_path.to_string_lossy().into_owned())
    } else {
        warn!("  config not found at {:?} — using defaults", config_path);
        let _ = app_handle.emit("system-log-event", "  config: NOT FOUND (using defaults)");
        None
    };

    // Set SAVANT_PROJECT_ROOT for Config::load_from to anchor project root
    std::env::set_var("SAVANT_PROJECT_ROOT", &resolver.base_data_path);
    info!(
        "  SAVANT_PROJECT_ROOT set to: {:?}",
        resolver.base_data_path
    );
    let _ = app_handle.emit(
        "system-log-event",
        &format!("  SAVANT_PROJECT_ROOT={:?}", resolver.base_data_path),
    );

    // Step 2: Check environment
    info!("[2/5] Checking environment...");
    let _ = app_handle.emit("system-log-event", "[2/5] Checking environment...");

    let has_or_key = std::env::var("OR_MASTER_KEY").is_ok();
    let dev_mode = std::env::var("SAVANT_DEV_MODE").is_ok();
    info!(
        "  OR_MASTER_KEY: {}",
        if has_or_key { "SET" } else { "NOT SET" }
    );
    info!("  SAVANT_DEV_MODE: {}", dev_mode);
    let _ = app_handle.emit(
        "system-log-event",
        &format!(
            "  OR_MASTER_KEY: {} | DEV_MODE: {}",
            if has_or_key { "SET" } else { "NOT SET" },
            dev_mode
        ),
    );

    // Step 3: Ignite
    info!("[3/5] Calling IgnitionService::ignite()...");
    let _ = app_handle.emit("system-log-event", "[3/5] Starting IgnitionService...");

    match IgnitionService::ignite(config_path_str.as_deref()).await {
        Ok(ignition) => {
            info!("[3/5] IgnitionService returned OK");

            // Step 4: Store state
            info!("[4/5] Storing ignition state...");
            let _ = app_handle.emit("system-log-event", "[4/5] Storing swarm state...");

            let ignition_arc = Arc::new(ignition);
            *lock = Some(Arc::clone(&ignition_arc));

            let nexus = ignition_arc.nexus.clone();
            *state.nexus.lock().await = Some(nexus.clone());

            // Step 5: Start event forwarder
            info!("[5/5] Starting event forwarder...");
            let _ = app_handle.emit("system-log-event", "[5/5] Starting event forwarder...");
            start_event_forwarder(Arc::clone(&ignition_arc), app_handle.clone()).await;

            let msg = "Swarm Ignition Sequence Complete";
            info!("=== {} ===", msg);
            let _ = app_handle.emit("system-log-event", msg);
            Ok(msg.into())
        }
        Err(e) => {
            let msg = format!("IGNITION FAILED: {}", e);
            error!("{}", msg);
            let _ = app_handle.emit("system-log-event", &msg);

            // Also log the full error chain
            let full_err = format!("  Error details: {:?}", e);
            error!("{}", full_err);
            let _ = app_handle.emit("system-log-event", &full_err);

            Err(msg)
        }
    }
}

#[tauri::command]
async fn get_swarm_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let lock = state.ignition.lock().await;
    if let Some(ref ignition) = *lock {
        Ok(serde_json::json!({
            "status": "ACTIVE",
            "agent_name": ignition.config.ai.model,
            "agents_online": ignition.swarm.active_agents_count().await,
        }))
    } else {
        Ok(serde_json::json!({
            "status": "IDLE"
        }))
    }
}

#[tauri::command]
async fn get_version(app_handle: AppHandle) -> Result<String, String> {
    let version = app_handle
        .config()
        .version
        .clone()
        .unwrap_or_else(|| "0.0.0".to_string());
    Ok(version)
}

// --- Browser Commands ---

/// Shows the browser window.
#[tauri::command]
async fn show_browser(app_handle: AppHandle) -> Result<String, String> {
    if let Some(window) = app_handle.get_webview_window("browser") {
        window.show().map_err(|e| format!("Failed to show browser window: {}", e))?;
        window.set_focus().map_err(|e| format!("Failed to focus browser window: {}", e))?;
        info!("[browser] Browser window shown");
        Ok("Browser window shown".to_string())
    } else {
        Err("Browser window not found".to_string())
    }
}

/// Hides the browser window.
#[tauri::command]
async fn hide_browser(app_handle: AppHandle) -> Result<String, String> {
    if let Some(window) = app_handle.get_webview_window("browser") {
        window.hide().map_err(|e| format!("Failed to hide browser window: {}", e))?;
        info!("[browser] Browser window hidden");
        Ok("Browser window hidden".to_string())
    } else {
        Err("Browser window not found".to_string())
    }
}

/// Gets all open browser tabs.
#[tauri::command]
async fn browser_get_tabs(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let lock = state.browser.lock().await;
    match &*lock {
        Some(engine) => {
            let tabs = engine.list_tabs();
            let tab_json: Vec<serde_json::Value> = tabs
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "id": t.id.0,
                        "url": t.url,
                        "title": t.title,
                        "loading": t.loading,
                        "agent_name": t.agent_name,
                    })
                })
                .collect();
            Ok(serde_json::json!(tab_json))
        }
        None => Ok(serde_json::json!([])),
    }
}

/// Navigates the active browser tab to a URL.
#[tauri::command]
async fn browser_navigate(
    state: State<'_, AppState>,
    app_handle: AppHandle,
    url: String,
) -> Result<String, String> {
    let lock = state.browser.lock().await;
    let engine = lock.as_ref().ok_or("Browser engine not initialized")?;

    // Navigate the active tab, or create one if none exists.
    let tab_id = if let Some(active) = engine.active_tab() {
        engine.navigate_tab(&active.id, &url).map_err(|e| e.to_string())?;
        active.id
    } else {
        let tab = engine.create_tab(url.clone(), None).map_err(|e| e.to_string())?;
        tab.id
    };

    // Tell the browser window to navigate via JS injection.
    if let Some(window) = app_handle.get_webview_window("browser") {
        let js = format!("window.location.href = {};", serde_json::to_string(&url).unwrap_or_else(|_| "\"\"".to_string()));
        if let Err(e) = window.eval(&js) {
            warn!("[browser] Navigation eval failed: {}", e);
        }
    }

    info!("[browser] Navigated to {}", url);
    Ok(format!("Navigated tab {} to {}", tab_id.0, url))
}

/// Extracts text content from the current browser page via JS injection.
#[tauri::command]
async fn browser_get_content(app_handle: AppHandle) -> Result<String, String> {
    let window = app_handle
        .get_webview_window("browser")
        .ok_or("Browser window not found")?;

    // Tauri v2 eval executes JS but does not return the result synchronously.
    // For full content extraction, agents should use the BrowserTool which
    // communicates with the browser engine via the Rust backend.
    // This command serves as a simple smoke test for window responsiveness.
    let _ = window.eval("document.body ? document.body.innerText : 'No content'");
    Ok("Content extraction available via BrowserTool (agent-side)".to_string())
}

/// Goes back in browser history.
#[tauri::command]
async fn browser_go_back(app_handle: AppHandle) -> Result<String, String> {
    let window = app_handle
        .get_webview_window("browser")
        .ok_or("Browser window not found")?;

    window.eval("window.history.back();")
        .map_err(|e| format!("Failed to go back: {}", e))?;
    Ok("Going back".to_string())
}

/// Goes forward in browser history.
#[tauri::command]
async fn browser_go_forward(app_handle: AppHandle) -> Result<String, String> {
    let window = app_handle
        .get_webview_window("browser")
        .ok_or("Browser window not found")?;

    window.eval("window.history.forward();")
        .map_err(|e| format!("Failed to go forward: {}", e))?;
    Ok("Going forward".to_string())
}

/// Reloads the current browser page.
#[tauri::command]
async fn browser_reload(app_handle: AppHandle) -> Result<String, String> {
    let window = app_handle
        .get_webview_window("browser")
        .ok_or("Browser window not found")?;

    window.eval("window.location.reload();")
        .map_err(|e| format!("Failed to reload: {}", e))?;
    Ok("Reloading".to_string())
}

/// Creates a new browser tab with the given URL.
#[tauri::command]
async fn browser_new_tab(
    state: State<'_, AppState>,
    url: String,
) -> Result<String, String> {
    let lock = state.browser.lock().await;
    let engine = lock.as_ref().ok_or("Browser engine not initialized")?;

    let tab = engine.create_tab(url.clone(), None).map_err(|e| e.to_string())?;
    Ok(format!("Created tab {}", tab.id.0))
}

/// Closes a browser tab by ID.
#[tauri::command]
async fn browser_close_tab(
    state: State<'_, AppState>,
    tab_id: String,
) -> Result<String, String> {
    let lock = state.browser.lock().await;
    let engine = lock.as_ref().ok_or("Browser engine not initialized")?;

    let tid = savant_browser::types::TabId(tab_id);
    engine.close_tab(&tid).map_err(|e| e.to_string())?;
    Ok(format!("Closed tab {}", tid.0))
}

/// Switches the active browser tab.
#[tauri::command]
async fn browser_switch_tab(
    state: State<'_, AppState>,
    tab_id: String,
) -> Result<String, String> {
    let lock = state.browser.lock().await;
    let engine = lock.as_ref().ok_or("Browser engine not initialized")?;

    let tid = savant_browser::types::TabId(tab_id);
    engine.switch_tab(&tid).map_err(|e| e.to_string())?;
    Ok(format!("Switched to tab {}", tid.0))
}

/// Write a line directly to the log file (before tracing is initialized)
fn bootstrap_log(msg: &str) {
    let log_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("logs")))
        .unwrap_or_else(|| std::path::PathBuf::from("logs"));
    let _ = std::fs::create_dir_all(&log_dir);
    let log_path = log_dir.join("savant-desktop.log");
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&log_path) {
        let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let _ = writeln!(f, "{} [BOOT] {}", ts, msg);
    }
}

async fn start_event_forwarder(ignition: Arc<SwarmIgnition>, app_handle: AppHandle) {
    let nexus = ignition.nexus.clone();
    tokio::spawn(async move {
        let (mut event_rx, _log_rx) = nexus.subscribe().await;
        while let Ok(event) = event_rx.recv().await {
            if let Ok(json_str) = serde_json::to_string(&event) {
                if let Err(e) = app_handle.emit("gateway-event", format!("EVENT:{}", json_str)) {
                    tracing::debug!("[desktop] Failed to emit gateway-event: {}", e);
                }
            }
        }
    });
}

fn main() {
    bootstrap_log("Savant Desktop starting...");
    // .env loaded in setup hook via SavantPathResolver (not from CWD)
    bootstrap_log("Starting Tauri builder...");

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState {
            ignition: Mutex::new(None),
            nexus: Mutex::new(None),
            browser: Mutex::new(None),
        })
        .setup(|app| {
            bootstrap_log("Tauri setup() called");
            let handle = app.handle();

            // Initialize tracing immediately (not deferred)
            let app_handle = handle.clone();
            let bridge = LogBridge::new(app_handle.clone());
            use tracing_subscriber::layer::SubscriberExt;
            use tracing_subscriber::util::SubscriberInitExt;
            if let Err(e) = tracing_subscriber::registry()
                .with(tracing_subscriber::EnvFilter::new(
                    "info,savant_desktop=debug,savant_agent=debug",
                ))
                .with({
                    #[cfg(debug_assertions)]
                    {
                        tracing_subscriber::fmt::layer()
                    }
                    #[cfg(not(debug_assertions))]
                    {
                        tracing_subscriber::fmt::layer().with_writer(std::io::sink)
                    }
                })
                .with(bridge)
                .try_init()
            {
                // No eprintln! in release — it spawns a console window on Windows
                let _ = e;
            }

            bootstrap_log("Tracing initialized");

            // Initialize path resolver (dev vs installed mode)
            let handle2 = handle.clone();
            match SavantPathResolver::new(&handle2) {
                Ok(resolver) => {
                    info!("  data_path: {:?}", resolver.base_data_path);
                    info!("  config_file: {:?}", resolver.config_file());
                    info!("  env_file: {:?}", resolver.env_file());
                    bootstrap_log(&format!("Path resolver: {:?}", resolver.base_data_path));
                    app.manage(resolver);
                }
                Err(e) => {
                    bootstrap_log(&format!("PATH RESOLVER FAILED: {}", e));
                    return Err(e.into());
                }
            }

            // Load .env from the resolved path (not CWD)
            if let Some(resolver) = app.try_state::<SavantPathResolver>() {
                let env_path = resolver.env_file();
                if env_path.exists() {
                    if let Err(e) = dotenvy::from_path(&env_path) {
                        tracing::warn!("[desktop] Failed to load .env from {:?}: {}", env_path, e);
                    } else {
                        info!("[desktop] .env loaded from {:?}", env_path);
                    }
                } else {
                    info!(
                        "[desktop] No .env found at {:?} — will prompt in dashboard",
                        env_path
                    );
                }
            }

            // Initialize BrowserEngine
            if let Some(resolver) = app.try_state::<SavantPathResolver>() {
                match BrowserEngine::new(&resolver.base_data_path, BrowserConfig::default()) {
                    Ok(engine) => {
                        if let Some(app_state) = app.try_state::<AppState>() {
                            let mut browser_lock = app_state.browser.blocking_lock();
                            *browser_lock = Some(Arc::clone(&engine));
                            info!(
                                "[browser] BrowserEngine initialized with {} max tabs",
                                engine.config().max_tabs
                            );
                        }
                    }
                    Err(e) => {
                        warn!("[browser] Failed to initialize BrowserEngine: {}", e);
                    }
                }
            }

            // Emit startup status after a short delay for webview
            let app_handle2 = handle.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                let version = app_handle2
                    .config()
                    .version
                    .clone()
                    .unwrap_or_else(|| "0.0.0".to_string());
                let _ = app_handle2.emit(
                    "system-log-event",
                    format!("Savant Desktop v{} — Ready", version),
                );
                info!("Desktop v{} initialized", version);
            });

            // Check for auto-updates (non-blocking)
            let update_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                info!("[updater] Checking for updates...");
                match update_handle.updater() {
                    Ok(updater) => match updater.check().await {
                        Ok(Some(update)) => {
                            info!("[updater] Update available: {}", update.version);
                            let _ = update_handle.emit(
                                "system-log-event",
                                format!("Update available: v{}", update.version),
                            );
                            match update.download_and_install(|_, _| {}, || {}).await {
                                Ok(_) => {
                                    info!("[updater] Update installed successfully");
                                }
                                Err(e) => {
                                    warn!("[updater] Failed to install update: {}", e);
                                }
                            }
                        }
                        Ok(None) => {
                            info!("[updater] No update available");
                        }
                        Err(e) => {
                            info!("[updater] Update check failed: {}", e);
                        }
                    },
                    Err(e) => {
                        info!("[updater] Updater not available: {}", e);
                    }
                }
            });

            // System tray
            let show_item = MenuItemBuilder::with_id("show", "Show Dashboard").build(app)?;
            let show_browser_item = MenuItemBuilder::with_id("show_browser", "Show Browser").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quit Savant").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&show_item)
                .item(&show_browser_item)
                .separator()
                .item(&quit_item)
                .build()?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().cloned().unwrap_or_else(|| {
                    warn!("default_window_icon() returned None");
                    tauri::image::Image::from_path("icons/icon.ico")
                        .expect("CRITICAL: Brand icon 'icons/icon.ico' not found.")
                }))
                .menu(&menu)
                .tooltip("Savant Swarm")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "show_browser" => {
                        if let Some(window) = app.get_webview_window("browser") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            bootstrap_log("Tauri setup() complete");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ignite_swarm,
            get_swarm_status,
            get_version,
            show_browser,
            hide_browser,
            browser_get_tabs,
            browser_navigate,
            browser_get_content,
            browser_go_back,
            browser_go_forward,
            browser_reload,
            browser_new_tab,
            browser_close_tab,
            browser_switch_tab,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            bootstrap_log(&format!("Tauri runtime error: {}", e));
            eprintln!("Tauri runtime error: {}", e);
            std::process::exit(1);
        });
}
