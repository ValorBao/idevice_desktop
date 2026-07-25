use std::collections::HashMap;

use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

use crate::discovery::DiscoveryCatalog;

#[derive(Default)]
pub struct AppState {
    pub selected_udid: RwLock<Option<String>>,
    pub discovery: RwLock<DiscoveryCatalog>,
    pub tasks: Mutex<HashMap<String, CancellationToken>>,
}

impl AppState {
    pub async fn selected(&self, override_udid: Option<String>) -> Option<String> {
        match override_udid {
            Some(udid) => Some(udid),
            None => self.selected_udid.read().await.clone(),
        }
    }

    pub async fn replace_task(&self, key: impl Into<String>, token: CancellationToken) {
        let mut tasks = self.tasks.lock().await;
        if let Some(previous) = tasks.insert(key.into(), token) {
            previous.cancel();
        }
    }

    pub async fn cancel_task(&self, key: &str) {
        if let Some(token) = self.tasks.lock().await.remove(key) {
            token.cancel();
        }
    }

    pub async fn cancel_device_tasks(&self) {
        let mut tasks = self.tasks.lock().await;
        let keys = tasks
            .keys()
            .filter(|key| key.as_str() != "device-monitor")
            .cloned()
            .collect::<Vec<_>>();
        for key in keys {
            if let Some(token) = tasks.remove(&key) {
                token.cancel();
            }
        }
    }
}
