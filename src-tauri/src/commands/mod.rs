mod account_signing;
mod apps;
mod crash_reports;
mod developer;
mod device;
mod diagnostics;
mod files;
mod live_screen;
mod location;
mod logs;
mod network_capture;
mod notifications;
mod overview;
mod pasteboard;
mod performance;
mod processes;
mod profiles;
mod screenshot;
mod signing;
mod xctest;

pub use account_signing::*;
pub use apps::*;
pub use crash_reports::*;
pub use developer::*;
pub use device::*;
pub use diagnostics::*;
pub use files::*;
pub use live_screen::*;
pub use location::*;
pub use logs::*;
pub use network_capture::*;
pub use notifications::*;
pub use overview::*;
pub use pasteboard::*;
pub use performance::*;
pub use processes::*;
pub use profiles::*;
pub use screenshot::*;
pub use signing::*;
pub use xctest::*;

#[tauri::command]
pub async fn health() -> &'static str {
    "ok"
}
