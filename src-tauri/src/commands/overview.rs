use idevice::{
    IdeviceService,
    afc::AfcClient,
    diagnostics_relay::DiagnosticsRelayClient,
    lockdown::LockdownClient,
    provider::IdeviceProvider,
    usbmuxd::{Connection, UsbmuxdConnection},
};
use tauri::State;

use crate::{
    error::{CommandError, CommandResult},
    provider::selected_provider,
    state::AppState,
    types::{BatterySummary, DeviceOverview, StorageSummary},
    utils::{dict_f64, dict_string, dict_u64, plist_to_json},
};

fn health_percent(dict: &plist::Dictionary) -> Option<u64> {
    dict_u64(
        dict,
        &["MaximumCapacityPercent", "BatteryHealthMetric"],
    )
    .filter(|value| *value <= 100)
    .or_else(|| {
        let maximum = dict_u64(
            dict,
            &[
                "AppleRawMaxCapacity",
                "NominalChargeCapacity",
                "FullChargeCapacity",
                "MaxCapacity",
            ],
        )?;
        let design = dict_u64(dict, &["DesignCapacity"])?;
        (maximum > 100 && design > 0)
            .then(|| ((maximum as f64 / design as f64) * 100.0).round() as u64)
            .filter(|value| *value <= 100)
    })
}

fn nested_dict<'a>(dict: &'a plist::Dictionary, key: &str) -> &'a plist::Dictionary {
    dict.get(key)
        .and_then(plist::Value::as_dictionary)
        .unwrap_or(dict)
}

async fn connection_for(udid: &str) -> String {
    let Ok(mut mux) = UsbmuxdConnection::default().await else {
        return "Unknown".into();
    };
    let Ok(device) = mux.get_device(udid).await else {
        return "Unknown".into();
    };
    match device.connection_type {
        Connection::Usb => "USB".into(),
        Connection::Network(address) => format!("Network · {address}"),
        Connection::Unknown(value) => value,
    }
}

#[tauri::command]
pub async fn overview_get(
    state: State<'_, AppState>,
    udid: Option<String>,
) -> CommandResult<DeviceOverview> {
    let (udid, provider) = selected_provider(&state, udid).await?;
    let pairing_file = provider
        .get_pairing_file()
        .await
        .map_err(CommandError::from)?;
    let mut lockdown = LockdownClient::connect(&provider)
        .await
        .map_err(CommandError::from)?;
    lockdown
        .start_session(&pairing_file)
        .await
        .map_err(CommandError::from)?;
    let values = lockdown
        .get_value(None, None)
        .await
        .map_err(CommandError::from)?;
    let values = values.as_dictionary().ok_or_else(|| {
        CommandError::new(
            "protocol",
            "Lockdown returned a non-dictionary value",
            false,
        )
    })?;

    let battery_value = match DiagnosticsRelayClient::connect(&provider).await {
        Ok(mut client) => client.gasguage().await.ok().flatten(),
        Err(_) => None,
    };
    let battery_root = battery_value.clone().unwrap_or_default();
    let battery_dict = nested_dict(&battery_root, "GasGauge");
    let battery_domain = lockdown
        .get_value(None, Some("com.apple.mobile.battery"))
        .await
        .ok()
        .and_then(|value| value.into_dictionary())
        .unwrap_or_default();
    let level = dict_u64(
        battery_dict,
        &[
            "CurrentCapacity",
            "BatteryCurrentCapacity",
            "AbsoluteCapacity",
        ],
    )
    .or_else(|| dict_u64(&battery_domain, &["BatteryCurrentCapacity"]))
    .or_else(|| dict_u64(values, &["BatteryCurrentCapacity"]));
    let temperature = dict_f64(battery_dict, &["Temperature", "BatteryTemperature"])
        .map(|value| if value > 100.0 { value / 100.0 } else { value });
    let voltage = dict_f64(battery_dict, &["Voltage", "BatteryVoltage"])
        .map(|value| if value > 100.0 { value / 1000.0 } else { value });

    let storage = match AfcClient::connect(&provider).await {
        Ok(mut afc) => afc.get_device_info().await.ok().map(|info| StorageSummary {
            total_bytes: info.total_bytes as u64,
            free_bytes: info.free_bytes as u64,
            used_bytes: info.total_bytes.saturating_sub(info.free_bytes) as u64,
            block_size: info.block_size as u64,
        }),
        Err(_) => None,
    };

    Ok(DeviceOverview {
        udid: udid.clone(),
        name: dict_string(values, &["DeviceName"]),
        product_type: dict_string(values, &["ProductType"]),
        product_version: dict_string(values, &["ProductVersion"]),
        build_version: dict_string(values, &["BuildVersion"]),
        serial_number: dict_string(values, &["SerialNumber"]),
        unique_chip_id: dict_string(values, &["UniqueChipID"])
            .or_else(|| dict_u64(values, &["UniqueChipID"]).map(|value| format!("0x{value:X}"))),
        hardware_model: dict_string(values, &["HardwareModel"]),
        hardware_platform: dict_string(values, &["HardwarePlatform", "CPUArchitecture"]),
        wifi_address: dict_string(values, &["WiFiAddress"]),
        connection: connection_for(&udid).await,
        paired: true,
        battery: BatterySummary {
            level,
            health_percent: health_percent(battery_dict),
            cycle_count: dict_u64(battery_dict, &["CycleCount", "BatteryCycleCount"]),
            temperature_celsius: temperature,
            voltage_volts: voltage,
            raw: battery_value
                .as_ref()
                .map(|value| plist_to_json(&plist::Value::Dictionary(value.clone())))
                .unwrap_or(serde_json::Value::Null),
        },
        storage,
    })
}
