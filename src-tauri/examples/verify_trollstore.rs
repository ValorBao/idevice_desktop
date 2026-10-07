//! Real-device check for the TrollStore page.
//!
//! Usage:
//!   cargo run --example verify_trollstore -- <udid>
//!   cargo run --example verify_trollstore -- <udid> --probe-url
//!
//! The default run is read-only: version gate, whether TrollStore is installed,
//! and which removable system apps the helper restore could use.
//!
//! `--probe-url` needs TrollStore installed. It serves only a 404 on this Mac's
//! LAN address and opens TrollStore with an install URL pointing at it. No IPA
//! exists at that URL, so nothing is installed; the phone shows a download
//! error. The point is to see whether the request reaches the Mac at all,
//! which answers both "does --payload-url reach TrollStore" and "does the
//! phone allow cleartext http to the LAN".

use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use idevice::{
    IdeviceService, installation_proxy::InstallationProxyClient, lockdown::LockdownClient,
    provider::IdeviceProvider,
};
use idevice_desktop_lib::{
    provider::routed_provider_for,
    trollstore::{
        TROLLSTORE_BUNDLE_ID, helper_install_caution, helper_install_refusal,
        is_replaceable_apple_app, removable_bundle_name,
    },
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

const PROBE_WAIT: Duration = Duration::from_secs(45);

#[tokio::main]
async fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let Some(udid) = args.first().cloned() else {
        eprintln!("usage: verify_trollstore <udid> [--probe-url]");
        std::process::exit(2);
    };
    let probe = args.iter().any(|arg| arg == "--probe-url");

    let provider = routed_provider_for(&udid, None).await.expect("USB route");
    let mut lockdown = LockdownClient::connect(&provider).await.expect("lockdown");
    let pairing = provider.get_pairing_file().await.expect("pairing");
    lockdown.start_session(&pairing).await.expect("session");
    let version = lockdown
        .get_value(Some("ProductVersion"), None)
        .await
        .ok()
        .and_then(|value| value.into_string())
        .unwrap_or_default();
    let build = lockdown
        .get_value(Some("BuildVersion"), None)
        .await
        .ok()
        .and_then(|value| value.into_string())
        .unwrap_or_default();
    println!("device: iOS {version} ({build})");
    match helper_install_refusal(&version, &build) {
        None => println!("helper gate: in range"),
        Some(reason) => println!("helper gate: refused — {reason}"),
    }
    if let Some(caution) = helper_install_caution(&version) {
        println!("helper caution: {caution}");
    }

    let mut proxy = InstallationProxyClient::connect(&provider)
        .await
        .expect("installation proxy");
    let apps = proxy.get_apps(None, None).await.expect("app lookup");
    let installed = apps.contains_key(TROLLSTORE_BUNDLE_ID);
    println!("registered apps: {}", apps.len());
    println!("TrollStore installed: {installed}");
    let mut removable = apps
        .iter()
        .filter_map(|(bundle_id, value)| {
            let dict = value.as_dictionary()?;
            let path = dict.get("Path")?.as_string()?;
            let bundle = removable_bundle_name(path)?;
            let kind = dict
                .get("ApplicationType")
                .and_then(|value| value.as_string());
            is_replaceable_apple_app(bundle_id, kind).then(|| format!("{bundle_id} ({bundle})"))
        })
        .collect::<Vec<_>>();
    removable.sort();
    println!("removable system apps: {}", removable.len());
    for app in removable.iter().take(40) {
        println!("  {app}");
    }

    if !probe {
        return;
    }
    if !installed {
        println!("probe: skipped, TrollStore is not installed");
        return;
    }
    let address = lan_ipv4();
    let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(address), 0))
        .await
        .expect("bind probe listener");
    let port = listener.local_addr().unwrap().port();
    let ipa_url = format!(
        "http://{address}:{port}/probe-{}/missing.ipa",
        uuid::Uuid::new_v4().simple()
    );
    let payload = format!("apple-magnifier://install?url={}", encode(&ipa_url));
    println!("probe: serving 404 at {ipa_url}");

    let server = tokio::spawn(async move {
        let accepted = tokio::time::timeout(PROBE_WAIT, listener.accept()).await;
        match accepted {
            Ok(Ok((mut stream, peer))) => {
                let mut buffer = vec![0u8; 2048];
                let read = stream.read(&mut buffer).await.unwrap_or(0);
                let first = String::from_utf8_lossy(&buffer[..read])
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_owned();
                let _ = stream
                    .write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    )
                    .await;
                Some(format!("{peer} -> {first}"))
            }
            _ => None,
        }
    });

    let output = tokio::process::Command::new("/usr/bin/xcrun")
        .args([
            "devicectl",
            "device",
            "process",
            "launch",
            "--device",
            &udid,
            "--terminate-existing",
            "--payload-url",
            &payload,
            TROLLSTORE_BUNDLE_ID,
        ])
        .output()
        .await
        .expect("run devicectl");
    println!(
        "probe: devicectl exit {} {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
            .lines()
            .last()
            .unwrap_or("")
    );
    match server.await.ok().flatten() {
        Some(line) => println!("probe: REQUEST RECEIVED {line}"),
        None => println!("probe: no request within {}s", PROBE_WAIT.as_secs()),
    }
}

fn lan_ipv4() -> std::net::Ipv4Addr {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").expect("udp");
    socket.connect("1.1.1.1:80").expect("route");
    match socket.local_addr().expect("local").ip() {
        IpAddr::V4(address) => address,
        IpAddr::V6(_) => panic!("no IPv4 LAN address"),
    }
}

fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}
