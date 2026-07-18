use std::{net::SocketAddr, path::Path, time::Duration};

use idevice::{
    IdeviceService,
    lockdown::LockdownClient,
    provider::IdeviceProvider,
    remote_pairing::{
        RemotePairingClient, RpPairingFile, RpPairingSocket, connect_tls_psk_tunnel_native,
    },
    rsd::RsdHandshake,
    tcp::handle::AdapterHandle,
};
use mdns_sd::{ServiceDaemon, ServiceEvent};
use tauri::Manager;
use tokio::net::TcpStream;

use crate::error::{CommandError, CommandResult};

pub struct RsdTunnel {
    pub adapter: AdapterHandle,
    pub handshake: RsdHandshake,
}

pub fn remote_pairing_path(
    app: &tauri::AppHandle,
    udid: &str,
) -> CommandResult<std::path::PathBuf> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| CommandError::new("configuration", error.to_string(), false))?;
    Ok(directory.join(format!("remote-pairing-{udid}.plist")))
}

async fn connect_host(hostname: &str, port: u16) -> CommandResult<TcpStream> {
    let addresses: Vec<SocketAddr> =
        tokio::net::lookup_host((hostname.trim_end_matches('.'), port))
            .await
            .map_err(|error| CommandError::new("tunnel", error.to_string(), true))?
            .collect();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect(address).await {
            Ok(stream) => return Ok(stream),
            Err(error) => last_error = Some(error),
        }
    }
    Err(CommandError::new(
        "tunnel",
        format!(
            "Unable to connect to {hostname}:{port}: {}",
            last_error
                .map(|error| error.to_string())
                .unwrap_or_else(|| "Bonjour returned no address".into())
        ),
        true,
    ))
}

async fn discover_remote_pairing() -> CommandResult<(String, u16)> {
    let daemon = ServiceDaemon::new()
        .map_err(|error| CommandError::new("bonjour", error.to_string(), true))?;
    let receiver = daemon
        .browse("_remotepairing._tcp.local.")
        .map_err(|error| CommandError::new("bonjour", error.to_string(), true))?;
    let result = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            match receiver.recv_async().await {
                Ok(ServiceEvent::ServiceResolved(info)) => {
                    return Ok((info.get_hostname().to_owned(), info.get_port()));
                }
                Ok(_) => {}
                Err(error) => {
                    return Err(CommandError::new("bonjour", error.to_string(), true));
                }
            }
        }
    })
    .await
    .map_err(|_| {
        CommandError::new(
            "bonjour",
            "Timed out discovering the iOS 17.0–17.3 RemotePairing service",
            true,
        )
    })?;
    let _ = daemon.shutdown();
    result
}

async fn bootstrap_remote_pairing(
    provider: &impl IdeviceProvider,
    pairing_path: &Path,
    host_identifier: &str,
) -> CommandResult<()> {
    let mut lockdown = LockdownClient::connect(provider)
        .await
        .map_err(CommandError::from)?;
    let lockdown_pairing = provider
        .get_pairing_file()
        .await
        .map_err(CommandError::from)?;
    let legacy_tls = lockdown
        .start_session(&lockdown_pairing)
        .await
        .map_err(CommandError::from)?;
    let (port, ssl) = lockdown
        .start_service("com.apple.dt.remotepairingdeviced.lockdown")
        .await
        .map_err(|error| {
            CommandError::new(
                "pairing",
                format!("Unable to start the USB RemotePairing bootstrap service: {error}"),
                true,
            )
        })?;
    let mut service = provider.connect(port).await.map_err(CommandError::from)?;
    if ssl {
        service
            .start_session(&lockdown_pairing, legacy_tls)
            .await
            .map_err(CommandError::from)?;
    }
    let socket = service.get_socket().ok_or_else(|| {
        CommandError::new(
            "pairing",
            "RemotePairing bootstrap returned no socket",
            true,
        )
    })?;
    let mut pairing = RpPairingFile::generate(host_identifier);
    let mut client = RemotePairingClient::new(RpPairingSocket::new(socket), host_identifier);
    client
        .connect(&mut pairing, async || "000000".to_owned())
        .await
        .map_err(|error| {
            CommandError::new(
                "pairing",
                format!("USB RemotePairing bootstrap failed: {error}"),
                true,
            )
        })?;
    if let Some(parent) = pairing_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    pairing
        .write_to_file(pairing_path)
        .await
        .map_err(CommandError::from)
}

pub async fn open_remote_pairing_tunnel(
    provider: &impl IdeviceProvider,
    pairing_path: &Path,
    host_identifier: &str,
) -> CommandResult<RsdTunnel> {
    if RpPairingFile::read_from_file(pairing_path).await.is_err() {
        let mut last_error = None;
        for attempt in 0..3 {
            match bootstrap_remote_pairing(provider, pairing_path, host_identifier).await {
                Ok(()) => {
                    last_error = None;
                    break;
                }
                Err(error) => {
                    last_error = Some(error);
                    if attempt < 2 {
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                }
            }
        }
        if let Some(error) = last_error {
            return Err(error);
        }
    }
    let (hostname, pairing_port) = discover_remote_pairing().await?;
    let pairing_stream = connect_host(&hostname, pairing_port).await?;
    let pairing_socket = RpPairingSocket::new(pairing_stream);

    let mut pairing = RpPairingFile::read_from_file(pairing_path)
        .await
        .map_err(CommandError::from)?;
    let mut client = RemotePairingClient::new(pairing_socket, host_identifier);
    client
        .connect(&mut pairing, async || "000000".to_owned())
        .await
        .map_err(|error| {
            CommandError::new("pairing", format!("RemotePairing failed: {error}"), true)
        })?;
    let tunnel_port = client.create_tcp_listener().await.map_err(|error| {
        CommandError::new(
            "tunnel",
            format!("Creating the RemotePairing tunnel listener failed: {error}"),
            true,
        )
    })?;
    let tunnel_stream = connect_host(&hostname, tunnel_port).await?;
    let tunnel = connect_tls_psk_tunnel_native(tunnel_stream, client.encryption_key())
        .await
        .map_err(|error| {
            CommandError::new(
                "tunnel",
                format!("TLS-PSK tunnel handshake failed: {error}"),
                true,
            )
        })?;
    let client_ip = tunnel
        .info
        .client_address
        .parse()
        .map_err(CommandError::from)?;
    let server_ip = tunnel
        .info
        .server_address
        .parse()
        .map_err(CommandError::from)?;
    let rsd_port = tunnel.info.server_rsd_port;
    let mtu = tunnel.info.mtu as usize;
    let mut adapter =
        idevice::tcp::adapter::Adapter::new(Box::new(tunnel.into_inner()), client_ip, server_ip);
    adapter.set_mss(mtu.saturating_sub(60));
    let mut adapter = adapter.to_async_handle();
    let rsd_stream = adapter
        .connect(rsd_port)
        .await
        .map_err(|error| CommandError::new("tunnel", error.to_string(), true))?;
    let handshake = RsdHandshake::new(rsd_stream).await.map_err(|error| {
        CommandError::new(
            "tunnel",
            format!("Tunneled RSD handshake failed: {error}"),
            true,
        )
    })?;
    Ok(RsdTunnel { adapter, handshake })
}
