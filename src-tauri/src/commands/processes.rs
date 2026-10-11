use std::{collections::HashSet, future::Future, time::Duration};

use idevice::{
    IdeviceService, RsdService,
    core_device::{AppServiceClient, ProcessToken},
    dvt::{
        device_info::{DeviceInfoClient, RunningProcess},
        process_control::ProcessControlClient,
        remote_server::RemoteServerClient,
    },
    installation_proxy::InstallationProxyClient,
};
use tauri::{AppHandle, State};
use tokio_util::sync::CancellationToken;

use crate::{
    device_version::{DeveloperGeneration, developer_generation},
    error::{CommandError, CommandResult},
    provider::RoutedProvider,
    state::AppState,
    transport::DeviceContext,
    tunnel::RsdTunnel,
    types::{ProcessLaunch, ProcessSnapshot, ProcessSummary},
};

const PROCESS_TIMEOUT: Duration = Duration::from_secs(45);
const STOP_WAIT: Duration = Duration::from_secs(10);
const REMOTE_PAIRING_ATTEMPTS: usize = 3;
const DVT_STOP_LIMITATION: &str = "Stopping applications is disabled on the DVT fallback because this device did not terminate an identity-checked test process. Enable the Developer Disk Image services so CoreDevice AppService is available.";
const SIGTERM: u32 = 15;

enum ProcessConnection {
    Legacy,
    Core {
        route: &'static str,
        tunnel: Box<RsdTunnel>,
        user_app_names: HashSet<String>,
    },
}

/// The software tunnel types are intentionally driven by a single-threaded
/// runtime. This is the same boundary used by screenshot, location, and JIT.
async fn run_on_local_runtime<T, F, Fut>(token: CancellationToken, operation: F) -> CommandResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = CommandResult<T>> + 'static,
{
    tokio::task::spawn_blocking(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| CommandError::new("runtime", error.to_string(), true))?;
        runtime.block_on(async move {
            tokio::select! {
                _ = token.cancelled() => Err(CommandError::new(
                    "processes",
                    "The process operation was cancelled because the device session changed",
                    true,
                )),
                result = tokio::time::timeout(PROCESS_TIMEOUT, operation()) => result.map_err(|_| {
                    CommandError::new(
                        "processes",
                        "The process operation timed out. Keep the device unlocked and retry.",
                        true,
                    )
                })?,
            }
        })
    })
    .await
    .map_err(|error| CommandError::new("runtime", error.to_string(), true))?
}

async fn connect(context: &DeviceContext) -> CommandResult<ProcessConnection> {
    let provider = context.provider().await?;
    let generation = developer_generation(&provider).await?;
    if generation == DeveloperGeneration::Legacy {
        return Ok(ProcessConnection::Legacy);
    }
    let user_app_names = user_application_names(&provider).await;
    match context
        .open_rsd_tunnel(&provider, generation, REMOTE_PAIRING_ATTEMPTS)
        .await?
    {
        None => Ok(ProcessConnection::Legacy),
        Some((route, tunnel)) => Ok(ProcessConnection::Core {
            route,
            tunnel: Box::new(tunnel),
            user_app_names,
        }),
    }
}

/// Returns names that can be tied back to an installed user application.
///
/// DVT's `isApplication` flag also covers system UI services. Matching the
/// executable and display names from Installation Proxy supplements the
/// stronger `/var/containers/Bundle/Application` boundary reported for running
/// user apps. Sideloaded apps may register as `System`, so the path boundary is
/// necessary while still excluding `/Applications` and system service paths.
async fn user_application_names(provider: &RoutedProvider) -> HashSet<String> {
    let result = async {
        let mut client = InstallationProxyClient::connect(provider).await?;
        client.get_apps(Some("User"), None).await
    }
    .await;
    let Ok(apps) = result else {
        tracing::warn!("unable to classify user applications for process stop safety");
        return HashSet::new();
    };
    apps.into_values()
        .filter_map(|value| value.into_dictionary())
        .flat_map(|dictionary| {
            ["CFBundleExecutable", "CFBundleDisplayName", "CFBundleName"]
                .into_iter()
                .filter_map(move |key| {
                    dictionary
                        .get(key)
                        .and_then(plist::Value::as_string)
                        .map(str::to_lowercase)
                })
        })
        .collect()
}

fn legacy_snapshot() -> ProcessSnapshot {
    ProcessSnapshot {
        processes: Vec::new(),
        transport: "Legacy".into(),
        available: false,
        supports_launch: false,
        supports_stop: false,
        limitation: Some(
            "Process monitoring is unavailable on iOS 16 and earlier because the verified Legacy instruments service does not provide a reliable workflow."
                .into(),
        ),
    }
}

fn core_summary(process: ProcessToken) -> ProcessSummary {
    let executable_path = process.executable_url.map(|url| url.relative);
    let name = executable_path
        .as_deref()
        .and_then(|path| path.trim_end_matches('/').rsplit('/').next())
        .filter(|name| !name.is_empty())
        .unwrap_or("Unknown process")
        .to_string();
    let is_application = executable_path
        .as_deref()
        .is_some_and(|path| path.contains(".app/"));
    let can_stop = executable_path.as_deref().is_some_and(is_user_app_path);
    let identity = format!(
        "core:{}:{}",
        process.pid,
        executable_path.as_deref().unwrap_or("")
    );
    ProcessSummary {
        pid: process.pid,
        name,
        executable_path,
        is_application,
        can_stop,
        identity,
    }
}

fn is_user_app_path(path: &str) -> bool {
    path.contains("/var/containers/Bundle/Application/") && path.contains(".app/")
}

fn dvt_summary(process: RunningProcess, user_app_names: &HashSet<String>) -> ProcessSummary {
    let executable_path = [process.real_app_name.as_str(), process.name.as_str()]
        .into_iter()
        .find(|value| value.contains('/'))
        .map(str::to_string);
    let can_stop = process.is_application
        && (executable_path.as_deref().is_some_and(is_user_app_path)
            || [process.name.as_str(), process.real_app_name.as_str()]
                .into_iter()
                .filter(|name| !name.is_empty() && !name.contains('/'))
                .any(|name| user_app_names.contains(&name.to_lowercase())));
    let name = [process.real_app_name.as_str(), process.name.as_str()]
        .into_iter()
        .find(|value| !value.is_empty() && !value.contains('/'))
        .map(str::to_string)
        .or_else(|| {
            executable_path
                .as_deref()
                .and_then(|path| path.trim_end_matches('/').rsplit('/').next())
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "Unknown process".into());
    let identity = format!(
        "dvt:{}:{}:{}:{}",
        process.pid, process.name, process.real_app_name, process.start_page_count
    );
    ProcessSummary {
        pid: process.pid,
        name,
        executable_path,
        is_application: process.is_application,
        can_stop,
        identity,
    }
}

fn sort_processes(processes: &mut [ProcessSummary]) {
    processes.sort_by(|left, right| {
        right
            .is_application
            .cmp(&left.is_application)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.pid.cmp(&right.pid))
    });
}

fn validate_bundle_id(bundle_id: &str) -> CommandResult<()> {
    if bundle_id.is_empty()
        || bundle_id.len() > 255
        || bundle_id.chars().any(|character| {
            character.is_control() || character.is_whitespace() || character == '/'
        })
    {
        return Err(CommandError::new(
            "processes",
            "Choose a valid installed application bundle identifier",
            false,
        ));
    }
    Ok(())
}

fn matching_stoppable_process<'a>(
    processes: &'a [ProcessSummary],
    pid: u32,
    identity: &str,
) -> CommandResult<&'a ProcessSummary> {
    let process = processes
        .iter()
        .find(|process| process.pid == pid)
        .ok_or_else(|| {
            CommandError::new(
                "processes",
                format!("Pid {pid} is no longer running. Refresh the process list."),
                true,
            )
        })?;
    if process.identity != identity {
        return Err(CommandError::new(
            "processes",
            format!("Pid {pid} now belongs to a different process. Refresh before stopping it."),
            true,
        ));
    }
    if pid <= 1 || !process.can_stop {
        return Err(CommandError::new(
            "processes",
            "The first Processes release only stops installed user applications",
            false,
        ));
    }
    Ok(process)
}

fn core_capabilities(has_app_service: bool) -> (bool, bool, Option<String>) {
    (
        true,
        has_app_service,
        (!has_app_service).then(|| DVT_STOP_LIMITATION.into()),
    )
}

async fn list_impl(context: DeviceContext) -> CommandResult<ProcessSnapshot> {
    let ProcessConnection::Core {
        route,
        mut tunnel,
        user_app_names,
    } = connect(&context).await?
    else {
        return Ok(legacy_snapshot());
    };

    let (mut processes, protocol) = if tunnel
        .handshake
        .services
        .contains_key("com.apple.coredevice.appservice")
    {
        let mut client = AppServiceClient::connect_rsd(&mut tunnel.adapter, &mut tunnel.handshake)
            .await
            .map_err(CommandError::from)?;
        let processes = client
            .list_processes()
            .await
            .map_err(CommandError::from)?
            .into_iter()
            .map(core_summary)
            .collect::<Vec<_>>();
        (processes, "CoreDevice AppService")
    } else {
        let mut remote =
            RemoteServerClient::connect_rsd(&mut tunnel.adapter, &mut tunnel.handshake)
                .await
                .map_err(|error| {
                    CommandError::new(
                        "processes",
                        format!(
                            "DVT is unavailable. Mount the Developer Disk Image and retry: {error}"
                        ),
                        true,
                    )
                })?;
        remote.read_message(0).await.map_err(CommandError::from)?;
        let mut client = DeviceInfoClient::new(&mut remote)
            .await
            .map_err(CommandError::from)?;
        let processes = client
            .running_processes()
            .await
            .map_err(CommandError::from)?
            .into_iter()
            .map(|process| dvt_summary(process, &user_app_names))
            .collect::<Vec<_>>();
        (processes, "DVT DeviceInfo")
    };
    sort_processes(&mut processes);
    let (supports_launch, supports_stop, limitation) =
        core_capabilities(protocol == "CoreDevice AppService");
    Ok(ProcessSnapshot {
        processes,
        transport: format!("{protocol} · {route}"),
        available: true,
        supports_launch,
        supports_stop,
        limitation,
    })
}

/// Read-only service-layer entry point used by the Tauri command and the
/// hardware verification harness. Keeping the harness on this function avoids
/// proving a second copy of the transport-selection logic.
pub async fn processes_snapshot_for_device(
    context: DeviceContext,
) -> CommandResult<ProcessSnapshot> {
    list_impl(context).await
}

async fn launch_impl(context: DeviceContext, bundle_id: String) -> CommandResult<ProcessLaunch> {
    validate_bundle_id(&bundle_id)?;
    let ProcessConnection::Core {
        route, mut tunnel, ..
    } = connect(&context).await?
    else {
        return Err(CommandError::new(
            "processes",
            "Launching applications from Processes is unavailable on iOS 16 and earlier",
            false,
        ));
    };

    let (pid, protocol) = if tunnel
        .handshake
        .services
        .contains_key("com.apple.coredevice.appservice")
    {
        let mut client = AppServiceClient::connect_rsd(&mut tunnel.adapter, &mut tunnel.handshake)
            .await
            .map_err(CommandError::from)?;
        let launch = client
            .launch_application(bundle_id.clone(), &[], false, false, None, None, None)
            .await
            .map_err(CommandError::from)?;
        (launch.pid, "CoreDevice AppService")
    } else {
        let mut remote =
            RemoteServerClient::connect_rsd(&mut tunnel.adapter, &mut tunnel.handshake)
                .await
                .map_err(CommandError::from)?;
        remote.read_message(0).await.map_err(CommandError::from)?;
        let mut client = ProcessControlClient::new(&mut remote)
            .await
            .map_err(CommandError::from)?;
        let pid = client
            .launch_app(bundle_id.clone(), None, None, false, false)
            .await
            .map_err(CommandError::from)?;
        let pid = u32::try_from(pid).map_err(|_| {
            CommandError::new(
                "processes",
                "The device returned an invalid process id",
                true,
            )
        })?;
        (pid, "DVT ProcessControl")
    };
    Ok(ProcessLaunch {
        pid,
        bundle_id,
        transport: format!("{protocol} · {route}"),
    })
}

/// Service-layer launch entry point shared by the Tauri command and real-device
/// verification. This keeps hardware acceptance on the same route selection
/// and protocol fallback as the desktop UI.
pub async fn process_launch_for_device(
    context: DeviceContext,
    bundle_id: String,
) -> CommandResult<ProcessLaunch> {
    launch_impl(context, bundle_id).await
}

async fn stop_impl(context: DeviceContext, pid: u32, identity: String) -> CommandResult<()> {
    let ProcessConnection::Core { mut tunnel, .. } = connect(&context).await? else {
        return Err(CommandError::new(
            "processes",
            "Stopping processes is unavailable on iOS 16 and earlier",
            false,
        ));
    };

    if tunnel
        .handshake
        .services
        .contains_key("com.apple.coredevice.appservice")
    {
        let mut client = AppServiceClient::connect_rsd(&mut tunnel.adapter, &mut tunnel.handshake)
            .await
            .map_err(CommandError::from)?;
        let current = client
            .list_processes()
            .await
            .map_err(CommandError::from)?
            .into_iter()
            .map(core_summary)
            .collect::<Vec<_>>();
        matching_stoppable_process(&current, pid, &identity)?;
        client
            .send_signal(pid, SIGTERM)
            .await
            .map_err(CommandError::from)?;
        let deadline = tokio::time::Instant::now() + STOP_WAIT;
        loop {
            let running = client
                .list_processes()
                .await
                .map_err(CommandError::from)?
                .into_iter()
                .any(|process| process.pid == pid);
            if !running {
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(CommandError::new(
                    "processes",
                    format!("Pid {pid} remained running after SIGTERM"),
                    true,
                ));
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    Err(CommandError::new("processes", DVT_STOP_LIMITATION, false))
}

/// Service-layer stop entry point shared by the Tauri command and real-device
/// verification. The implementation always relists the process and validates
/// its opaque identity and installed-user-app classification before signaling.
pub async fn process_stop_for_device(
    context: DeviceContext,
    pid: u32,
    identity: String,
) -> CommandResult<()> {
    stop_impl(context, pid, identity).await
}

#[tauri::command]
pub async fn processes_list(
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
) -> CommandResult<ProcessSnapshot> {
    let context = DeviceContext::resolve(&app, &state, udid).await?;
    let token = CancellationToken::new();
    let task_key = format!("process-list:{}", uuid::Uuid::new_v4());
    state.replace_task(task_key.clone(), token.clone()).await;
    let result = run_on_local_runtime(token, move || processes_snapshot_for_device(context)).await;
    state.cancel_task(&task_key).await;
    result
}

#[tauri::command]
pub async fn process_launch(
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
    bundle_id: String,
) -> CommandResult<ProcessLaunch> {
    let context = DeviceContext::resolve(&app, &state, udid).await?;
    let token = CancellationToken::new();
    let task_key = format!("process-control:{}", uuid::Uuid::new_v4());
    state.replace_task(task_key.clone(), token.clone()).await;
    let result =
        run_on_local_runtime(token, move || process_launch_for_device(context, bundle_id)).await;
    state.cancel_task(&task_key).await;
    result
}

#[tauri::command]
pub async fn process_stop(
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
    pid: u32,
    identity: String,
) -> CommandResult<()> {
    let context = DeviceContext::resolve(&app, &state, udid).await?;
    let token = CancellationToken::new();
    let task_key = format!("process-control:{}", uuid::Uuid::new_v4());
    state.replace_task(task_key.clone(), token.clone()).await;
    let result = run_on_local_runtime(token, move || {
        process_stop_for_device(context, pid, identity)
    })
    .await;
    state.cancel_task(&task_key).await;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn application(pid: u32, identity: &str) -> ProcessSummary {
        ProcessSummary {
            pid,
            name: "Example".into(),
            executable_path: Some("/Applications/Example.app/Example".into()),
            is_application: true,
            can_stop: true,
            identity: identity.into(),
        }
    }

    #[test]
    fn derives_core_application_identity_from_its_executable() {
        let summary = core_summary(ProcessToken {
            pid: 42,
            executable_url: Some(idevice::core_device::ExecutableUrl {
                relative: "/private/var/containers/Bundle/Application/ID/Example.app/Example"
                    .into(),
            }),
        });
        assert_eq!(summary.name, "Example");
        assert!(summary.is_application);
        assert!(summary.can_stop);
        assert!(summary.identity.contains("Example.app/Example"));

        let system = core_summary(ProcessToken {
            pid: 43,
            executable_url: Some(idevice::core_device::ExecutableUrl {
                relative: "/System/Library/CoreServices/SpringBoard.app/SpringBoard".into(),
            }),
        });
        assert!(system.is_application);
        assert!(!system.can_stop);
    }

    #[test]
    fn dvt_only_allows_installed_user_app_names_or_container_paths() {
        let names = HashSet::from(["example".to_string()]);
        let user = dvt_summary(
            RunningProcess {
                pid: 42,
                name: "Example".into(),
                real_app_name: String::new(),
                is_application: true,
                start_page_count: 1,
            },
            &names,
        );
        let system = dvt_summary(
            RunningProcess {
                pid: 43,
                name: "SpringBoard".into(),
                real_app_name: "/System/Library/CoreServices/SpringBoard.app/SpringBoard".into(),
                is_application: true,
                start_page_count: 1,
            },
            &names,
        );
        let sideloaded = dvt_summary(
            RunningProcess {
                pid: 44,
                name: "AppsDump".into(),
                real_app_name: "/var/containers/Bundle/Application/ID/AppsDump.app/AppsDump".into(),
                is_application: true,
                start_page_count: 1,
            },
            &HashSet::new(),
        );
        assert!(user.can_stop);
        assert!(!system.can_stop);
        assert_eq!(sideloaded.name, "AppsDump");
        assert_eq!(
            sideloaded.executable_path.as_deref(),
            Some("/var/containers/Bundle/Application/ID/AppsDump.app/AppsDump")
        );
        assert!(sideloaded.can_stop);
    }

    #[test]
    fn dvt_fallback_does_not_advertise_unverified_stop_control() {
        let (supports_launch, supports_stop, limitation) = core_capabilities(false);
        assert!(supports_launch);
        assert!(!supports_stop);
        assert!(limitation.is_some_and(|message| message.contains("DVT fallback")));

        let (_, supports_stop, limitation) = core_capabilities(true);
        assert!(supports_stop);
        assert!(limitation.is_none());
    }

    #[test]
    fn rejects_a_stale_process_identity() {
        let processes = vec![application(42, "new")];
        let error = matching_stoppable_process(&processes, 42, "old").unwrap_err();
        assert!(error.message.contains("different process"));
        assert!(error.retryable);
    }

    #[test]
    fn refuses_to_stop_system_processes() {
        let mut process = application(42, "same");
        process.can_stop = false;
        let error = matching_stoppable_process(&[process], 42, "same").unwrap_err();
        assert!(
            error
                .message
                .contains("only stops installed user applications")
        );
        assert!(!error.retryable);
    }

    #[test]
    fn rejects_invalid_bundle_identifiers() {
        assert!(validate_bundle_id("").is_err());
        assert!(validate_bundle_id("com.example/My App").is_err());
        assert!(validate_bundle_id("com.example.app").is_ok());
    }

    #[tokio::test]
    async fn device_session_cancellation_drops_an_in_flight_operation() {
        let token = CancellationToken::new();
        let cancel = token.clone();
        let operation = tokio::spawn(async move {
            run_on_local_runtime(token, || async {
                std::future::pending::<CommandResult<()>>().await
            })
            .await
        });
        cancel.cancel();

        let error = tokio::time::timeout(Duration::from_secs(2), operation)
            .await
            .expect("cancelled operation returned")
            .expect("task joined")
            .expect_err("operation should be cancelled");
        assert!(error.message.contains("device session changed"));
    }
}
