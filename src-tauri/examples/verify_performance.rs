//! Read-only real-device proof for the Performance sysmontap schema.
//!
//! Usage: cargo run --example verify_performance -- <udid>
//!
//! The harness prints the device-provided attribute order and a bounded summary
//! of three raw samples. It never launches, stops, or modifies a device process.

use std::{future::Future, path::PathBuf, time::Duration};

use idevice::{
    IdeviceError, IdeviceService, ReadWrite, RsdService,
    core_device_proxy::CoreDeviceProxy,
    dvt::{
        device_info::DeviceInfoClient,
        message::AuxValue,
        remote_server::{Channel, RemoteServerClient},
        sysmontap::SysmontapSample,
    },
    rsd::RsdHandshake,
    tcp::handle::AdapterHandle,
};
use idevice_desktop_lib::{
    device_version::{DeveloperGeneration, ios_version},
    error::{CommandError, CommandResult},
    provider::routed_provider_for,
    tunnel::open_remote_pairing_tunnel,
};
use plist::{Dictionary, Value};

const STEP_TIMEOUT: Duration = Duration::from_secs(45);
const SAMPLE_COUNT: usize = 6;

struct RsdTransport {
    adapter: AdapterHandle,
    handshake: RsdHandshake,
}

struct DiagnosticSysmontapClient<'a, R: ReadWrite> {
    channel: Channel<'a, R>,
}

impl<'a, R: ReadWrite> DiagnosticSysmontapClient<'a, R> {
    async fn new(client: &'a mut RemoteServerClient<R>) -> Result<Self, IdeviceError> {
        let channel = client
            .make_channel("com.apple.instruments.server.services.sysmontap")
            .await?;
        Ok(Self { channel })
    }

    async fn set_config(
        &mut self,
        interval_ms: u32,
        process_attributes: &[String],
        system_attributes: &[String],
    ) -> Result<(), IdeviceError> {
        let mut config = Dictionary::new();
        // Apple's sysmontap uses `ur` as output frequency, independent of the
        // sampling interval. pymobiledevice3 sends the minimum supported 1 ms.
        config.insert("ur".into(), Value::Integer(1i64.into()));
        config.insert("bm".into(), Value::Integer(0i64.into()));
        config.insert(
            "procAttrs".into(),
            Value::Array(
                process_attributes
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        );
        config.insert(
            "sysAttrs".into(),
            Value::Array(
                system_attributes
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        );
        config.insert("cpuUsage".into(), Value::Boolean(true));
        config.insert("physFootprint".into(), Value::Boolean(true));
        config.insert(
            "sampleInterval".into(),
            Value::Integer(((interval_ms as i64) * 1_000_000).into()),
        );
        self.channel
            .call_method(
                Some(Value::String("setConfig:".into())),
                Some(vec![AuxValue::archived_value(Value::Dictionary(config))]),
                false,
            )
            .await
    }

    async fn start(&mut self) -> Result<(), IdeviceError> {
        self.channel
            .call_method(Some(Value::String("start".into())), None, false)
            .await?;
        self.channel.read_message().await?;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), IdeviceError> {
        self.channel
            .call_method(Some(Value::String("stop".into())), None, false)
            .await
    }

    async fn next_sample(&mut self) -> Result<SysmontapSample, IdeviceError> {
        loop {
            let message = self.channel.read_message().await?;
            let Some(decoded) = message.data else {
                continue;
            };
            let rows = match decoded {
                Value::Array(rows) => rows,
                Value::Dictionary(row) => vec![Value::Dictionary(row)],
                _ => continue,
            };
            for row in rows {
                let Some(row) = row.into_dictionary() else {
                    continue;
                };
                if row.contains_key("Processes")
                    || row.contains_key("System")
                    || row.contains_key("SystemCPUUsage")
                {
                    return Ok(SysmontapSample {
                        processes: row.get("Processes").and_then(Value::as_dictionary).cloned(),
                        system: row.get("System").and_then(Value::as_array).cloned(),
                        system_cpu_usage: row
                            .get("SystemCPUUsage")
                            .and_then(Value::as_dictionary)
                            .cloned(),
                    });
                }
            }
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let udid = std::env::args()
        .nth(1)
        .expect("usage: verify_performance <udid>");

    if let Err(error) = run(&udid).await {
        eprintln!("RESULT: FAIL — {error:?}");
        std::process::exit(1);
    }
}

async fn run(udid: &str) -> CommandResult<()> {
    println!("== Performance verification for {udid} ==");
    let provider = timeout("connecting to the device", routed_provider_for(udid, None)).await?;
    let version = timeout("reading the iOS version", ios_version(&provider)).await?;
    println!(
        "  iOS {}.{}.{} -> {:?}",
        version.major,
        version.minor,
        version.patch,
        version.developer_generation()
    );

    let mut transport = match version.developer_generation() {
        DeveloperGeneration::Legacy => {
            return Err(CommandError::new(
                "performance",
                "Legacy sysmontap is intentionally unavailable",
                false,
            ));
        }
        DeveloperGeneration::CoreDeviceRemote => {
            let pairing_path = app_data_dir().join(format!("remote-pairing-{udid}.plist"));
            let tunnel = timeout(
                "opening the RemotePairing developer tunnel",
                open_remote_pairing_tunnel(&provider, &pairing_path, "idevice-desktop", None),
            )
            .await?;
            RsdTransport {
                adapter: tunnel.adapter,
                handshake: tunnel.handshake,
            }
        }
        DeveloperGeneration::CoreDeviceLockdown => {
            timeout("opening the CoreDeviceProxy developer tunnel", async {
                let proxy = CoreDeviceProxy::connect(&provider).await?;
                let rsd_port = proxy.tunnel_info().server_rsd_port;
                let mut adapter = proxy.create_software_tunnel()?.to_async_handle();
                let stream = adapter.connect(rsd_port).await?;
                let handshake = RsdHandshake::new(stream).await?;
                Ok::<_, idevice::IdeviceError>(RsdTransport { adapter, handshake })
            })
            .await?
        }
    };

    let mut remote = timeout(
        "opening the DVT remote server",
        RemoteServerClient::connect_rsd(&mut transport.adapter, &mut transport.handshake),
    )
    .await?;
    timeout("reading the DVT handshake", remote.read_message(0)).await?;

    let (process_attributes, system_attributes) = {
        let mut device_info =
            timeout("opening DVT DeviceInfo", DeviceInfoClient::new(&mut remote)).await?;
        let process_attributes = timeout(
            "reading process attributes",
            device_info.sysmon_process_attributes(),
        )
        .await?;
        let system_attributes = timeout(
            "reading system attributes",
            device_info.sysmon_system_attributes(),
        )
        .await?;
        (process_attributes, system_attributes)
    };

    println!("  process attributes ({}):", process_attributes.len());
    for (index, attribute) in process_attributes.iter().enumerate() {
        println!("    {index:>2}: {attribute}");
    }

    let mut sysmontap = timeout(
        "opening sysmontap",
        DiagnosticSysmontapClient::new(&mut remote),
    )
    .await?;
    timeout(
        "configuring sysmontap",
        sysmontap.set_config(1_000, &process_attributes, &system_attributes),
    )
    .await?;
    timeout("starting sysmontap", sysmontap.start()).await?;

    for sample_index in 1..=SAMPLE_COUNT {
        let sample = timeout("reading a sysmontap sample", sysmontap.next_sample()).await?;
        let process_count = sample.processes.as_ref().map_or(0, |value| value.len());
        println!("  sample {sample_index}: {process_count} process rows");
        if let Some(processes) = sample.processes {
            for (pid, value) in processes.iter().take(3) {
                let values = value.as_array();
                println!(
                    "    pid={pid}: values={} attrs={}",
                    values.map_or(0, |values| values.len()),
                    process_attributes.len()
                );
                if let Some(values) = values {
                    for attribute in [
                        "pid",
                        "name",
                        "comm",
                        "cpuUsage",
                        "avgPowerScore",
                        "physFootprint",
                        "memResidentSize",
                        "uniqueID",
                    ] {
                        if let Some(index) = process_attributes
                            .iter()
                            .position(|candidate| candidate == attribute)
                        {
                            println!("      {index:>2} {attribute}: {:?}", values.get(index));
                        }
                    }
                }
            }
        }
    }

    timeout("stopping sysmontap", sysmontap.stop()).await?;
    println!("RESULT: PASS — captured {SAMPLE_COUNT} bounded read-only samples");
    Ok(())
}

async fn timeout<T, E>(label: &str, future: impl Future<Output = Result<T, E>>) -> CommandResult<T>
where
    E: std::fmt::Display,
{
    print!("  {label} ... ");
    match tokio::time::timeout(STEP_TIMEOUT, future).await {
        Ok(Ok(value)) => {
            println!("ok");
            Ok(value)
        }
        Ok(Err(error)) => {
            println!("FAILED: {error}");
            Err(CommandError::new("performance", error.to_string(), true))
        }
        Err(_) => {
            println!("TIMED OUT");
            Err(CommandError::new(
                "performance",
                format!("Timed out while {label}"),
                true,
            ))
        }
    }
}

fn app_data_dir() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").expect("HOME"))
        .join("Library/Application Support/dev.idevice.desktop")
}
