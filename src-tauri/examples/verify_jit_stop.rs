//! Real-device check for what ending a JIT session leaves behind.
//!
//! Usage: cargo run --example verify_jit_stop -- <udid> <bundle_id>
//!
//! `jit_stop` cancels the session token, and the worker answers by sending the
//! debugserver detach command. Nothing terminates the application, by design:
//! ending a JIT session should not close what the user is using. This confirms
//! that on a real device, by re-attaching afterwards.

use std::time::Duration;

use idevice::{
    IdeviceService, debug_proxy::DebugProxyClient, installation_proxy::InstallationProxyClient,
};
use idevice_desktop_lib::{
    device_version::{DeveloperGeneration, ios_version},
    error::{CommandError, CommandResult},
    provider::{RoutedProvider, lockdown_service_socket, routed_provider_for},
};

const DEBUGSERVER: &str = "com.apple.debugserver.DVTSecureSocketProxy";

async fn executable_name(provider: &RoutedProvider, bundle_id: &str) -> CommandResult<String> {
    let mut client = InstallationProxyClient::connect(provider).await?;
    let apps = client.get_apps(None, None).await?;
    apps.get(bundle_id)
        .and_then(|value| value.as_dictionary())
        .and_then(|dict| dict.get("CFBundleExecutable"))
        .and_then(|value| value.as_string())
        .map(ToOwned::to_owned)
        .ok_or_else(|| CommandError::new("jit", "not installed", false))
}

/// Attaches by name and reports whether debugserver accepted it.
async fn attach(
    provider: &RoutedProvider,
    executable: &str,
) -> CommandResult<(DebugProxyClient<Box<dyn idevice::ReadWrite>>, bool)> {
    let socket = lockdown_service_socket(provider, DEBUGSERVER).await?;
    let mut debug = DebugProxyClient::new(socket);
    let encoded: String = executable
        .bytes()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let reply = tokio::time::timeout(
        Duration::from_secs(20),
        debug.send_command(format!("vAttachName;{encoded}").into()),
    )
    .await
    .map_err(|_| CommandError::new("jit", "attach timed out", true))??;
    let text = reply.unwrap_or_default();
    Ok((debug, text.starts_with('T') || text.starts_with('S')))
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut args = std::env::args().skip(1);
    let udid = args
        .next()
        .expect("usage: verify_jit_stop <udid> <bundle_id>");
    let bundle_id = args.next().expect("bundle id");

    let provider = routed_provider_for(&udid, None).await.expect("provider");
    let version = ios_version(&provider).await.expect("version");
    if version.developer_generation() != DeveloperGeneration::Legacy {
        println!("This check covers the legacy attach-by-name path; run it on iOS 16 or earlier.");
        return;
    }
    let executable = executable_name(&provider, &bundle_id)
        .await
        .expect("executable");
    println!("target: {bundle_id} ({executable})\n");

    let (mut debug, attached) = attach(&provider, &executable).await.expect("attach");
    println!(
        "1. attach              {}",
        if attached {
            "ok"
        } else {
            "REJECTED (is the app running?)"
        }
    );
    if !attached {
        return;
    }

    // This is what jit_stop triggers once the session token is cancelled.
    let detached = debug.send_command("D".into()).await;
    println!(
        "2. detach (jit_stop)   {}",
        if detached.is_ok() { "ok" } else { "failed" }
    );
    drop(debug);

    tokio::time::sleep(Duration::from_secs(1)).await;

    // If the app survived, debugserver can attach to it again.
    match attach(&provider, &executable).await {
        Ok((mut again, true)) => {
            println!("3. app still running   ok (re-attached)");
            let _ = again.send_command("D".into()).await;
            println!("\nRESULT: PASS — the session detached and left the app running");
        }
        Ok((_, false)) => println!("\nRESULT: FAIL — the app is gone after detaching"),
        Err(error) => println!("\nRESULT: FAIL — re-attach errored: {error:?}"),
    }
}
