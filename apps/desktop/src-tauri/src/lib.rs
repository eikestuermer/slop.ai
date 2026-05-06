//! Tauri host for the Slop AI desktop app.
//!
//! Owns the in-memory project state, the op log on disk, the BYO endpoint
//! configuration, and the IPC commands that the frontend invokes.

use tauri::Manager;

mod commands;
mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,slop_=debug")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let state = state::AppState::new(app.handle().clone())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_project,
            commands::new_project,
            commands::import_asset,
            commands::generate_proxies,
            commands::transcribe_asset,
            commands::detect_scenes,
            commands::build_candidates,
            commands::plan_rough_cut,
            commands::regenerate_range,
            commands::pin_clip,
            commands::unpin_clip,
            commands::render_preview,
            commands::export_otio,
            commands::get_timeline,
            commands::get_endpoint_config,
            commands::set_endpoint_config,
            commands::set_privacy_mode,
            commands::get_privacy_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
