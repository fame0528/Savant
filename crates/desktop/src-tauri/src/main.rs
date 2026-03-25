#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use savant_agent::orchestration::ignition::{IgnitionService, SwarmIgnition};
use savant_core::bus::NexusBridge;
use serde_json;
use std::sync::Arc;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, State,
};
use tokio::sync::Mutex;
use tracing::{error, field::Visit, info, Subscriber};
use tracing_subscriber::layer::{Context, Layer};

struct LogBridge {
    app_handle: tauri::AppHandle,
}

impl<S: Subscriber> Layer<S> for LogBridge {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = LogVisitor::default();
        event.record(&mut visitor);

        let msg = format!("[{}] {}", event.metadata().level(), visitor.message);
        if let Err(e) = self.app_handle.emit("system-log-event", msg) {
            tracing::debug!("[desktop] Failed to emit system-log-event: {}", e);
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
}

#[tauri::command]
async fn ignite_swarm(state: State<'_, AppState>, app_handle: AppHandle) -> Result<String, String> {
    info!("Tauri: Initiating Swarm Ignition...");

    let mut lock = state.ignition.lock().await;
    if lock.is_some() {
        let msg = "Swarm is already active";
        info!("Tauri: {}", msg);
        if let Err(e) = app_handle.emit("system-log-event", msg) {
            tracing::debug!("[desktop] Failed to emit system-log-event: {}", e);
        }
        return Ok(msg.into());
    }

    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let project_root = exe_path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent());

    let config_path = project_root
        .map(|root| root.join("config").join("savant.toml"))
        .filter(|p| p.exists())
        .map(|p| p.to_string_lossy().into_owned());

    match IgnitionService::ignite(config_path.as_deref()).await {
        Ok(ignition) => {
            let ignition_arc = Arc::new(ignition);
            *lock = Some(Arc::clone(&ignition_arc));

            let nexus = ignition_arc.nexus.clone();
            *state.nexus.lock().await = Some(nexus.clone());

            start_event_forwarder(Arc::clone(&ignition_arc), app_handle.clone()).await;

            let msg = "Swarm Ignition Sequence Complete";
            info!("Tauri: {}", msg);
            if let Err(e) = app_handle.emit("system-log-event", msg) {
                tracing::debug!("[desktop] Failed to emit system-log-event: {}", e);
            }
            Ok(msg.into())
        }
        Err(e) => {
            let msg = format!("Ignition Failed: {}", e);
            error!("Tauri: {}", msg);
            if let Err(e) = app_handle.emit("system-log-event", &msg) {
                tracing::debug!("[desktop] Failed to emit system-log-event: {}", e);
            }
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
    dotenvy::dotenv().ok();

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState {
            ignition: Mutex::new(None),
            nexus: Mutex::new(None),
        })
        .setup(|app| {
            let handle = app.handle();

            // Defer tracing init + status emission until window is ready
            let app_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                // Wait for webview to load
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;

                // Initialize tracing
                let bridge = LogBridge {
                    app_handle: app_handle.clone(),
                };
                use tracing_subscriber::layer::SubscriberExt;
                use tracing_subscriber::util::SubscriberInitExt;
                if let Err(e) = tracing_subscriber::registry()
                    .with(tracing_subscriber::EnvFilter::new(
                        "info,savant_desktop=debug,savant_agent=debug",
                    ))
                    .with(tracing_subscriber::fmt::layer())
                    .with(bridge)
                    .try_init()
                {
                    tracing::debug!("[desktop] Tracing subscriber init failed (may already be initialized): {}", e);
                }

                // Emit initial status
                let version = app_handle
                    .config()
                    .version
                    .clone()
                    .unwrap_or_else(|| "0.0.0".to_string());
                if let Err(e) = app_handle.emit(
                    "system-log-event",
                    format!("Savant Desktop v{} starting", version),
                ) {
                    tracing::debug!("[desktop] Failed to emit system-log-event: {}", e);
                }
                if let Err(e) = app_handle.emit("system-log-event", "Initializing...") {
                    tracing::debug!("[desktop] Failed to emit system-log-event: {}", e);
                }
            });

            // System tray
            let show_item = MenuItemBuilder::with_id("show", "Show Dashboard").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quit Savant").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&show_item)
                .separator()
                .item(&quit_item)
                .build()?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon()
                    .cloned()
                    .unwrap_or_else(|| {
                        tracing::warn!("default_window_icon() returned None, loading from bundled icon");
                        tauri::image::Image::from_path("icons/icon.ico")
                            .expect("CRITICAL: Brand icon 'icons/icon.ico' not found. Branding must always be visible.")
                    }))
                .menu(&menu)
                .tooltip("Savant Swarm")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            if let Err(e) = window.show() {
                                tracing::debug!("[desktop] Failed to show window: {}", e);
                            }
                            if let Err(e) = window.set_focus() {
                                tracing::debug!("[desktop] Failed to focus window: {}", e);
                            }
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ignite_swarm,
            get_swarm_status,
            get_version
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            tracing::error!("Tauri runtime error: {}", e);
            std::process::exit(1);
        });
}
