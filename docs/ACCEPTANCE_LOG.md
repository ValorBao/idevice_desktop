# Real-Device Acceptance Log

This is the evidence log for the hardware-acceptance gate in
[`FEATURE_PLAN.md`](FEATURE_PLAN.md). A backend harness result proves the device
transport; a frontend regression proves the visible workflow and cleanup calls.
Neither is recorded as complete desktop-interface acceptance on its own.

## 2026-07-26 — current-surface pass

| Device | iOS | Connection | Workflow | Result | Evidence / cleanup |
| --- | --- | --- | --- | --- | --- |
| Three attached phones | 14.2, 17.0, 26.5 | USB + network | Discovery and identity | Pass | Three distinct paired catalog entries; no duplicate UDIDs |
| `90384b8…` (iPhone10,1) | 14.2 | USB | Crash Reports transport | Pass | Legacy Lockdown opened and listed 678 root entries; read-only |
| `00008110…` | 26.5 | Network | Crash Reports transport | Pass | CoreDeviceProxy exposed the RSD crash shim and listed 23 root entries; read-only |
| `00008020…` (iPhone11,8) | 17.0 | Network | Locked-device failure | Pass | Lockdown returned `device is locked` immediately instead of leaving a loading state |
| `90384b8…` (iPhone10,1) | 14.2 | USB | JIT candidate discovery | Pass | All 127 registered applications were inspected; two `get-task-allow` candidates were reachable by the selector |
| `00008110…` | 26.5 | Network | Device disappears before JIT discovery | Pass (failure path) | The operation returned `no active usbmuxd or Bonjour Lockdown route`; a fresh discovery showed the device was no longer present |
| Frontend regression | n/a | mocked desktop boundary | Location main and cleanup paths | Pass | Start uses the selected UDID and coordinates; leaving the page calls `location_stop` |
| Frontend regression | n/a | mocked desktop boundary | JIT main and cleanup paths | Pass | Selector uses a debuggable app; leaving the page calls `jit_stop` |
| Frontend regression | n/a | mocked desktop boundary | Large crash preview | Pass | A 6 MB result is marked as a 4 MB preview while Export requests the original report |
| Frontend regression | n/a | mocked desktop boundary | Device switch | Pass | The active page remounts for the new UDID, clearing old device state and running page cleanup |

### Defect fixed during the pass

The active page was keyed only by page name. Switching devices therefore kept
component state from the previous phone even though the backend cancelled its task.
The page session is now keyed by both page and UDID, so Location, JIT, Crash Reports,
Files, and the other device-bound pages start with state belonging to the new phone.

### Still required before the Foundation gate closes

- Complete Location set/clear through the desktop interface on Legacy.
- Complete JIT attach/detach through the desktop interface on Legacy.
- Preview and export an actual report larger than 4 MB.
- Exercise sleep/wake association through the full discovery catalog.
- Disconnect a device while a Location, JIT, crash read, and long-running log task is active.

The local macOS session did not grant assistive-access control to the acceptance
runner. That prevents an automated click-through from being counted as desktop-
interface evidence; it is an environment limitation, not a product pass or failure.

## 2026-07-28 — Processes protocol proof

| Device | iOS | Connection | Workflow | Result | Evidence / cleanup |
| --- | --- | --- | --- | --- | --- |
| `00008110…` (iPhone14,5) | 26.5 | usbmuxd network record | Read-only process list | Pass | CoreDeviceProxy exposed 62 RSD services. `com.apple.coredevice.appservice` was absent, so the harness used `com.apple.instruments.dtservicehub`; DVT DeviceInfo returned 220 running processes. No process was changed |

This proves the read-only backend path for the CoreDeviceLockdown generation. It
does not satisfy Processes interface acceptance. Launch, PID verification, stop,
and cleanup still require an explicit mutating harness run on this generation.
CoreDeviceRemote and Legacy remained to be probed at the end of this session.

## 2026-08-04 — iOS 17 Processes protocol proof

| Device | iOS | Connection | Workflow | Result | Evidence / cleanup |
| --- | --- | --- | --- | --- | --- |
| `00008020…` (iPhone11,8) | 17.0 | USB + RemotePairing | Pairing and route prerequisite | Pass | usbmuxd reported one USB device with an existing pair record; paired Lockdown returned `ProductVersion=17.0` |
| `00008020…` (iPhone11,8) | 17.0 | RemotePairing/RSD | Read-only process list | Pass | RSD advertised both AppService and DVT; the harness correctly preferred CoreDevice AppService and returned 473 processes |
| `00008020…` (iPhone11,8) | 17.0 | RemotePairing/RSD | Launch, PID visibility, stop, and cleanup | Pass | AppService launched `cn.gblw.AppsDump` as new pid 1522, the next list contained it, SIGTERM succeeded, and the final list confirmed it exited |
| `00008020…` (iPhone11,8) | 17.0 | RemotePairing/RSD | JIT transport recheck | Pass | The JIT harness launched the same test app as pid 1533, disabled its memory limit, received a successful `T11` attach reply, detached, and terminated only the launched process |

This completes the Processes protocol proof for CoreDeviceRemote, including its
mutating cleanup path. It does not add a visible Processes interface or count as
desktop-interface acceptance. CoreDeviceLockdown still needs the mutating path and
Legacy still needs a support boundary result.

## 2026-08-05 — iOS 17 Location desktop acceptance

| Device | iOS | Connection | Workflow | Result | Evidence / cleanup |
| --- | --- | --- | --- | --- | --- |
| `00008020…` (iPhone11,8) | 17.0 | USB + RemotePairing/RSD | Location preset set and clear through the desktop interface | Pass | The user selected Apple Park and started simulation; the interface reported an active DVT/RSD override and the phone reflected the simulated position. Stop returned the interface to real GPS and the phone restored its real location |

This closes the current-surface Location main path for CoreDeviceRemote. Legacy
still needs the same desktop-interface click-through; its lower-level Lockdown
set, reconnect-for-clear, and real-GPS restoration path already passed on iOS 14.2.

## 2026-08-08 — Processes workflow integration

| Device | iOS | Connection | Workflow | Result | Evidence / cleanup |
| --- | --- | --- | --- | --- | --- |
| Frontend regression | n/a | mocked desktop boundary | List and support boundary | Pass | The selected UDID loads the process snapshot; installed-user-application and protected system states render separately; Legacy returns an explicit unsupported state with no dead controls |
| Frontend regression | n/a | mocked desktop boundary | Launch and stop | Pass | Launch targets the selected device and refreshes; stop requires destructive confirmation and returns the row's opaque process identity with its PID and UDID |
| Frontend regression | n/a | mocked DVT boundary | Read-only stop limitation | Pass | DVT process rows remain visible and launch stays available, while the unverified Stop action is replaced by `Read only` and the interface explains how AppService becomes available |
| Rust regression | n/a | pure decision boundary | Stop and lifecycle safety | Pass | A changed process identity is rejected as stale; AppService and DVT user-container paths plus DVT Installation Proxy names distinguish installed user applications from system applications; malformed bundle identifiers are rejected before device I/O; device-session cancellation drops an in-flight process operation |
| Browser demonstration | n/a | mock data | Monitor layout and launch interaction | Pass (UI only) | Processes rendered without overflow at the default viewport and 820×650; search, protected rows, and demo launch state were visible; no browser warnings or errors were recorded |
| `00008110…` (iPhone14,5) | 26.5 | usbmuxd network record | DVT read-only regression | Pass | CoreDeviceProxy opened with 62 RSD services; AppService was absent; DVT DeviceInfo returned 277 running processes. No process was launched, signalled, or stopped |
| `00008110…` (iPhone14,5) | 26.5 | route disappeared | Production list failure path | Pass (failure path) | The production service-layer harness returned `no active usbmuxd or Bonjour Lockdown route` after the device disappeared instead of selecting another target; no process operation was attempted |
| `00008020…` (iPhone11,8) | 17.0 | USB + RemotePairing/RSD | Production list and DVT safety classification | Pass | Before AppService was advertised, the production service selected DVT DeviceInfo, returned 419 processes, mapped executable paths from DVT's `realAppName`, and exposed only user-container or Installation Proxy-matched applications as stoppable |
| `00008020…` (iPhone11,8) | 17.0 | DVT ProcessControl | DVT fallback stop | Blocked | `killPid:` accepted two identity-checked requests for the dedicated `cn.gblw.AppsDump` pid 3451, but the PID remained listed. No daily application was signalled. Apple `devicectl --kill` removed the test process and enabled the developer-image services, proving the process was terminable and exposing AppService |
| `00008020…` (iPhone11,8) | 17.0 | CoreDevice AppService · RemotePairing/RSD | Production list, launch, identity check, stop, and cleanup | Pass | The production service-layer harness listed 441 processes, launched only `cn.gblw.AppsDump` as new pid 4096, found its user-container identity in a fresh list, sent SIGTERM, and confirmed the final list no longer contained the PID |

These checks establish the visible workflow and safety contract. They do not count
as real-device desktop-interface acceptance. The production AppService command path
now has a full iOS 17.0 hardware pass. The Monitor interface still needs a desktop
click-through, and the iOS 26.5 device must visibly confirm the DVT read-only
limitation. DVT Stop is intentionally not advertised when AppService is unavailable.

## 2026-08-09 — iOS 17 Processes desktop acceptance

| Device | iOS | Connection | Workflow | Result | Evidence / cleanup |
| --- | --- | --- | --- | --- | --- |
| `00008020…` (iPhone11,8) | 17.0 | Tauri desktop · CoreDevice AppService · RemotePairing/RSD | Monitor list, launch, confirmed stop, and cleanup | Pass | macOS accessibility automation opened Monitor → Processes, selected the debuggable side-loaded `AppsDump2 · cn.gblw.AppsDump` entry, and pressed Launch. A production snapshot confirmed new pid 4130, its `/var/containers/Bundle/Application/.../AppsDump.app/AppsDump` identity, and `canStop=true`. The native confirmation named `AppsDump` and pid 4130 exactly. After Stop, a production snapshot dropped from 468 to 467 processes and no longer contained the PID; the visible row also disappeared after the five-second interface refresh |

This closes the iOS 17.0 AppService desktop main and cleanup paths for Processes.
The launch selector now merges ordinary user applications with debuggable side-loaded
applications by bundle identifier, which makes the designated test build available
without exposing all system applications. Cross-generation visible acceptance still
needs the DVT read-only notice on iOS 26.5 and the Legacy unavailable state on iOS 14.2.

## 2026-08-09 — connected-device cold-start association

| Device | iOS | Connection | Workflow | Result | Evidence / cleanup |
| --- | --- | --- | --- | --- | --- |
| `00008020…` (iPhone11,8) | 17.0 | USB + RemotePairing catalog | Launch the Tauri application while the phone is already connected | Pass | A fresh desktop process immediately rendered `qiu的iPhone` as `USB connected · ready`. The subsequent Debug Tools JIT command and independent Processes snapshots both targeted `00008020…`, confirming that the visible selection and backend route belonged to the same device |

This closes the connected-device cold-start association check. Sleep/wake recovery
still requires a separate physical session transition.

## 2026-08-09 — iOS 17 JIT desktop acceptance

| Device | iOS | Connection | Workflow | Result | Evidence / cleanup |
| --- | --- | --- | --- | --- | --- |
| `00008020…` (iPhone11,8) | 17.0 | Tauri desktop · RemotePairing/RSD · debugserver | Debug Tools start, attach, stop, and cleanup | Pass | The interface showed DDI `Mounted` and RSD `Available`, selected `AppsDump2 · cn.gblw.AppsDump`, and reported `debugserver attached · pid 4151`. A production Processes snapshot independently identified `AppsDump` at pid 4151. Stop returned the interface to `no process attached` and `idle · ready to attach`, while a second snapshot still found pid 4151, proving detach did not terminate the application. The production identity-checked stop path then cleaned the test process and completed a fresh launch/stop cycle as pid 4488 with no residual AppsDump process |

This closes the iOS 17.0 JIT desktop attach/detach main path and confirms the
intended stop contract: the debug session ends while the application stays open.
The Legacy attach-by-name implementation has lower-level real-device evidence but
still needs the same visible desktop-interface confirmation.

## 2026-08-09 — large crash-report availability probe

| Device | iOS | Connection | Workflow | Result | Evidence / cleanup |
| --- | --- | --- | --- | --- | --- |
| `00008020…` (iPhone11,8) | 17.0 | USB Lockdown | Find a real report larger than the 4 MiB preview cap | Inconclusive | The read-only harness inspected 43 crash-report entries and found no report between 4 MiB and the 64 MiB safety cap. No device file was changed and no temporary export remained |

The large-report preview/export acceptance item stays open until a suitable report
exists; absence of a sample is not treated as a product pass or failure.

## 2026-08-01 — automatic device-loss lifecycle regression

| Device | iOS | Connection | Workflow | Result | Evidence / cleanup |
| --- | --- | --- | --- | --- | --- |
| Frontend regression | n/a | mocked desktop boundary | All devices disappear | Pass | The active device page unmounts immediately and `device_disconnect` is called, which cancels every registered device task while preserving discovery monitoring |
| Frontend regression | n/a | mocked desktop boundary | Selected device remains visible but becomes unusable | Pass | The active page unmounts, the interface returns to prerequisite guidance, and `device_disconnect` clears the backend selection and tasks |

Before this fix, discovery updated the visible connection state without ending the
backend session. The old page could remain mounted behind onboarding and continue
using a fallback demonstration-device identifier for desktop commands. Device-bound
pages now render only while a usable session exists. This is regression evidence;
the corresponding mid-operation physical-disconnect checks remain hardware gaps.

## 2026-08-04 — device-monitor concurrency regression

| Device | iOS | Connection | Workflow | Result | Evidence / cleanup |
| --- | --- | --- | --- | --- | --- |
| Frontend regression | n/a | mocked desktop boundary | Overlapping catalog refresh | Pass | A device event received during an in-flight listing is coalesced into a second listing; the stale first snapshot cannot select or render the wrong device |
| Frontend regression | n/a | mocked desktop boundary | Delayed event subscription | Pass | If the component unmounts before `deviceChanged` finishes subscribing, the returned listener is immediately released |

These checks harden the desktop session lifecycle without claiming real-device
acceptance. Physical hot-plug, sleep/wake, and mid-operation disconnects remain in
the Foundation hardware backlog.

## 2026-08-29 — 0.0.3 read-only desktop acceptance and Performance repair

| Device | iOS | Connection | Workflow | Result | Evidence / cleanup |
| --- | --- | --- | --- | --- | --- |
| `00008020…` (iPhone11,8) | 17.0 | Tauri desktop · USB + RemotePairing/RSD | Performance live CPU and memory | Pass after repair | The first desktop run showed changing system CPU but zero process rows. A bounded raw harness proved that the upstream Sysmontap client sent the 1-second sample period as the `ur` output frequency, suppressing process rows, and that system-only rows alternate with process rows. Production now sends `ur=1`, keeps `sampleInterval` independent, and emits only non-empty process snapshots. The rebuilt interface advanced from 7 to 26 one-second samples, retained 80 processes, and displayed both live CPU and memory without an error |
| `00008020…` (iPhone11,8) | 17.0 | Tauri desktop · DVT Screenshot · RemotePairing/RSD | Live Screen preview and hidden-window pause | Partial pass | The interface displayed 828 × 1792 PNG frames, reached frame 25 at a measured 1.9 FPS against the 2 FPS target, and visibly entered the hidden-window paused state. Save Frame, explicit Stop, and a traced page-exit cleanup remain open |
| `00008020…` (iPhone11,8) | 17.0 | Tauri desktop · Lockdown Misagent | Provisioning Profiles read-only inspection | Pass | One installed profile rendered with name, UUID, Ad Hoc scope, team, application identifier, one registered device, disabled debug entitlement, 13 KB signed size, and a two-day expiry warning. No profile was installed or removed |
| `00008020…` (iPhone11,8) | 17.0 | Tauri desktop · Installation Proxy metadata | Test Lab empty-runner preflight | Pass | The page found 23 optional target applications and no `.xctrunner`, showed installation guidance, and surfaced the iOS 17.0–17.3 `RemotePairing/RSD route pending` limitation. No test plan or process was started |
| `00008020…` (iPhone11,8) | 17.0 | Tauri desktop | Pasteboard privacy initial state | Pass (boundary only) | The page showed no device items or last write and did not read clipboard contents until the manual control is pressed. No clipboard data was read or written |
| `00008020…` (iPhone11,8) | 17.0 | Tauri desktop | Notification Observation explicit-selection state | Pass (boundary only) | Start remained disabled with zero selected names and zero events. No Notification Proxy subscription was opened |
| `00008020…` (iPhone11,8) | 17.0 | Tauri desktop | Network Capture safe initial state | Pass (boundary only) | The page showed zero packets, no destination, and no active transport until Start. No packet capture or local PCAP file was created |

This session used only read-only device operations. It deliberately did not start a
packet capture, inspect or replace clipboard contents, subscribe to device
notifications, run XCTest, mutate profiles, or launch and stop applications.
