# idevice desktop Development Progress

> Last updated: 2026-07-22
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
| Frontend production build | Passed on 2026-07-22 with `npm run build` |
| Rust static check | Passed on 2026-07-22 with `cargo check --manifest-path src-tauri/Cargo.toml` |
| Rust unit tests | Passed on 2026-07-22: 7 passed, 0 failed |
| Test coverage | Covers IPA signature checks, file-path protection, and iOS generation selection; no frontend, integration, or automated real-device tests yet |
| Real-device verification | Initial iOS 17.0 USB smoke test passed; broader compatibility coverage is still required |
| Git branch | `main` |
| Worktree | Contains uncommitted changes; see Active Work |

## 3. Feature Progress

| Module | Code status | Current assessment | Next verification |
| --- | --- | --- | --- |
| Device discovery and hot plug | Integrated | Build passed; real-device verification pending | USB reconnects, multiple devices, and network devices |
| Pair, unpair, select, and disconnect | Integrated | Build passed; real-device verification pending | Trust accepted, trust rejected, and stale pairing records |
| Overview | Integrated | iOS 17.0 Lockdown, storage, battery, and RSD screenshot paths verified | Missing fields and DDI retry failure behavior |
| Diagnostics | Five query categories integrated | Battery diagnostics verified on iOS 17.0 | Remaining query categories and permission failures across iOS versions |
| AFC and file sharing | Integrated | AFC device info and root listing verified on iOS 17.0 | Large files, mutations, read-only paths, and app containers |
| App list, installation, and uninstallation | Integrated | User-app listing verified; icons and filtering are in progress | IPA progress, icon retrieval, and uninstall confirmation |
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

## 5. Active Work

Before this documentation was created, the worktree had nine modified feature files relative to `main`, with approximately 130 additions and 37 deletions. These changes remain uncommitted:

- Show only user applications and retrieve real icons through SpringBoardServices.
- Add `iconDataUrl` and `icon_data_url` to the TypeScript and Rust `InstalledApp` contract.
- Cache screenshots by UDID and retry after automatic DDI mounting when necessary.
- Replace the static location grid with an interactive Leaflet and OpenStreetMap view.
- Add Leaflet and its TypeScript definitions.

The frontend production build and Rust static check pass. These changes still require real-device and desktop-interaction testing before they should be considered complete.

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
- Prioritize crash reports, device console workflows, performance monitoring, packet capture, and process control.

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
