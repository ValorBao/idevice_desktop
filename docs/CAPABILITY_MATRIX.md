# `idevice-tools` GUI Coverage Matrix

> Last updated: 2026-08-10
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
| CoreDevice apps and processes | `app_service` | Partial | Monitor exposes search and refresh through AppService or DVT, application launch on both protocol routes, and confirmed stop only when AppService is advertised. The complete iOS 17.0 Tauri AppService workflow is accepted; DVT stop is explicitly read-only after `killPid:` failed to terminate an identity-checked test process. The DVT and Legacy visible limitations remain pending. Standard I/O is outside the first slice |
| Application management | `ideviceinstaller`, `instproxy`, `application_listing` | Partial | User-app list with icons and filtering, IPA installation, and uninstallation; broader installation coordination is pending |
| Provisioning profiles | `misagent` | Partial | A dedicated read-only page lists profiles through USB Lockdown or modern RSD, parses signing scope, team, application identifier, registered-device coverage, debug entitlement, and expiration state, and exposes search plus attention/development/distribution filters. Malformed profiles remain visible without failing the list. Automated transport, parsing, expiry, and interface checks pass; hardware acceptance, cryptographic signer-chain verification, install, and confirmed removal remain pending |
| Crash reports | `crash_logs` | Partial | List, filter, preview, and export use Lockdown over USB and the RSD shim over iOS 17 network routes; report removal is not exposed |
| Installation coordination | `installcoordination_proxy` | Not covered | Installation sessions and diagnostics need a dedicated design |
| Device logs | `syslog_relay`, `os_trace_relay` | Covered | Live stream, pause, filter, and clear |
| Performance sampling | `sysmontap` | Partial | Monitor exposes iOS 17+ system/process CPU and memory sampling, stable-identity rolling history, filtering, pause/resume, bounded retention, and CSV export. Automated lifecycle and parsing checks pass; real-device acceptance is pending. Energy and graphics metrics are outside this slice, and Legacy devices show an explicit unavailable state |
| Packet capture | `pcapd` | Partial | Monitor streams device packets directly into an adjacent partial file, exposes packet/byte/duration progress and optional PID/interface filters, then atomically saves a Wireshark-compatible PCAP or deletes it on cancel. USB and modern RSD routes are integrated with disk and size safeguards; real-device acceptance is pending |
| Notification observation | `notification_proxy` | Partial | Monitor requires an explicit preset or custom subscription list, shows a searchable newest-first timeline, bounds retained names to 500, isolates restarted sessions, and stops on pause, page exit, device switch, or disconnect. USB Lockdown and modern RSD routes are integrated; the upstream client relays names rather than payloads. Posting is not exposed and hardware acceptance is pending |
| Device diagnostics | `diagnostics`, `diagnosticsservice` | Partial | Battery, Gestalt, IORegistry, NAND, and Wi-Fi |
| Screenshot | `screenshot` | Covered | Device preview and refresh in Overview |
| Screen streaming | `screenshot`, DVT screenshot | Partial | A dedicated Live Screen tool refreshes PNG frames at a 2 FPS target across Legacy Screenshotr and modern DVT/RSD routes, reports measured frame rate and resolution, supports fit/100% scaling and still export, and tears down on stop, page exit, hidden window, device switch, or disconnect. Automated lifecycle and validation checks pass; hardware acceptance and high-frame-rate HEVC streaming remain pending |
| Developer Mode | `amfi` | Covered | Reveal, enable, and accept Developer Mode |
| DDI management | `mounter` | Covered | Manual and automatic mounting, unmounting, and progress |
| Debug and JIT | `debug_proxy`, `process_control` | Partial | JIT covers both generations: iOS 17 and later launch the app and attach by pid, while iOS 16 and earlier attach by process name to an app the user opened. Monitor exposes application launch on modern routes and identity-checked stop through AppService; interface acceptance remains pending |
| Location simulation | `location_simulation`, `location` | Covered | Presets, map selection, DVT/RSD, and Lockdown transports |
| SpringBoard | `springboard`, `rotate` | Partial | App icons are used; wallpaper, orientation, and other controls are not exposed |
| CoreDevice pasteboard | `pasteboard` | Partial | A dedicated iOS 17+ page performs user-triggered UTF-8 text and PNG/JPEG image reads plus confirmed replacement writes through RemotePairing or CoreDeviceProxy RSD. Reads promise all payloads and resolve only the selected supported type when its advertised size is known and within the 1 MB text or 12 MB image limit. Image writes are locally validated, previewed, bound to the selected device through a one-use preparation token, and cleared on exit. There is no background monitoring, host clipboard access, TIFF, or transcoding; hardware acceptance remains pending |
| CoreDevice and RSD inspection | `remotexpc` | Partial | Used internally by screenshots, JIT, and location; no service browser |
| XCTest | `xctest` | Partial | Test Lab performs bounded read-only discovery of installed `.xctrunner` candidates and optional target apps, checks required path/container/`-Runner` metadata plus `get-task-allow`, and combines the selection with Developer Mode, DDI, and RSD readiness. It also validates a non-executing Standard XCTest or WDA plan against freshly read metadata, normalizing bounded include/skip filters, target, timeout, and WDA bridge intent. Legacy Lockdown and iOS 17.4+ CoreDeviceProxy execution routes are identified; iOS 17.0–17.3 is explicitly blocked until a RemotePairing/RSD adapter exists. No process is launched; event output, runtime timeout enforcement, Stop/cleanup, WDA networking, and hardware acceptance remain pending |

## Capabilities to Add

### Daily Developer Workflows

| Capability | Upstream command | Suggested GUI | Priority |
| --- | --- | --- | --- |
| Process control | `device_info`, `process_control`, `app_service` | Monitor workflow integrated with search, refresh, launch, AppService-only confirmed stop, stale-PID protection, and explicit Legacy/DVT stop limitations. DVT read-only listing is verified on iOS 26.5; the complete Tauri AppService list/launch/confirmation/stop/cleanup path is accepted on iOS 17.0. DVT and Legacy visible limitations remain pending | P0 |
| Performance overview | `sysmontap`, `energy_monitor`, `graphics` | Sysmontap CPU/memory workflow integrated with process filters, rolling charts, pause/resume, and CSV export; hardware acceptance and later energy/graphics scope remain | P0 |
| Packet capture | `pcapd`, `network_monitor` | Pcapd workflow integrated with destination selection, PID/interface filters, live statistics, stop-and-save, cancel-and-delete, and disk safeguards; hardware acceptance and richer Network Monitor events remain | P0 |
| Screen streaming | `screencapture`, `screencaptureservice` | Compatible PNG-refresh preview and still export are integrated; hardware acceptance, HEVC live video, and recording status remain | P1 |
| Notification observation | `notifications`, `notification_proxy` | Read-only subscription list and bounded live name timeline are integrated; hardware acceptance and any separately designed posting workflow remain | P1 |
| Provisioning profiles | `misagent` | Read-only list, inspection, search, and expiration warnings are integrated; hardware acceptance, install, and confirmed removal remain | P1 |
| XCTest and WDA | `xctest` | Read-only runner/target selection, prerequisite inspection, and validated Standard XCTest/WDA plan previews are integrated; process launch, event output, timeout enforcement, Stop/cleanup, RemotePairing support, WDA readiness, and localhost port bridging remain | P1 |
| Pasteboard | `pasteboard` | Bounded text and PNG/JPEG read/write are integrated with local image preview, device-bound preparation, confirmation, privacy guidance, and cleanup; hardware acceptance and broader formats remain | P1 |

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
