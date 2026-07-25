# idevice desktop Development Progress

> Last updated: 2026-07-25
> Release: 0.0.1; next patch in development
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
| Rust unit tests | Passed on 2026-07-25: 39 passed, 0 failed |
| Rust formatting and linting | Passed on 2026-07-25 with `cargo fmt --check` and strict Clippy warnings |
| Unsigned macOS package | Apple Silicon `idevice_0.0.1_aarch64.dmg` built and passed `hdiutil verify` on 2026-07-25 |
| Test coverage | Covers IPA signature checks, file-path protection, crash-report handling and transport selection, iOS generation selection, discovery transport merging, JIT attach-reply parsing, debuggable-application filtering, and the serialization contract with `src/api.ts`; no frontend, integration, or automated real-device tests yet |
| Real-device verification | iPhone11,8 on iOS 17.0 passed USB/Bonjour merging, crash reports over USB and RemotePairing/RSD, and the JIT tunnel through application launch; iPhone10,1 on iOS 14.2 passed USB discovery/routing, crash reports, screenshot, logs, diagnostics, AFC, app listing, legacy location, and a full unpair/re-pair |
| Verification harnesses | `src-tauri/examples/verify_jit.rs` and `verify_pairing.rs` drive the real provider, tunnel, and command code against an attached device |
| Branches | All validation branches are merged; `master` is at the 2026-07-25 JIT and pairing cycle |
| Worktree | Clean. The merged patch carries the iOS 17 network crash-report route, the legacy location cleanup, the JIT attach-failure fix, the JIT selector fix, and the frontend split |

## 3. Feature Progress

| Module | Code status | Current assessment | Next verification |
| --- | --- | --- | --- |
| Device discovery and hot plug | usbmuxd and Bonjour catalog integrated | iOS 17.0 USB/Bonjour merging and physical transitions pass; iOS 14.2 USB discovery, pairing state, catalog entry, and routing pass | Verify multiple devices, sleeping-device behavior, and cold-start association |
| Pair, unpair, select, and disconnect | Integrated | iOS 14.2 unpair and re-pair pass end to end; the USB-only guard correctly refuses a network record on iOS 17.0 | First-time trust prompt on an untrusted host, trust rejected, and stale pairing records |
| Overview | Integrated | iOS 17.0 Lockdown, storage, battery, and RSD screenshot paths pass; iOS 14.2 legacy screenshot returned a valid 379,466-byte PNG | Missing fields and DDI retry failure behavior |
| Diagnostics | Five query categories integrated | Battery, MobileGestalt, IORegistry, NAND, and Wi-Fi request paths pass on iOS 14.2; battery also passes on iOS 17.0 | Permission failures and payload differences across more iOS versions |
| AFC and file sharing | Integrated | Root listing passes on iOS 14.2 and 17.0; a 43-byte iOS 14.2 test file passed upload/download equality and cleanup | Large files, read-only paths, and app containers |
| App list, installation, and uninstallation | Integrated | iOS 14.2 returned 10 user apps and a non-empty app icon; listing is also verified on iOS 17.0 | IPA progress and uninstall confirmation |
| Crash reports | List, filter, preview, and export integrated | USB Lockdown list/read/export pass on iOS 14.2 and 17.0; network RemotePairing/RSD passes on iOS 17.0 | Verify the iOS 17.4+ CoreDeviceProxy path and previews larger than 4 MB |
| Live logs | Integrated | OS Trace connection and event receipt verified on iOS 14.2 and 17.0 | Long sessions, pause, disconnects, and high throughput |
| Developer Mode | Integrated | Status query verified on iOS 17.0 | Enable flow, reboot or confirmation, and failure recovery |
| DDI mount and unmount | Legacy and personalized paths integrated | A full mount and unmount cycle passes on iOS 17.0, and every mounted-image signal the project reads agrees with the RSD service list | Devices from iOS 16 and 17.4+, and mounting through Choose files rather than devicectl |
| JIT | Integrated for both generations | iOS 17.0 passes launch, `vAttach`, detach, and cleanup; iOS 14.2 passes attach-by-name to a running app and detach, leaving the app running. A rejected attach is reported as a failure, and the selector offers debuggable applications | Verify `jit_stop` from the interface and device-switch cleanup |
| Location simulation | Legacy and DVT/RSD paths integrated | iOS 14.2 Lockdown set/clear and restoration of real GPS pass after reconnecting for clear | Verify frontend map selection and the iOS 17+ DVT/RSD path |
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

The frontend production build, Rust static check, 15 Rust unit tests, formatting check, strict Clippy check, and browser interaction check pass.

The 2026-07-25 iPhone11,8 and iOS 17.0 acceptance session established the following:

- With USB attached, the catalog represented the phone as one connectable device with `USB`, `Wi-Fi`, and `RemotePairing` transports.
- After physical USB removal, usbmuxd no longer listed the phone while mobdev2 continued advertising through Bonjour.
- The project selected its Bonjour TCP provider, started a paired Lockdown session, and read `ProductVersion=17.0`.
- Reattaching USB restored the three transports on the same catalog record without creating a duplicate device.
- The USB crash-report service listed reports, previewed a 179,345-byte `.ips`, and exported an identical complete file.
- A separate best-effort baseline copied 570 reports, approximately 80 MB, without removing them from the device.
- The original direct Lockdown crash-report service closed during AFC root listing with `UnexpectedEof`; routing the same operation through the RSD `com.apple.crashreportcopymobile.shim.remote` service fixed the failure.
- RemotePairing/RSD listed, previewed, and exported the same 179,345-byte `.ips` byte-for-byte over the network.

The 2026-07-25 iPhone10,1 and iOS 14.2 session also verified a paired, connectable USB catalog entry and correct USB provider routing. The crash-report traversal found 315 reports, including nested entries, then read and exported a 530,010-byte `.ips` byte-for-byte without removing it from the phone. This device did not advertise a network route during the session.

The same iOS 14.2 device exposed the legacy location service after mounting its matching DeveloperDiskImage. Setting `31.2304, 121.4737` succeeded, but the device closed that service connection after the set command, so the original same-connection clear failed with `Broken pipe`. Reconnecting before clear fixed the implementation; the repeated set/clear round trip passed and restored real GPS. The test DeveloperDiskImage was unmounted afterward.

The iOS 14.2 feature sweep captured a valid 379,466-byte PNG, received an OS Trace event, completed all five diagnostic request paths, and listed 10 user applications with a 9,478-byte sample icon. AFC listed 14 root entries and round-tripped a unique 43-byte test file through `/PublicStaging`; byte equality and post-delete absence were both verified. No application was installed or uninstalled, and the matching DeveloperDiskImage was unmounted after the sweep.

The 2026-07-25 JIT and pairing session used two `src-tauri/examples` harnesses that call the project's real provider, tunnel, and command code. Pairing passed on both devices: the iOS 14.2 device completed a full unpair and re-pair, and the iOS 17.0 network record was correctly refused by the USB-only guard.

JIT was verified in two stages. With no DDI mounted, the RemotePairing tunnel opened in 656 ms and exposed 57 RSD services, and the DVT handshake, application launch, and memory-limit removal all succeeded, but `DebugProxyClient::connect_rsd` failed with "service not found". `ImageMounter::copy_devices()` returned no images and `lookup_image` found neither a Developer nor a Personalized image, confirming the device carried no DDI. `devicectl --auto-mount-ddis` repeatedly failed with `kAMDMobileImageMounterExistingTransferInProgress` until the device was rebooted; it then mounted DDI 17E202, the RSD service count rose from 57 to 69, and `com.apple.internal.dt.remote.debugproxy` appeared. That established the earlier failure as a missing DDI rather than a routing defect.

The attach round trip then exposed a real defect. debugserver answered `vAttach` for an App Store build with `E96;…`, which decodes to "attach failed (Not allowed to attach to process…)". Because that is a protocol-level error packet rather than a transport error, `send_command` returned success and the session was reported as attached: the interface would have shown a JIT badge over a process that was never attached, and the launched process was left running. `attach_failure` now decodes the reply, surfaces the device's own explanation, and routes through the same cleanup path used for a failed debug-server connection. Four unit tests cover stop packets, a hex-encoded reason, a bare error code, and a payload that only resembles an error.

A successful attach was then verified against a TrollStore-installed application. `vAttach` returned a `T11` stop packet carrying full register state, detach succeeded, and the launched process was terminated. That also confirms the new attach-failure check does not misread a successful stop packet as an error, so both directions of the JIT attach path are now covered.

Reaching that application exposed a separate defect. `apps_list` requests only the `User` application type and the interface then filters out anything marked as a system application. Every debuggable application on the device is registered as `System`: a survey of all 209 registered applications found eight carrying `get-task-allow`, including the TrollStore-installed tooling, and none of them can appear in the JIT selector. The JIT backend therefore works while the feature stays unreachable through the interface for its primary audience, since applications installed through TrollStore or a sideloader are exactly the ones that need manual JIT. Applications distributed through the App Store never carry `get-task-allow` and are granted their JIT privileges by the system directly.

A DDI mount and unmount cycle was then verified on the same device, which also settled a doubt raised during the JIT work. Two separate readings of `copy_devices` had returned no images, suggesting it might not reflect a personalized DDI on iOS 17 and that `developer_status` might depend on its devicectl fallback to be correct. Taking the missing reading while an image was actually mounted disproved that: `copy_devices` reported one image, `lookup_image("Personalized")` returned a 48-byte signature, and the RSD service list rose from 57 to 69 with the debug proxy appearing. All three agreed again after unmounting. Both earlier readings had simply been taken while no image was mounted, and `ddi_mounted` needs no correction. `lookup_image("Developer")` reports nothing throughout, which is correct for a generation that uses personalized images.

Remaining acceptance gaps are multiple simultaneously visible devices, a sleeping device, reports larger than the 4 MB preview limit, devices on iOS 15/16 or iOS 17.4+, and the first-time trust prompt on a host the device has never authorized. The iOS 17.4+ CoreDeviceProxy crash-report route is integrated but not hardware-verified. JIT on iOS 16 and earlier is refused by design and remains unimplemented, which excludes the large TrollStore audience on those versions.

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
- Publish the RSD crash-report fix in the next patch, then prioritize device console workflows, performance monitoring, packet capture, and process control.

### P0: JIT Reach and Coverage

- ~~List debuggable applications in the JIT selector instead of user applications.~~ Done on 2026-07-25: `apps_debuggable` filters on `get-task-allow` across every application type, and the Apps page still lists user applications.
- ~~Implement JIT for iOS 16 and earlier through the legacy debugserver transport.~~ Done on 2026-07-25 and verified on iPhone10,1 / iOS 14.2. The instruments server is unusable on that system, so the legacy path attaches by process name to an app the user opened rather than launching it.
- Verify `jit_stop` from the interface and confirm that switching or disconnecting a device tears down an active JIT session.

### P1: Testing and Stability

- Add unit tests for version selection, path normalization, cross-boundary data conversion, and error mapping.
- ~~Add serialization contract tests to prevent drift between Rust and TypeScript fields.~~ Done on 2026-07-25: thirteen tests compare what serde emits against the declarations parsed from `src/api.ts`. Both directions were confirmed to fail on an induced mismatch.
- Verify that device switches, disconnects, and page unmounts reliably stop long-running tasks.
- Cover empty states, timeouts, permission denial, mid-operation disconnects, and large files.

### P1: Frontend Maintainability

- ~~Split `App.tsx` into page components, shared components, and helpers.~~ Done on 2026-07-25: the shell dropped from 1,147 to 233 lines, with eight page modules, five shared components, and two helper modules. The move was verified line by line and the production bundle was unchanged apart from module boundaries.
- ~~Extract the repeated desktop-guard and toast-on-error pattern into a shared hook.~~ Done on 2026-07-25: `useDeviceTask` took the duplicated catch-and-toast line from 15 occurrences to 3. The remaining three are load paths whose catch clauses do more than report.
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
| 2026-07-25 | iPhone11,8 | 17.0 (21A329) | Network | Crash-report list, preview, and export over RemotePairing/RSD | Pass | The direct Lockdown service failed twice with `UnexpectedEof`; switching to the RSD shim listed and exported a 179,345-byte IPS byte-for-byte |
| 2026-07-25 | iPhone10,1 | 14.2 (18B92) | USB | Discovery, route selection, and crash-report round trip | Pass | Identified one paired and connectable USB record; traversed 315 reports and exported a nested 530,010-byte IPS byte-for-byte without deleting the source |
| 2026-07-25 | iPhone10,1 | 14.2 (18B92) | USB | Legacy location simulation set and clear | Pass | Matching DeveloperDiskImage was required; reconnect-before-clear fixed `Broken pipe`; test coordinates were cleared and the image was unmounted |
| 2026-07-25 | iPhone10,1 | 14.2 (18B92) | USB | Screenshot, OS Trace, diagnostics, AFC, and app listing | Pass | Valid PNG and log event returned; five diagnostic requests completed; 43-byte AFC file round-tripped and was removed; 10 user apps and a sample icon returned |
| 2026-07-25 | iPhone10,1 | 14.2 (18B92) | USB | Unpair and re-pair through `device_pair` | Pass | Trust record removed, then re-paired in a single call; the host ID and usbmuxd device ID both changed, and the new record opened a Lockdown session reading 14.2. The device did not re-prompt because it still authorized this host while unlocked, so the first-time trust dialog stayed uncovered |
| 2026-07-25 | iPhone11,8 | 17.0 (21A329) | Network | `device_pair` transport guard | Pass | Refused a non-USB record with "Initial pairing requires a USB connection" before touching the trust relationship |
| 2026-07-25 | iPhone11,8 | 17.0 (21A329) | Network | JIT tunnel, launch, and failure cleanup | Partial | RemotePairing tunnel opened in 656 ms exposing 57 RSD services; DVT handshake, `launch_app`, and `disable_memory_limit` passed; `DebugProxyClient::connect_rsd` returned "service not found" because no DDI was mounted, and the cleanup path then terminated the launched process as designed |
| 2026-07-25 | iPhone11,8 | 17.0 (21A329) | USB | DDI mount and debug-proxy availability | Pass | `devicectl --auto-mount-ddis` failed with `ExistingTransferInProgress` until the device was rebooted, then mounted DDI 17E202; RSD services went from 57 to 69 and exposed `com.apple.internal.dt.remote.debugproxy`, confirming the earlier "service not found" was a missing DDI rather than a routing defect |
| 2026-07-25 | iPhone11,8 | 17.0 (21A329) | USB | JIT attach round trip | Pass (attach rejected as expected) | The debug server connected and `vAttach` returned `E96;…` decoding to "attach failed (Not allowed to attach to process…)" for an App Store build. This exposed a defect: the reply was previously reported as an attached session. Attach rejection is now surfaced as an error and the launched process is terminated |
| 2026-07-25 | iPhone11,8 | 17.0 (21A329) | USB | Full JIT session against a debuggable application | Pass | Attaching to a TrollStore-installed application returned a `T11` stop packet with full register state; detach and process termination both succeeded. This also confirms the attach-failure check does not reject a successful stop packet |
| 2026-07-25 | iPhone10,1 | 14.2 (18B92) | USB | Legacy JIT attach and detach | Pass | With the matching DeveloperDiskImage mounted, `debugserver.DVTSecureSocketProxy` opened in 191 ms and `vAttachName` for `ShopeeSG` returned a `T11` stop packet with full register state. Detach succeeded and the app was deliberately left running |
| 2026-07-25 | iPhone10,1 | 14.2 (18B92) | USB | Legacy instruments server | Fail | `StartService` returns a port for `com.apple.instruments.remoteserver.DVTSecureSocketProxy`, but the socket never responds: a TLS handshake and a plaintext DVT handshake each stalled past 45 seconds. Both plain-name variants answer `InvalidService`. `assertion_agent` on the same transport works, so lockdown itself is healthy. Device logs show `handle_start_service` and `spawn_xpc_service` with the service name redacted, and nothing afterwards |
| 2026-07-25 | iPhone11,8 | 17.0 (21A329) | USB | DDI mount and unmount cycle | Pass | Unmounted, mounted, and unmounted again. `copy_devices` reported 0, 1, then 0 images; `lookup_image("Personalized")` returned nothing, a 48-byte signature, then nothing; RSD services moved 57 → 69 → 57 with the debug proxy appearing and disappearing. `ddi_unmount` returned success and every signal returned to baseline |
| YYYY-MM-DD | Device model | Version | USB/Network | Feature name | Pass/Fail/Partial | Error or environment details |

## 9. Update Checklist

At the end of each development cycle:

1. Update build, test, and worktree status in Current Snapshot.
2. Move completed work from Active Work into the milestone history.
3. Record real-device verification with specific environment details.
4. Reassess P0, P1, and P2 priorities and document new risks or decisions.
