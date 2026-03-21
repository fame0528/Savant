#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use savant_agent::orchestration::ignition::{IgnitionService, SwarmIgnition};
use savant_core::bus::NexusBridge;
use serde_json;
use std::sync::Arc;
use tauri::{
    AppHandle, CustomMenuItem, Manager, State, SystemTray, SystemTrayMenu, SystemTrayMenuItem,
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
        let _ = self.app_handle.emit_all("system-log-event", msg);
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
    info!("🚀 Tauri: Initiating Swarm Ignition...");

    let mut lock = state.ignition.lock().await;
    if lock.is_some() {
        let msg = "Swarm is already active";
        info!("✅ Tauri: {}", msg);
        let _ = app_handle.emit_all("system-log-event", msg);
        return Ok(msg.into());
    }

    // Find project root relative to executable to locate config
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
            info!("✅ Tauri: {}", msg);
            let _ = app_handle.emit_all("system-log-event", msg);
            Ok(msg.into())
        }
        Err(e) => {
            let msg = format!("Ignition Failed: {}", e);
            error!("❌ Tauri: {}", msg);
            let _ = app_handle.emit_all("system-log-event", &msg);
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

async fn start_event_forwarder(ignition: Arc<SwarmIgnition>, app_handle: AppHandle) {
    let nexus = ignition.nexus.clone();
    tokio::spawn(async move {
        let (mut event_rx, _log_rx) = nexus.subscribe().await;
        while let Ok(event) = event_rx.recv().await {
            if let Ok(json_str) = serde_json::to_string(&event) {
                let _ = app_handle.emit_all("gateway-event", format!("EVENT:{}", json_str));
            }
        }
    });
}

fn main() {
    // Load .env from project root
    dotenvy::dotenv().ok();

    let tray_menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("show".to_string(), "Show Dashboard"))
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("quit".to_string(), "Quit Savant"));

    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .manage(AppState {
            ignition: Mutex::new(None),
            nexus: Mutex::new(None),
        })
        .setup(|app| {
            let handle = app.handle();
            let bridge = LogBridge { app_handle: handle };

            use tracing_subscriber::layer::SubscriberExt;
            use tracing_subscriber::util::SubscriberInitExt;

            let _ = tracing_subscriber::registry()
                .with(tracing_subscriber::EnvFilter::new(
                    "info,savant_desktop=debug,savant_agent=debug",
                ))
                .with(tracing_subscriber::fmt::layer())
                .with(bridge)
                .try_init();

            info!("🧬 Savant Swarm Desktop Substrate Online");
            Ok(())
        })
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| match event {
            tauri::SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "quit" => {
                    std::process::exit(0);
                }
                "show" => {
                    let window = app.get_window("main").unwrap();
                    window.show().unwrap();
                    window.set_focus().unwrap();
                }
                _ => {}
            },
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![ignite_swarm, get_swarm_status])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
