# idevice desktop Development Progress

> Last updated: 2026-07-25
> Version: 0.0.1
> Stage: most MVP capabilities are integrated; the project is entering real-device validation, stability work, and code organization.

The product direction is confirmed: developer tools first, macOS-only for the initial release, and long-term GUI coverage of device capabilities that currently require `idevice-tools`.

## 1. Status Definitions

- **Integrated**: the frontend and backend code path exists.
- **Build passed**: the current worktree passes the corresponding static check or production build.
- **Real-device verification pending**: no traceable device, system version, and result has been recorded here.
- **In progress**: the current worktree contains related uncommitted changes.

## 2. Current Snapshot

| Item | Status |
| --- | --- |
| Frontend production build | Passed on 2026-07-25 with `npm run build` |
| Rust static check | Passed on 2026-07-25 with `cargo check --manifest-path src-tauri/Cargo.toml` |
| Rust unit tests | Passed on 2026-07-25: 14 passed, 0 failed |
| Rust formatting and linting | Passed on 2026-07-25 with `cargo fmt --check` and strict Clippy warnings |
| Unsigned macOS package | Apple Silicon `idevice_0.0.1_aarch64.dmg` built and passed `hdiutil verify` on 2026-07-25 |
| Test coverage | Covers IPA signature checks, file-path protection, crash-report handling, iOS generation selection, and discovery transport merging; no frontend, integration, or automated real-device tests yet |
| Real-device verification | iPhone11,8 on iOS 17.0 passed USB/Bonjour merging, physical USB disconnect and reconnect, direct TCP Lockdown fallback, and USB crash-report list/preview/export; crash-report listing over direct Bonjour TCP failed twice with `UnexpectedEof` |
| Git branch | `master` |
| Worktree | Version `0.0.1` is published; the 2026-07-25 real-device acceptance record is the current follow-up cycle |

## 3. Feature Progress

| Module | Code status | Current assessment | Next verification |
| --- | --- | --- | --- |
| Device discovery and hot plug | usbmuxd and Bonjour catalog integrated | iOS 17.0 USB/Bonjour merging, physical disconnect/reconnect, and direct TCP Lockdown fallback pass | Verify multiple devices, sleeping-device behavior, and cold-start association |
| Pair, unpair, select, and disconnect | Integrated | Build passed; real-device verification pending | Trust accepted, trust rejected, and stale pairing records |
| Overview | Integrated | iOS 17.0 Lockdown, storage, battery, and RSD screenshot paths verified | Missing fields and DDI retry failure behavior |
| Diagnostics | Five query categories integrated | Battery diagnostics verified on iOS 17.0 | Remaining query categories and permission failures across iOS versions |
| AFC and file sharing | Integrated | AFC device info and root listing verified on iOS 17.0 | Large files, mutations, read-only paths, and app containers |
| App list, installation, and uninstallation | Integrated | User-app listing verified; icons and filtering are in progress | IPA progress, icon retrieval, and uninstall confirmation |
| Crash reports | List, filter, preview, and export integrated | USB list, preview, and byte-for-byte complete export pass on iOS 17.0; direct Bonjour TCP listing fails with `UnexpectedEof` | Diagnose network service closure; verify nested reports and previews larger than 4 MB |
| Live logs | Integrated | OS Trace connection and event receipt verified on iOS 17.0 | Long sessions, pause, disconnects, and high throughput |
| Developer Mode | Integrated | Status query verified on iOS 17.0 | Enable flow, reboot or confirmation, and failure recovery |
| DDI mount and unmount | Legacy and personalized paths integrated | Mounted-image status and iOS 17.0 RSD screenshot path verified | Mount/unmount mutations and devices from iOS 16 and 17.4+ |
| JIT | Integrated | Build passed; real-device verification pending | Launch, attach, stop, and device-switch cleanup |
| Location simulation | Legacy and DVT/RSD paths integrated | Interactive map improvements are in progress | Map selection, presets, and restoration of real GPS |
| Browser demo mode | Integrated | Frontend build passed | Visual and state consistency with desktop mode |
| Three themes and light/dark appearance | Integrated | Frontend build passed | Small windows, long content, and accessibility |

## 4. Completed Milestones

### 2026-07-18: Project Initialization

- Created the React, Tauri 2, and Rust project structure.
- Imported and implemented the primary design handoff.
- Connected real-device commands and browser demo mode.

Commit: `8ffd17a chore: initialize idevice desktop repository`

### 2026-07-18: iOS Developer-Service Generations

- Defined separate paths for iOS 16 and earlier, iOS 17.0–17.3, and iOS 17.4+.
- Applied version-aware handling to CoreDevice/RSD, DDI, JIT, and location.

Commit: `b2e37b8 fix: handle iOS developer services by version`

### 2026-07-19: File, Installation, and Log Hardening

- Hardened live-log task startup and shutdown.
- Improved IPA installation handling.
- Hardened AFC and House Arrest file access.

Commit: `a584c07 feat: harden logs installs and file access`

### 2026-07-25: Crash Reports and Unified Discovery

Implemented the first crash-report workflow:

- Recursively list reports exposed by `com.apple.crashreportcopymobile`, after requesting a best-effort report flush.
- Filter reports by filename, process, path, `.ips`, or `.crash`.
- Preview text safely with a 4 MB display limit.
- Export the complete original report to a user-selected local path.
- Validate remote paths and cap a listing at 2,000 reports.
- Represent report size as unknown until previewed instead of displaying a false `0 B`, avoiding up to 2,000 AFC metadata round trips during listing.
- Provide browser demonstration data and a responsive list-and-preview interface.

Implemented the unified device-discovery layer:

- Monitor usbmuxd and the `_apple-mobdev2._tcp`, `_remotepairing._tcp`, and manual RemotePairing Bonjour services in parallel.
- Merge USB and network observations by UDID, Wi-Fi MAC address, hostname, or address overlap.
- Coalesce all transitively matching observations when mobdev2 bridges an earlier RemotePairing record to USB, and hide unidentifiable RemotePairing-only records from the device list.
- Share one Bonjour daemon and multicast socket across all three browsed service types.
- Prefer USB while preserving Bonjour presence after USB disconnects.
- Route a known paired device through direct Bonjour TCP Lockdown when its usbmuxd record disappears.
- Route iOS 17.0–17.3 screenshot, location, and JIT tunnels to the RemotePairing endpoint associated with the selected device instead of the first service found.
- Keep mobdev2 and manual RemotePairing Bonjour-only devices visible with a clear “network route unavailable” state until they can be associated with a pairing record.

Commit: `b9883f6 feat: add crash reports and unified device discovery`

## 5. Active Validation

The frontend production build, Rust static check, 14 Rust unit tests, formatting check, strict Clippy check, and browser interaction check pass.

The 2026-07-25 iPhone11,8 and iOS 17.0 acceptance session established the following:

- With USB attached, the catalog represented the phone as one connectable device with `USB`, `Wi-Fi`, and `RemotePairing` transports.
- After physical USB removal, usbmuxd no longer listed the phone while mobdev2 continued advertising through Bonjour.
- The project selected its Bonjour TCP provider, started a paired Lockdown session, and read `ProductVersion=17.0`.
- Reattaching USB restored the three transports on the same catalog record without creating a duplicate device.
- The USB crash-report service listed reports, previewed a 179,345-byte `.ips`, and exported an identical complete file.
- A separate best-effort baseline copied 570 reports, approximately 80 MB, without removing them from the device.
- Direct Bonjour TCP connected to the crash-report service but root listing failed twice because the device closed the TLS stream with `UnexpectedEof`.

Remaining acceptance gaps are multiple devices, a sleeping device, nested crash-report directories, reports larger than the 4 MB preview limit, and iOS 16 or iOS 17.4+ devices.

## 6. Recommended Next Phase

### P0: Establish a Trustworthy Real-Device Baseline

- Record the date, device, iOS version, connection type, feature, result, and relevant log or error for every validation session.
- Cover iOS 16, iOS 17.0–17.3, and iOS 17.4+ paths. Mark unavailable devices as explicit coverage gaps.
- Prioritize the main and cleanup paths for pairing, Overview, files, installation, uninstallation, logs, DDI, JIT, and location.
- Validate the active application-icon, screenshot-cache, and map-interaction changes.

### P0: Maintain CLI-to-GUI Coverage

- Use [`CAPABILITY_MATRIX.md`](CAPABILITY_MATRIX.md) as the capability baseline for the pinned upstream revision.
- Track Not integrated, Partial, Integrated, Build passed, and Real-device verified separately for each capability.
- When upgrading `idevice`, compare command and feature changes and update the matrix before scheduling work.
- Diagnose the direct-TCP crash-report `UnexpectedEof`, then prioritize device console workflows, performance monitoring, packet capture, and process control.

### P1: Testing and Stability

- Add unit tests for version selection, path normalization, cross-boundary data conversion, and error mapping.
- Add serialization contract tests to prevent drift between Rust and TypeScript fields.
- Verify that device switches, disconnects, and page unmounts reliably stop long-running tasks.
- Cover empty states, timeouts, permission denial, mid-operation disconnects, and large files.

### P1: Frontend Maintainability

- Split `App.tsx` into page components, shared components, and hooks.
- Consolidate device sessions, notifications, and asynchronous loading into a clear state model.
- Add component tests for critical interactions and audit keyboard access, focus, and color contrast.

### P2: Release Preparation

- Define minimum macOS and iOS versions.
- Tighten the Tauri CSP and evaluate the online map dependency and offline fallback.
- Complete the application icon, macOS signing, notarization, release notes, and full third-party license inventory.
- Verify that the project MIT License and complete third-party notices ship with every release artifact.
- Define an upgrade and regression process for the pinned `idevice` revision.

## 7. Open Decisions

- **Compatibility:** minimum iOS version and the device/system combinations required for the first release
- **macOS coverage:** minimum macOS version and whether Intel Macs are included initially
- **Map strategy:** online OpenStreetMap, an offline option, or a replaceable tile source
- **Distribution:** internal tool, open-source project, or general-user application

## 8. Real-Device Verification Log

Append future validation results using this format:

| Date | Device | iOS | Connection | Feature | Result | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| 2026-07-22 | iPhone11,8 | 17.0 (21A329) | USB | Pairing and Lockdown | Pass | Existing pairing validated; product information query succeeded |
| 2026-07-22 | iPhone11,8 | 17.0 (21A329) | USB | Diagnostics, AFC, and app listing | Pass | Battery dictionary, AFC root, storage, and 16 user apps returned |
| 2026-07-22 | iPhone11,8 | 17.0 (21A329) | USB | Developer status and OS Trace | Pass | Developer Mode enabled, one mounted image record, and a live log event received |
| 2026-07-22 | iPhone11,8 | 17.0 (21A329) | USB | RemotePairing/RSD screenshot | Pass | Project screenshot implementation returned a non-empty PNG data URL |
| 2026-07-25 | iPhone11,8 | 17.0 (21A329) | USB → Network → USB | Unified discovery and Lockdown fallback | Pass | One record transitioned from USB + Wi-Fi + RemotePairing to Wi-Fi + RemotePairing; direct TCP Lockdown read ProductVersion 17.0; USB reconnect restored all transports without duplication |
| 2026-07-25 | iPhone11,8 | 17.0 (21A329) | USB | Crash-report list, preview, and export | Pass | Listed reports, previewed a 179,345-byte IPS, and verified complete export byte-for-byte; a keep-on-device baseline copied 570 reports |
| 2026-07-25 | iPhone11,8 | 17.0 (21A329) | Network | Crash-report listing over direct TCP | Fail | Service connection succeeded, but root listing failed twice with TLS `UnexpectedEof`; USB path remained healthy |
| YYYY-MM-DD | Device model | Version | USB/Network | Feature name | Pass/Fail/Partial | Error or environment details |

## 9. Update Checklist

At the end of each development cycle:

1. Update build, test, and worktree status in Current Snapshot.
2. Move completed work from Active Work into the milestone history.
3. Record real-device verification with specific environment details.
4. Reassess P0, P1, and P2 priorities and document new risks or decisions.
