use std::future::Future;

use tokio_util::sync::CancellationToken;

use crate::error::CommandError;

/// Runs a long-lived device stream on a dedicated current-thread runtime so
/// blocking DVT clients cannot stall the Tauri async pool. Errors are reported
/// only if the caller has not already cancelled the session.
pub fn spawn_stream_task<F, Fut>(
    token: CancellationToken,
    work: F,
    on_error: impl FnOnce(String) + Send + 'static,
) where
    F: FnOnce(CancellationToken) -> Fut + Send + 'static,
    Fut: Future<Output = Result<(), CommandError>>,
{
    tauri::async_runtime::spawn_blocking(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                on_error(error.to_string());
                return;
            }
        };
        let completion_token = token.clone();
        match runtime.block_on(work(token)) {
            Ok(()) => {}
            Err(error) if !completion_token.is_cancelled() => on_error(error.message),
            Err(_) => {}
        }
    });
}
