use std::collections::{HashMap, VecDeque};

use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

use crate::discovery::DiscoveryCatalog;

/// Task-key prefix for log streams.
///
/// These are the only tasks a user starts and stops by session ID, so they are
/// the only ones whose recently stopped IDs the registry remembers: a stop can
/// reach the backend before the start it cancels.
pub const LOG_SESSION_PREFIX: &str = "logs:";

/// The task key for one log session.
pub fn log_session_key(session_id: &str) -> String {
    format!("{LOG_SESSION_PREFIX}{session_id}")
}

#[derive(Default)]
pub struct AppState {
    pub selected_udid: RwLock<Option<String>>,
    pub discovery: RwLock<DiscoveryCatalog>,
    pub tasks: Mutex<HashMap<String, CancellationToken>>,
    stopped_sessions: Mutex<VecDeque<String>>,
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

    /// Replace a stream without letting a late stop for its predecessor cancel it.
    pub async fn replace_session_task(&self, prefix: &str, id: &str, token: CancellationToken) {
        let mut tasks = self.tasks.lock().await;
        let key = format!("{prefix}{id}");
        if self.stopped_sessions.lock().await.contains(&key) {
            token.cancel();
            return;
        }
        tasks.retain(|key, previous| {
            if key.starts_with(prefix) {
                previous.cancel();
                false
            } else {
                true
            }
        });
        tasks.insert(format!("{prefix}{id}"), token);
    }

    pub async fn cancel_task(&self, key: &str) {
        let mut tasks = self.tasks.lock().await;
        if key.starts_with(LOG_SESSION_PREFIX) {
            let mut stopped = self.stopped_sessions.lock().await;
            if stopped.len() == 64 {
                stopped.pop_front();
            }
            stopped.push_back(key.into());
        }
        if let Some(token) = tasks.remove(key) {
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
    #[tokio::test]
    async fn late_stream_stop_does_not_cancel_its_replacement() {
        let state = AppState::default();
        let old = CancellationToken::new();
        let current = CancellationToken::new();
        state
            .replace_session_task(LOG_SESSION_PREFIX, "old", old.clone())
            .await;
        state
            .replace_session_task(LOG_SESSION_PREFIX, "current", current.clone())
            .await;
        assert!(old.is_cancelled());
        state.cancel_task("logs:old").await;
        assert!(!current.is_cancelled());
        state.cancel_device_tasks().await;
        assert!(current.is_cancelled());
    }

    #[tokio::test]
    async fn stop_before_registration_prevents_a_late_start_replacing_current_logs() {
        let state = AppState::default();
        state.cancel_task("logs:old").await;
        let current = CancellationToken::new();
        state
            .replace_session_task(LOG_SESSION_PREFIX, "current", current.clone())
            .await;
        let old = CancellationToken::new();
        state
            .replace_session_task(LOG_SESSION_PREFIX, "old", old.clone())
            .await;
        assert!(old.is_cancelled());
        assert!(!current.is_cancelled());
    }
}
