# `idevice-tools` GUI Coverage Matrix

> Last updated: 2026-07-28
> Upstream baseline: `jkcoxson/idevice@8eed181f39a16ea70380ec8c3cff6bed07a1ef69`
> Goal: make upstream command-line capabilities safe and complete to operate through a macOS GUI.

## Status Definitions

- **Covered:** the GUI provides the primary user workflow.
- **Partial:** the backend or part of the interface exists, but the complete command capability is not exposed.
- **Not covered:** the application has no corresponding GUI workflow.
- **Design required:** the capability is risky or interaction-heavy and needs an explicit safety design first.

This matrix records capability coverage, not real-device compatibility. Real-device results belong in `PROGRESS.md`.
Delivery order and the shared usability Definition of Done are in [`FEATURE_PLAN.md`](FEATURE_PLAN.md).

## Current Coverage

| Capability group | Upstream command or service | GUI status | Current entry point or gap |
| --- | --- | --- | --- |
| Device discovery and selection | Provider and UDID selection | Partial | Unified catalog, physical USB/network transitions, device-targeted RemotePairing, paired Bonjour TCP Lockdown fallback, and keeping several attached devices apart are integrated; cold-start association remains pending |
| USB pairing | `pair` | Partial | Pair and unpair are available; advanced pairing information is incomplete |
| RemotePairing | `rppairing` | Partial | iOS 17.0–17.3 tunnels use the selected device's discovered endpoint; no dedicated pairing-management interface |
| Lockdown information | `ideviceinfo`, `lockdown`, `device_info` | Partial | Overview and Diagnostics show common fields |
| AFC files | `afc` | Covered | Browse, upload, download, create directories, and remove |
| App container files | House Arrest | Partial | File-sharing apps are supported; broader container access is pending |
| CoreDevice apps and processes | `app_service` | Partial | JIT uses launch; listing, processes, signals, and standard I/O are pending |
| Application management | `ideviceinstaller`, `instproxy`, `application_listing` | Partial | User-app list with icons and filtering, IPA installation, and uninstallation; broader installation coordination is pending |
| Crash reports | `crash_logs` | Partial | List, filter, preview, and export use Lockdown over USB and the RSD shim over iOS 17 network routes; report removal is not exposed |
| Installation coordination | `installcoordination_proxy` | Not covered | Installation sessions and diagnostics need a dedicated design |
| Device logs | `syslog_relay`, `os_trace_relay` | Covered | Live stream, pause, filter, and clear |
| Device diagnostics | `diagnostics`, `diagnosticsservice` | Partial | Battery, Gestalt, IORegistry, NAND, and Wi-Fi |
| Screenshot | `screenshot` | Covered | Device preview and refresh in Overview |
| Screen streaming | `screencapture`, `screencaptureservice` | Not covered | Planned as a dedicated live-screen tool |
| Developer Mode | `amfi` | Covered | Reveal, enable, and accept Developer Mode |
| DDI management | `mounter` | Covered | Manual and automatic mounting, unmounting, and progress |
| Debug and JIT | `debug_proxy`, `process_control` | Partial | JIT covers both generations: iOS 17 and later launch the app and attach by pid, while iOS 16 and earlier attach by process name to an app the user opened. General process control is pending |
| Location simulation | `location_simulation`, `location` | Covered | Presets, map selection, DVT/RSD, and Lockdown transports |
| SpringBoard | `springboard`, `rotate` | Partial | App icons are used; wallpaper, orientation, and other controls are not exposed |
| CoreDevice pasteboard | `pasteboard` | Not covered | Text and image read/write plus privacy guidance need design |
| CoreDevice and RSD inspection | `remotexpc` | Partial | Used internally by screenshots, JIT, and location; no service browser |
| XCTest | `xctest` | Not covered | Runner selection, status, and output are planned |

## Capabilities to Add

### Daily Developer Workflows

| Capability | Upstream command | Suggested GUI | Priority |
| --- | --- | --- | --- |
| Process control | `device_info`, `process_control`, `app_service` | Read-only DVT listing is verified on iOS 26.5; launch, stop, signals, UI, and the other generations remain pending | P0 |
| Performance overview | `sysmontap`, `energy_monitor`, `graphics` | Live metrics, process filters, time-series charts, and export | P0 |
| Packet capture | `pcapd`, `network_monitor` | Interface or process filters, start/stop, and PCAP save | P0 |
| Screen streaming | `screencapture`, `screencaptureservice` | Live image, screenshots, and recording status | P1 |
| Notification observation | `notifications`, `notification_proxy` | Subscription list, live events, and supported notification posting | P1 |
| Provisioning profiles | `misagent` | List, inspect, install, remove, and expiration warnings | P1 |
| XCTest and WDA | `xctest` | Runner selection, launch parameters, logs, and port bridging | P1 |
| Pasteboard | `pasteboard` | Text and image read/write | P1 |

### Specialized and Advanced Capabilities

| Capability | Upstream command | Suggested GUI | Status |
| --- | --- | --- | --- |
| Bluetooth packet logging | `bt_packet_logger` | Capture controls, file output, and filtering guidance | Not covered |
| Condition simulation | `condition_inducer` | Available-condition list, parameter forms, and reset | Not covered |
| HID injection | `hid` | Keyboard and touch controls with an action queue | Design required |
| Heartbeat service | `heartbeat_client` | Connection status and diagnostics | Not covered |
| Apple Watch companion service | `companion_proxy` | Paired-device and service management | Not covered |
| DVT packet parsing | `dvt_packet_parser` | File import, parsed results, and export | Not covered |
| Preboard | `preboard` | Operation panel and device status | Design required |

### High-Risk Device Lifecycle Capabilities

| Capability | Upstream command | Risk | Strategy |
| --- | --- | --- | --- |
| Activation management | `activation` | May change device activation state | Implement read-only status first; require strong confirmation for mutations |
| Backup and restore | `mobilebackup2` | Large data volume and possible data replacement | Dedicated workflow, storage validation, and recoverable progress |
| Restore mode | `restore`, `restore_service` | May cause data loss or make a device temporarily unusable | Evaluate after the initial release and provide no quick action by default |

## Coverage Acceptance Criteria

A command-line capability can be marked Covered only when all of the following are true:

1. The GUI represents its primary parameters and prerequisites.
2. Execution exposes state, progress, or continuous output, and long tasks can be stopped.
3. Success results can be inspected or exported, and failures are understandable.
4. Destructive operations include protection appropriate to their risk.
5. Device switching, disconnects, and window exit leave no background session behind.
6. The capability has passed at least a build check, with real-device verification recorded separately in `PROGRESS.md`.

## Upstream Synchronization

- This matrix follows the repository's pinned `idevice` revision and does not automatically represent the latest upstream state.
- Before changing the pinned revision, compare `tools/src/main.rs`, `tools/src/`, and crate features.
- Add new commands to this matrix before deciding page placement, risk level, and release scheduling.
- Removed or renamed upstream capabilities must be documented in release notes rather than disappearing silently.
