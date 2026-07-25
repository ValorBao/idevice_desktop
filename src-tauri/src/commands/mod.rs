mod apps;
mod crash_reports;
mod developer;
mod device;
mod diagnostics;
mod files;
mod location;
mod logs;
mod overview;
mod screenshot;

pub use apps::*;
pub use crash_reports::*;
pub use developer::*;
pub use device::*;
pub use diagnostics::*;
pub use files::*;
pub use location::*;
pub use logs::*;
pub use overview::*;
pub use screenshot::*;

#[tauri::command]
pub async fn health() -> &'static str {
    "ok"
}
