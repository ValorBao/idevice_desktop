use std::{future::Future, time::Duration};

use tokio_util::sync::CancellationToken;

use crate::error::{CommandError, CommandResult};

/// The software-tunnel adapter (`AdapterHandle`) is driven from the current
/// executor: its I/O wakers are registered on whatever runtime first polls
/// `create_software_tunnel`. A multi-thread Tokio runtime can move the
/// continuation to another worker after that poll, and later service calls
/// then stall.
///
/// Commands that open a Core-generation tunnel therefore hop onto a dedicated
/// `current_thread` runtime before they drive the adapter. Screenshot,
/// location, and JIT defined this boundary; Live Screen, performance,
/// processes, and pasteboard already follow it. This helper is that decision
/// written down — not leftover boilerplate to delete.
///
/// Device-monitor threads stay on their own dedicated runtimes because they
/// are long-lived usbmuxd/Bonjour watchers, not tunnel adapters.
///
/// Crash reports, profiles, notifications, and network capture still run on
/// the Tauri runtime today. They open a tunnel only for the Bonjour/Core
/// path and keep the adapter on that runtime. Moving them onto
/// `tunnel_runtime` would change scheduling of an already-working path, so
/// they stay put until a stall shows up. New commands that open Core tunnels
/// should use this helper.
pub fn tunnel_runtime() -> CommandResult<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| CommandError::new("runtime", error.to_string(), true))
}

/// Build [`tunnel_runtime`] and `block_on` the operation.
pub fn block_on_tunnel<T>(operation: impl Future<Output = CommandResult<T>>) -> CommandResult<T> {
    tunnel_runtime()?.block_on(operation)
}

/// Hop off the Tauri runtime onto [`tunnel_runtime`] for adapter work.
pub async fn run_on_tunnel_runtime<T, F, Fut>(operation: F) -> CommandResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = CommandResult<T>> + 'static,
{
    tokio::task::spawn_blocking(move || block_on_tunnel(operation()))
        .await
        .map_err(|error| CommandError::new("runtime", error.to_string(), true))?
}

/// Bound an asynchronous stage, including cancellation before its first poll.
/// A synchronous library call cannot be interrupted; check again when it yields.
pub async fn stage<T>(
    token: &CancellationToken,
    name: &str,
    limit: Duration,
    operation: impl Future<Output = CommandResult<T>>,
) -> CommandResult<T> {
    let deadline = tokio::time::Instant::now() + limit;
    let result = tokio::select! {
        biased;
        _ = token.cancelled() => return Err(cancelled(name)),
        result = tokio::time::timeout_at(deadline, operation) => result,
    };
    if token.is_cancelled() {
        return Err(cancelled(name));
    }
    if tokio::time::Instant::now() >= deadline {
        return Err(timed_out(name));
    }
    result.map_err(|_| timed_out(name))?
}

/// Stop and reap subprocesses before callers clean their working directories.
pub async fn wait_child(
    token: &CancellationToken,
    name: &str,
    limit: Duration,
    child: &mut tokio::process::Child,
) -> CommandResult<std::process::ExitStatus> {
    let result = stage(token, name, limit, async {
        child.wait().await.map_err(CommandError::from)
    })
    .await;
    if result.is_err() {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
    result
}

fn cancelled(name: &str) -> CommandError {
    CommandError::new("cancelled", format!("{name} cancelled"), false)
}

fn timed_out(name: &str) -> CommandError {
    CommandError::new("timeout", format!("{name} timed out"), true)
}

/// A single operation owns this slot until its guard is dropped. Cancelling an
/// old ID never affects a new operation, and cancellation does not free the slot
/// until the worker has finished cleaning up.
#[derive(Default)]
pub struct OperationSlot(std::sync::Mutex<OperationState>);

#[derive(Default)]
struct OperationState {
    active: Option<(String, CancellationToken)>,
    cancelled: std::collections::VecDeque<String>,
}

pub struct OperationGuard<'a> {
    slot: &'a OperationSlot,
    pub token: CancellationToken,
}

impl OperationSlot {
    pub fn begin(&self, id: &str) -> CommandResult<OperationGuard<'_>> {
        uuid::Uuid::parse_str(id)
            .map_err(|_| CommandError::new("operation", "Invalid operation ID", false))?;
        let mut slot = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if slot.cancelled.iter().any(|cancelled| cancelled == id) {
            return Err(cancelled("Operation"));
        }
        if slot.active.is_some() {
            return Err(CommandError::new(
                "busy",
                "An operation is already running; wait for it to finish",
                true,
            ));
        }
        let token = CancellationToken::new();
        slot.active = Some((id.into(), token.clone()));
        Ok(OperationGuard { slot: self, token })
    }

    pub fn cancel(&self, id: Option<&str>) {
        let mut slot = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if let Some((current, token)) = slot.active.as_ref()
            && id.is_none_or(|id| id == current)
        {
            token.cancel();
        }
        if let Some(id) = id {
            if slot.cancelled.len() == 64 {
                slot.cancelled.pop_front();
            }
            slot.cancelled.push_back(id.into());
        }
    }
}

impl Drop for OperationGuard<'_> {
    fn drop(&mut self) {
        self.token.cancel();
        self.slot
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .active
            .take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancelled_stage_never_starts_the_operation() {
        let token = CancellationToken::new();
        token.cancel();
        let result = stage(&token, "Connect", Duration::from_secs(1), async {
            panic!("cancelled operation was polled");
            #[allow(unreachable_code)]
            Ok(())
        })
        .await;
        assert_eq!(result.unwrap_err().kind, "cancelled");
    }

    #[tokio::test]
    async fn cancellation_drops_in_flight_work() {
        let token = CancellationToken::new();
        let cancel = token.clone();
        let (sender, receiver) = tokio::sync::oneshot::channel::<()>();
        let result = stage(&token, "Connect", Duration::from_secs(1), async move {
            let _sender = sender;
            cancel.cancel();
            std::future::pending::<CommandResult<()>>().await
        })
        .await;
        assert_eq!(result.unwrap_err().kind, "cancelled");
        assert!(receiver.await.is_err());
    }

    #[tokio::test]
    async fn stalled_stage_times_out() {
        let result = stage(
            &CancellationToken::new(),
            "Connect",
            Duration::from_millis(1),
            std::future::pending::<CommandResult<()>>(),
        )
        .await;
        assert_eq!(result.unwrap_err().kind, "timeout");
    }

    #[tokio::test]
    async fn synchronous_completion_cannot_hide_cancellation() {
        let token = CancellationToken::new();
        let result = stage(&token, "Sign", Duration::from_secs(1), async {
            token.cancel();
            Ok(())
        })
        .await;
        assert_eq!(result.unwrap_err().kind, "cancelled");
    }
    #[test]
    fn cancellation_reserves_the_slot_until_cleanup_and_old_ids_are_isolated() {
        let slot = OperationSlot::default();
        let first_id = uuid::Uuid::new_v4().to_string();
        let next_id = uuid::Uuid::new_v4().to_string();
        let first = slot.begin(&first_id).unwrap();
        slot.cancel(Some(&first_id));
        assert!(first.token.is_cancelled());
        assert!(slot.begin(&next_id).is_err());
        drop(first);
        let next = slot.begin(&next_id).unwrap();
        slot.cancel(Some(&first_id));
        assert!(!next.token.is_cancelled());
        slot.cancel(None);
        assert!(next.token.is_cancelled());
    }

    #[test]
    fn a_cancel_delivered_before_start_prevents_the_operation() {
        let slot = OperationSlot::default();
        let id = uuid::Uuid::new_v4().to_string();
        slot.cancel(Some(&id));
        assert!(matches!(slot.begin(&id), Err(error) if error.kind == "cancelled"));
    }

    #[tokio::test]
    async fn cancelled_packaging_reaps_its_child_before_returning() {
        let mut child = tokio::process::Command::new("/bin/sleep")
            .arg("30")
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let token = CancellationToken::new();
        token.cancel();
        let result = wait_child(&token, "Packaging", Duration::from_secs(1), &mut child).await;
        assert_eq!(result.unwrap_err().kind, "cancelled");
        assert!(child.try_wait().unwrap().is_some());
    }

    #[tokio::test]
    async fn timed_out_packaging_reaps_its_child_before_returning() {
        let mut child = tokio::process::Command::new("/bin/sleep")
            .arg("30")
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let result = wait_child(
            &CancellationToken::new(),
            "Packaging",
            Duration::from_millis(1),
            &mut child,
        )
        .await;
        assert_eq!(result.unwrap_err().kind, "timeout");
        assert!(child.try_wait().unwrap().is_some());
    }
}
