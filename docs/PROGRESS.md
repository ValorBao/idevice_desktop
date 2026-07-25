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
| Real-device verification | Prior iOS 17.0 mobdev2, RemotePairing, catalog merging, and network Lockdown reads passed; the 2026-07-25 acceptance run found no connected USB or network device, so physical transitions, direct TCP fallback, and crash reports remain pending |
| Git branch | `master` |
| Worktree | Crash-report and unified-discovery work is committed in `b9883f6`; the follow-up lint and documentation changes are the current cycle |

## 3. Feature Progress

| Module | Code status | Current assessment | Next verification |
| --- | --- | --- | --- |
| Device discovery and hot plug | usbmuxd and Bonjour catalog integrated | Live iOS 17.0 mobdev2 and RemotePairing resolution, catalog merging, and network Lockdown reads pass | Verify physical USB/network transitions, direct TCP fallback, and multiple devices |
| Pair, unpair, select, and disconnect | Integrated | Build passed; real-device verification pending | Trust accepted, trust rejected, and stale pairing records |
| Overview | Integrated | iOS 17.0 Lockdown, storage, battery, and RSD screenshot paths verified | Missing fields and DDI retry failure behavior |
| Diagnostics | Five query categories integrated | Battery diagnostics verified on iOS 17.0 | Remaining query categories and permission failures across iOS versions |
| AFC and file sharing | Integrated | AFC device info and root listing verified on iOS 17.0 | Large files, mutations, read-only paths, and app containers |
| App list, installation, and uninstallation | Integrated | User-app listing verified; icons and filtering are in progress | IPA progress, icon retrieval, and uninstall confirmation |
| Crash reports | List, filter, preview, and export integrated | Build passed; browser demo interaction verified | Real-device list, flush, nested reports, preview, and export |
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
- Keep unidentified Bonjour-only devices visible with a clear “network route unavailable” state until they can be associated with a pairing record.

Commit: `b9883f6 feat: add crash reports and unified device discovery`

## 5. Active Validation

The frontend production build, Rust static check, 14 Rust unit tests, formatting check, strict Clippy check, and browser interaction check pass. Live Bonjour discovery and merged routing previously passed on iOS 17.0.

The 2026-07-25 acceptance attempt could not detect a device through USB, network `idevice_id`, or Apple's CoreDevice device list. The following checks must be repeated with an awake, trusted device:

- Disconnect USB while the device continues advertising Bonjour, and verify direct TCP Lockdown keeps Overview available.
- Reconnect USB and verify the catalog merges transports without duplicating the device.
- Repeat discovery with multiple devices and confirm each RemotePairing endpoint is routed to the matching device.
- Refresh crash reports, including nested `.ips` and `.crash` entries, then verify preview, the 4 MB preview limit, and complete export.
- Record the device model, iOS build, connection type, result, and relevant error for each check.

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
- Validate the new crash-report workflow, then prioritize device console workflows, performance monitoring, packet capture, and process control.

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
| YYYY-MM-DD | Device model | Version | USB/Network | Feature name | Pass/Fail/Partial | Error or environment details |

## 9. Update Checklist

At the end of each development cycle:

1. Update build, test, and worktree status in Current Snapshot.
2. Move completed work from Active Work into the milestone history.
3. Record real-device verification with specific environment details.
4. Reassess P0, P1, and P2 priorities and document new risks or decisions.
