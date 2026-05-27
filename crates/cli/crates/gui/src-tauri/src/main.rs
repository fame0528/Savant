#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("savant_gui=info,tauri=info")
        .init();

    savant_gui_lib::run();
}
