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

- Complete Location set/clear through the desktop interface on Legacy and iOS 17+.
- Complete JIT attach/detach through the desktop interface.
- Preview and export an actual report larger than 4 MB.
- Exercise cold start and sleep/wake association through the full discovery catalog.
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
and cleanup still require an explicit mutating harness run, and the
CoreDeviceRemote and Legacy generations remain to be probed.

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
