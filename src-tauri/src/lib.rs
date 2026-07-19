mod commands;
mod device_version;
mod error;
mod provider;
mod state;
mod tunnel;
mod types;
mod utils;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Install rustls once before concurrent device connections can race to do it.
    let _ = rustls::crypto::ring::default_provider().install_default();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "idevice_desktop=info,warn".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::health,
            commands::device_list,
            commands::device_select,
            commands::device_disconnect,
            commands::device_pair,
            commands::device_forget,
            commands::device_monitor_start,
            commands::device_monitor_stop,
            commands::overview_get,
            commands::device_screenshot,
            commands::diagnostics_battery,
            commands::diagnostics_gestalt,
            commands::diagnostics_ioregistry,
            commands::diagnostics_nand,
            commands::diagnostics_wifi,
            commands::afc_list,
            commands::afc_mkdir,
            commands::afc_remove,
            commands::afc_upload,
            commands::afc_download,
            commands::file_sharing_apps,
            commands::apps_list,
            commands::app_install,
            commands::app_uninstall,
            commands::logs_start,
            commands::logs_stop,
            commands::developer_status,
            commands::developer_mode_reveal,
            commands::developer_mode_enable,
            commands::developer_mode_accept,
            commands::ddi_mount,
            commands::ddi_mount_auto,
            commands::ddi_unmount,
            commands::jit_start,
            commands::jit_stop,
            commands::location_start,
            commands::location_stop,
        ])
        .run(tauri::generate_context!())
        .expect("error while running idevice desktop");
}
