pub mod commands;
pub mod device_version;
pub mod discovery;
pub mod error;
pub mod legacy_ddi;
pub mod provider;
mod state;
mod task;
pub mod transport;
pub mod trollstore;
pub mod tunnel;
pub mod types;
mod utils;

use commands::{AccountSigningState, LiveScreenState, NetworkCaptureState, PasteboardState};
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
        .manage(NetworkCaptureState::default())
        .manage(LiveScreenState::default())
        .manage(PasteboardState::default())
        .manage(AccountSigningState::default())
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
            commands::afc_create_file,
            commands::afc_rename,
            commands::afc_remove,
            commands::afc_upload,
            commands::afc_download,
            commands::afc_transfer_cancel,
            commands::file_sharing_apps,
            commands::apps_list,
            commands::apps_debuggable,
            commands::app_install,
            commands::app_uninstall,
            commands::crash_reports_list,
            commands::crash_report_read,
            commands::crash_report_export,
            commands::logs_start,
            commands::logs_stop,
            commands::processes_list,
            commands::process_launch,
            commands::process_stop,
            commands::performance_start,
            commands::performance_stop,
            commands::performance_export_csv,
            commands::network_capture_start,
            commands::network_capture_stop,
            commands::network_capture_cancel,
            commands::notification_observation_start,
            commands::notification_observation_stop,
            commands::live_screen_start,
            commands::live_screen_stop,
            commands::live_screen_export_frame,
            commands::provisioning_profiles_list,
            commands::personal_signing_preflight,
            commands::personal_signing_export,
            commands::personal_account_status,
            commands::personal_account_login,
            commands::personal_account_submit_two_factor,
            commands::personal_account_cancel_two_factor,
            commands::personal_account_sign_out,
            commands::personal_account_sign_export,
            commands::personal_account_sign_cancel,
            commands::pasteboard_text_read,
            commands::pasteboard_text_write,
            commands::pasteboard_image_read,
            commands::pasteboard_image_prepare,
            commands::pasteboard_image_write,
            commands::pasteboard_image_discard,
            commands::xctest_preflight,
            commands::xctest_plan_prepare,
            commands::trollstore_status,
            commands::trollstore_helper_install,
            commands::trollstore_ipa_install,
            commands::developer_status,
            commands::developer_mode_reveal,
            commands::developer_mode_enable,
            commands::developer_mode_accept,
            commands::ddi_mount,
            commands::ddi_mount_auto,
            commands::ddi_ensure,
            commands::ddi_download,
            commands::ddi_unmount,
            commands::jit_start,
            commands::jit_stop,
            commands::location_start,
            commands::location_stop,
        ])
        .run(tauri::generate_context!())
        .expect("error while running idevice_desktop");
}
