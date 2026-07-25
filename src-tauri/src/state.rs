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

#[cfg(test)]
mod tests {
    use super::*;

    /// Long-running work registers a token here so switching or disconnecting a
    /// device can stop it. A leaked token means a JIT, log, or location session
    /// keeps running against a device the user has moved away from.
    #[tokio::test]
    async fn replacing_a_task_cancels_the_previous_one() {
        let state = AppState::default();
        let first = CancellationToken::new();
        state.replace_task("jit", first.clone()).await;

        let second = CancellationToken::new();
        state.replace_task("jit", second.clone()).await;

        assert!(first.is_cancelled(), "the superseded session kept running");
        assert!(!second.is_cancelled());
    }

    #[tokio::test]
    async fn cancelling_one_task_leaves_the_others_running() {
        let state = AppState::default();
        let jit = CancellationToken::new();
        let logs = CancellationToken::new();
        state.replace_task("jit", jit.clone()).await;
        state.replace_task("logs", logs.clone()).await;

        state.cancel_task("jit").await;

        assert!(jit.is_cancelled());
        assert!(!logs.is_cancelled());
        // A cancelled task is dropped, so a second call is a no-op.
        state.cancel_task("jit").await;
    }

    /// Device monitoring outlives a device switch on purpose: it is what
    /// discovers the next device.
    #[tokio::test]
    async fn switching_devices_stops_every_task_except_monitoring() {
        let state = AppState::default();
        let monitor = CancellationToken::new();
        let jit = CancellationToken::new();
        let logs = CancellationToken::new();
        let location = CancellationToken::new();
        state.replace_task("device-monitor", monitor.clone()).await;
        state.replace_task("jit", jit.clone()).await;
        state.replace_task("logs", logs.clone()).await;
        state.replace_task("location", location.clone()).await;

        state.cancel_device_tasks().await;

        assert!(!monitor.is_cancelled(), "monitoring must survive a switch");
        assert!(jit.is_cancelled());
        assert!(logs.is_cancelled());
        assert!(location.is_cancelled());
        assert_eq!(state.tasks.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn an_override_udid_wins_over_the_selection() {
        let state = AppState::default();
        *state.selected_udid.write().await = Some("selected".into());
        assert_eq!(state.selected(None).await.as_deref(), Some("selected"));
        assert_eq!(
            state.selected(Some("override".into())).await.as_deref(),
            Some("override")
        );
    }
}
