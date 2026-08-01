# idevice desktop Development Progress

> Last updated: 2026-07-28
> Release: 0.0.2 Developer Preview
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
| Frontend production build | Passed on 2026-07-26 with `npm run build` |
| Frontend regression tests | Passed on 2026-07-26: 18 passed, 0 failed across five Vitest files |
| GitHub Actions CI | Passed on PR #27: Frontend on Ubuntu in 26 seconds; Rust formatting, check, 63 tests, and strict Clippy on macOS 14 arm64 in 3 minutes 3 seconds |
| Rust static check | Passed on 2026-07-26 with `cargo check --manifest-path src-tauri/Cargo.toml` |
| Rust unit tests | Passed on 2026-07-26: 63 passed, 0 failed |
| Rust formatting and linting | Passed on 2026-07-26 with `cargo fmt --check` and strict Clippy warnings |
| Unsigned macOS package | Apple Silicon `idevice_0.0.2_aarch64.dmg` built on 2026-07-26; passes `hdiutil verify`, identifies itself as 0.0.2 with a macOS 11.0 minimum, carries the CSP in its arm64 release binary, and ships both licence files byte-identical to their sources |
| Test coverage | Frontend tests cover the desktop/demo task guard, Tauri-versus-browser destructive confirmation, in-app text prompts, native file-drop hit testing and cleanup, Files create/delete/drop/progress/cancel flows, Apps uninstall/IPA-drop flows, and a browser-demo interaction path. Rust tests cover IPA signature checks, file-path protection, crash-report handling and transport selection, iOS generation selection, discovery transport merging, device-selection routing, connection labelling, location coordinate validation, JIT attach-reply parsing, debuggable-application filtering, and the serialization contract with `src/api.ts`; there are no integration or automated real-device tests |
| Known desktop-only defect class | Browser APIs that work in demo mode and fail silently under Tauri. `window.confirm` resolves to false, `window.prompt` to null, and `window.alert` never appears, because wry implements no WKWebView JavaScript panel delegate; HTML5 `ondrop` never fires for OS drags, because Tauri consumes them first. Four controls shipped dead — Files delete, Files new folder, Apps uninstall, Apps sideload drop. All fixed on 2026-07-26; the rule and the approved replacements are in `CLAUDE.md` |
| Real-device verification | iPhone14,5 on iOS 26.5 passed the CoreDeviceProxy crash-report route, CoreDevice pairing, DDI mounting, and the JIT transport; iPhone11,8 on iOS 17.0 passed USB/Bonjour merging, crash reports over USB and RemotePairing/RSD, and the JIT tunnel through application launch; iPhone10,1 on iOS 14.2 passed USB discovery/routing, crash reports, screenshot, logs, diagnostics, AFC, app listing, legacy location, and a full unpair/re-pair |
| Verification harnesses | `src-tauri/examples/verify_jit.rs`, `verify_pairing.rs`, and `verify_processes.rs` drive the real provider, tunnel, and command code against an attached device |
| Branches | Current work continues on `codex/current-surface-acceptance`; `master` does not yet contain that acceptance pass |
| Worktree | In progress: Processes protocol harness and its first CoreDeviceLockdown result |

## 3. Feature Progress

| Module | Code status | Current assessment | Next verification |
| --- | --- | --- | --- |
| Device discovery and hot plug | usbmuxd and Bonjour catalog integrated | iOS 17.0 USB/Bonjour merging and physical transitions pass; iOS 14.2 USB discovery and routing pass; two devices attached at once stay separate with the correct transport each | Verify sleeping-device behavior and cold-start association |
| Pair, unpair, select, and disconnect | Integrated | iOS 14.2 unpair and re-pair pass end to end; the USB-only guard correctly refuses a network record on iOS 17.0 | First-time trust prompt on an untrusted host, trust rejected, and stale pairing records |
| Overview | Integrated | iOS 17.0 Lockdown, storage, battery, and RSD screenshot paths pass; iOS 14.2 legacy screenshot returned a valid 379,466-byte PNG | Missing fields and DDI retry failure behavior |
| Diagnostics | Five query categories integrated | Battery, MobileGestalt, IORegistry, NAND, and Wi-Fi request paths pass on iOS 14.2; battery also passes on iOS 17.0 | Permission failures and payload differences across more iOS versions |
| AFC and file sharing | Integrated | Root listing passes on iOS 14.2 and 17.0; a 43-byte iOS 14.2 test file passed upload/download equality and cleanup. Transfers stream in 1 MB chunks with progress and cancellation as of 2026-07-26, verified on a real device | Read-only paths, app containers, and cancelling mid-transfer |
| App list, installation, and uninstallation | Integrated | iOS 14.2 returned 10 user apps and a non-empty app icon; listing is also verified on iOS 17.0 | IPA progress and uninstall confirmation |
| Crash reports | List, filter, preview, and export integrated | USB Lockdown list/read/export pass on iOS 14.2 and 17.0; RemotePairing/RSD passes on iOS 17.0; the CoreDeviceProxy route passes on iOS 26.5, listing the same entries as direct Lockdown | Previews larger than 4 MB |
| Live logs | Integrated | OS Trace connection and event receipt verified on iOS 14.2 and 17.0 | Long sessions, pause, disconnects, and high throughput |
| Developer Mode | Integrated | Status query verified on iOS 17.0 | Enable flow, reboot or confirmation, and failure recovery |
| DDI mount and unmount | Legacy and personalized paths integrated | A full mount and unmount cycle passes on iOS 17.0, and every mounted-image signal the project reads agrees with the RSD service list | Devices from iOS 16 and 17.4+, and mounting through Choose files rather than devicectl |
| JIT | Integrated for both generations | iOS 17.0 passes launch, `vAttach`, detach, and cleanup; iOS 14.2 passes attach-by-name and detach. Ending a session leaves the application running, and the task registry cancels a session on device switch. A rejected attach is reported as a failure | Exercise the full sequence through the interface rather than a harness |
| Processes | Protocol proof in progress; no visible UI | The read-only harness listed 220 processes on iOS 26.5 through DVT DeviceInfo. That device advertised `com.apple.instruments.dtservicehub` but not `com.apple.coredevice.appservice`, so the backend must support DVT instead of assuming AppService exists on every iOS 17+ device | Verify launch, PID visibility, termination, and cleanup on iOS 26.5; repeat list and control on iOS 17.0; record Legacy as unsupported if its instruments service still stalls |
| Location simulation | Legacy and DVT/RSD paths integrated | iOS 14.2 Lockdown set/clear and restoration of real GPS pass after reconnecting for clear | Verify frontend map selection and the iOS 17+ DVT/RSD path |
| Browser demo mode | Integrated | Frontend build passed | Visual and state consistency with desktop mode |
| Single Device Lab theme | Integrated | The style and appearance switchers were removed on 2026-07-26; the interface is fixed to the dark Device Lab theme. Frontend build passed | Small windows, long content, and accessibility |

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

### 2026-07-26: Device Lab Visual Direction

Settled the interface on a single visual direction, which release notes were waiting on:

- Removed the in-app style and appearance switchers and fixed the application to the dark Device Lab theme. They were scaffolding from exploring three parallel directions and had stopped earning their place in the header.
- Reworked the title bar into a brand mark with a live connection status line, and gave the page header a device state readout in place of the theme controls.
- Rebuilt Overview as an object-led cockpit rather than a generic card dashboard.
- Added entry motion: pages fade and lift, Diagnostics stat values count up over smoothed sparklines, and capacity bars grow with a hover readout. All of it degrades under `prefers-reduced-motion`.
- Receded the dividers inside cards and faded the page-header and tab rules out toward the right, so dense pages stop reading as grid paper.
- Restyled the Location caption as light frosted glass and removed the redundant service-name badge from the map.

The map tiles stayed on standard OpenStreetMap. A detour through a dark basemap was reverted: a monochrome dark tileset loses the map's colour coding, and reproducing it with a CSS invert and hue-rotate filter produces colours that are simply wrong. Recorded here so the idea is not proposed again.

A follow-up pass removed what the switchers left behind: the `UiStyle` and `Appearance` types, the `.style-switcher` and `.appearance-button` rules including their two responsive overrides, and the five unreachable theme variable blocks. Only `.theme-clean.mode-dark` remains, because `device-lab.css` layers over it. The production stylesheet dropped from 90.18 kB to 86.58 kB.

Merge commit: `a06074c Merge pull request #20 from ValorBao/agent/device-lab-visual-polish`

### 2026-07-26: The Files Page Made Usable

An audit prompted by the observation that hardly anything on the Files page fully
worked. It was accurate: two of its four actions did nothing, drag-and-drop did not
exist, and a large transfer looked like a freeze.

Most of it traced to one cause — **browser APIs that work in demo mode and fail
silently under Tauri.** Four controls had shipped dead:

- `window.confirm` resolves to false, so Files delete and Apps uninstall returned at
  their guard. Replaced with the dialog plugin, which also needed `dialog:allow-message`.
- `window.prompt` resolves to null, so Files new folder returned at its guard and
  `afc_mkdir` had never once run from the desktop. Replaced with `PromptModal`; the
  plugin has no text input, so an in-app modal was the only option.
- HTML5 `ondrop` never fires for OS drags, and the `File.path` the handler read is an
  Electron extension WKWebView does not implement, so the Apps sideload zone could
  not have worked either way. Replaced with `useFileDrop` over Tauri's own event.

None of these threw. Each type-checked, read correctly, and passed in demo mode.

Transfers were rewritten to stream. Both directions had buffered whole files in
memory and emitted nothing, so a video from DCIM held the entire file in RAM behind
a frozen interface. They now loop in 1 MB chunks — the library's own wire limit —
with throttled progress, cancellation through the task registry, and cleanup of the
partial file on failure.

The remaining gaps were closed: listings keep entries whose metadata cannot be read
instead of silently shortening, `file_sharing_apps` queries every application type,
`rename` and empty-file creation are exposed, and AFC mutations finally log their
outcome — they had recorded nothing at all, which is how the drop defect went so
long without a trace.

PRs: #23, #24, and the Files completeness follow-up.

## 5. Active Validation

The frontend production build, 22 frontend regression tests, Rust static check, 63 Rust unit tests, formatting check, strict Clippy check, and browser interaction check pass. Current hardware-acceptance evidence is recorded in [`ACCEPTANCE_LOG.md`](ACCEPTANCE_LOG.md).

The Processes protocol proof started on 2026-07-28 with
`src-tauri/examples/verify_processes.rs`. Its default mode is read-only; an explicit
bundle ID is required to exercise launch and stop, and every mutating step has a
cleanup path and a 30-second timeout. On iOS 26.5, CoreDeviceProxy exposed 62 RSD
services but did not advertise `com.apple.coredevice.appservice`. The harness fell
back to `com.apple.instruments.dtservicehub`, completed the DVT handshake, and
returned 220 running processes through DeviceInfo. This is backend protocol evidence,
not interface acceptance. Launch and stop remain unverified.

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

Remaining acceptance gaps are a sleeping device, reports larger than the 4 MB preview limit, the first-time trust prompt on a host the device has never authorized, and a JIT attach on iOS 17.4 or later, which needs a debuggable application on such a device.

iOS 15 and 16 are not tracked as a separate gap. `developer_generation()` in `device_version.rs` branches on `major < 17`, so 14, 15, and 16 all select the same Legacy transport and DDI approach, and the verified iOS 14.2 result covers that branch. One behaviour sits outside this equivalence: Developer Mode arrived in iOS 16, so an iOS 16 device must enable it before developer services respond. On 14 and 15 `AmfiClient::connect` fails and the status degrades to `None`, which is correct for those versions but means the iOS 16 enable-and-reboot flow is exercised by no verified device.

## 6. Recommended Next Phase

### P0: Establish a Trustworthy Real-Device Baseline

- Record the date, device, iOS version, connection type, feature, result, and relevant log or error for every validation session.
- Cover one device per developer-service generation rather than one per iOS release. iOS 14.2 covers Legacy, 17.0 covers CoreDeviceRemote, and 26.5 covers CoreDeviceLockdown; all three are verified. iOS 15 and 16 fall inside the Legacy branch and are covered by the 14.2 result, with the iOS 16 Developer Mode flow noted above as the one exception.
- Prioritize the main and cleanup paths for pairing, Overview, files, installation, uninstallation, logs, DDI, JIT, and location.
- Validate the active application-icon and screenshot-cache changes.
- Validate the Location page against a device. Preset selection, map panning, and click-to-select were exercised in browser demo mode on 2026-07-26, which does not touch the DVT/RSD or legacy location transports.

### P0: Maintain CLI-to-GUI Coverage

- Use [`CAPABILITY_MATRIX.md`](CAPABILITY_MATRIX.md) as the capability baseline for the pinned upstream revision.
- Use [`FEATURE_PLAN.md`](FEATURE_PLAN.md) as the delivery order and Definition of Done; do not expose a capability before its complete workflow passes.
- Track Not integrated, Partial, Integrated, Build passed, and Real-device verified separately for each capability.
- When upgrading `idevice`, compare command and feature changes and update the matrix before scheduling work.
- ~~Publish the RSD crash-report fix in the next patch.~~ Shipped in 0.0.2.
- Finish the current-surface acceptance pass, then prove the Processes backend on real hardware before adding it to the Monitor interface.

### P0: JIT Reach and Coverage

- ~~List debuggable applications in the JIT selector instead of user applications.~~ Done on 2026-07-25: `apps_debuggable` filters on `get-task-allow` across every application type, and the Apps page still lists user applications.
- ~~Implement JIT for iOS 16 and earlier through the legacy debugserver transport.~~ Done on 2026-07-25 and verified on iPhone10,1 / iOS 14.2. The instruments server is unusable on that system, so the legacy path attaches by process name to an app the user opened rather than launching it.
- ~~Verify `jit_stop` and confirm that switching or disconnecting a device tears down an active JIT session.~~ Done on 2026-07-25: four tests cover the task registry, and a real-device check confirms ending a session detaches without closing the application.

### P1: Testing and Stability

- ~~Cover the command modules that had no tests at all.~~ Started on 2026-07-26: `device.rs` and `location.rs` were the two largest untested modules, and both now cover their pure decision logic — selection routing, connection labelling, and coordinate validation. `overview.rs`, `screenshot.rs`, `diagnostics.rs`, and `logs.rs` remain untested, though they are small and mostly pass values through.
- ~~Add a frontend test harness.~~ Done on 2026-07-26 with Vitest, Testing Library, jsdom, and 18 focused tests. The baseline covers `useDeviceTask` branching, Tauri destructive confirmation, `PromptModal`, native drag/drop coordinate handling and cleanup, Files create/delete/folder-drop/progress/cancel flows, Apps uninstall/IPA-drop flows, and a browser-demo interaction path. Coverage percentage is deliberately not the target.
- ~~Run the frontend and Rust gates automatically.~~ Done on 2026-07-26: GitHub Actions runs the frontend suite and production build on Ubuntu, then Rust formatting, check, tests, and strict Clippy on macOS 14 arm64. Rust 1.89.0 is pinned for reproducible Clippy results, and both jobs passed on PR #27.
- Integration tests are deliberately not planned. The integration boundary in this project is a real device, and a mocked stand-in would prove very little for the effort.

- ~~Add unit tests for version selection, path normalization, cross-boundary data conversion, and error mapping.~~ Done on 2026-07-25. The first three were already covered; error mapping was not, and `CommandError` itself had been missed by the contract tests despite crossing the boundary on every failed command.
- ~~Add serialization contract tests to prevent drift between Rust and TypeScript fields.~~ Done on 2026-07-25: thirteen tests compare what serde emits against the declarations parsed from `src/api.ts`. Both directions were confirmed to fail on an induced mismatch.
- Verify that device switches, disconnects, and page unmounts reliably stop long-running tasks.
- Cover empty states, timeouts, permission denial, mid-operation disconnects, and large files.

### P1: Frontend Maintainability

- ~~Split `App.tsx` into page components, shared components, and helpers.~~ Done on 2026-07-25: the shell dropped from 1,147 to 233 lines, with eight page modules, five shared components, and two helper modules. The move was verified line by line and the production bundle was unchanged apart from module boundaries.
- ~~Extract the repeated desktop-guard and toast-on-error pattern into a shared hook.~~ Done on 2026-07-25: `useDeviceTask` took the duplicated catch-and-toast line from 15 occurrences to 3. The remaining three are load paths whose catch clauses do more than report.
- Consolidate device sessions, notifications, and asynchronous loading into a clear state model.
- Expand component tests as new regressions appear, and audit keyboard access, focus, and color contrast.

### P2: Release Preparation

- ~~Define minimum macOS and iOS versions.~~ Done on 2026-07-25: macOS 11.0 on Apple Silicon, matching what the binary requires; iOS verified at 14.2 and above, with older paths present but unverified. See `PROJECT.md`.
- ~~Tighten the Tauri CSP.~~ Done on 2026-07-25: scripts are limited to bundled code, images to the app, `data:` URLs, and the map tile host, and object, frame, and form directives are closed. Verified in the running application.
- ~~Evaluate the online map dependency and an offline fallback.~~ Decided on 2026-07-25 to keep the online map with no fallback. The map is a convenience for picking coordinates, not a requirement for simulating a location, and presets remain available when tiles do not load.
- ~~Complete the application icon and the full third-party license inventory.~~ Done on 2026-07-25: the icon is rendered from its SVG source at 1024x1024, and the notices file now lists every dependency that ships.
- ~~Write release notes.~~ Done on 2026-07-26 for the 0.0.2 Developer Preview in [`RELEASE_NOTES_0.0.2.md`](RELEASE_NOTES_0.0.2.md).
- **macOS signing and notarization are blocked**: no Apple Developer account or certificate is available. Releases stay unsigned, so distribution has to keep the Gatekeeper warning and the instructions for opening an unsigned build.
- ~~Verify that the project MIT License and complete third-party notices ship with every release artifact.~~ Done on 2026-07-25: both files appear in the `.app` and the `.dmg`, byte-identical to their sources, and the CSP is embedded in the release binary. The DMG passes `hdiutil verify`.
- ~~Define an upgrade and regression process for the pinned `idevice` revision.~~ Defined on 2026-07-25 in `PROJECT.md`.

## 7. Open Decisions

The four decisions previously listed here were all settled during the 2026-07-25 and 2026-07-26 cycles and are recorded where they belong:

- **Compatibility** — resolved. One device per developer-service generation, verified at iOS 14.2, 17.0, and 26.5; see the P0 entry above and the Supported Versions table in `PROJECT.md`.
- **macOS coverage** — resolved. macOS 11.0 or later, Apple Silicon only for the initial release; see `PROJECT.md`.
- **Map strategy** — resolved. Online OpenStreetMap with no offline fallback; see the P2 entry above.
- **Distribution** — resolved. Open-source MIT project distributed as an unsigned GitHub prerelease; see the decision log in `PROJECT.md`.

Genuinely open:

- **Signing** stays blocked rather than undecided: there is no Apple Developer account, so every release ships unsigned.
- **iOS 16 Developer Mode** has no verified device. The enable-and-reboot flow is the one Legacy-generation behaviour the iOS 14.2 result does not cover.

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
| 2026-07-25 | iPhone14,5 | 26.5 | USB | Crash reports over CoreDeviceProxy | Pass | The version maps to the CoreDeviceLockdown generation. The tunnel exposed 62 RSD services including the crash-report shim, and listing through it returned the same 12 root entries as direct Lockdown. This route had been integrated since the crash-report work but never run on hardware |
| 2026-07-25 | iPhone14,5 | 26.5 | USB | CoreDevice pairing and DDI mounting | Pass | `devicectl` first refused with "must be paired", which is the condition `mount_ddi_with_devicectl` detects; pairing then succeeded and the DDI mounted. RSD services rose from 62 to 74 with two debug proxies, and `lookup_image("Personalized")` returned a 48-byte signature |
| 2026-07-25 | iPhone14,5 | 26.5 | USB | JIT transport | Partial | The CoreDeviceProxy tunnel opened in 89 ms exposing 74 services. Attach is uncovered: none of the 48 installed applications carry `get-task-allow`. The upstream XPC decoder logged a body-length mismatch during the handshake, which did not prevent it from completing |
| 2026-07-25 | iPhone10,1 + iPhone11,8 | 14.2 + 17.0 | USB + Network | Two devices attached at once | Pass | usbmuxd reported two records and the catalog produced exactly two entries with distinct UDIDs, labelling the USB device `USB` and the network device `usbmuxd Network` |
| 2026-07-25 | iPhone10,1 | 14.2 (18B92) | USB | Ending a JIT session | Pass | Attached by name, sent the detach command `jit_stop` triggers, then re-attached successfully, confirming the application survives the session ending |
| 2026-07-25 | iPhone10,1 | 14.2 (18B92) | USB | Legacy JIT attach and detach | Pass | With the matching DeveloperDiskImage mounted, `debugserver.DVTSecureSocketProxy` opened in 191 ms and `vAttachName` for `ShopeeSG` returned a `T11` stop packet with full register state. Detach succeeded and the app was deliberately left running |
| 2026-07-25 | iPhone10,1 | 14.2 (18B92) | USB | Legacy instruments server | Fail | `StartService` returns a port for `com.apple.instruments.remoteserver.DVTSecureSocketProxy`, but the socket never responds: a TLS handshake and a plaintext DVT handshake each stalled past 45 seconds. Both plain-name variants answer `InvalidService`. `assertion_agent` on the same transport works, so lockdown itself is healthy. Device logs show `handle_start_service` and `spawn_xpc_service` with the service name redacted, and nothing afterwards |
| 2026-07-25 | iPhone11,8 | 17.0 (21A329) | USB | DDI mount and unmount cycle | Pass | Unmounted, mounted, and unmounted again. `copy_devices` reported 0, 1, then 0 images; `lookup_image("Personalized")` returned nothing, a 48-byte signature, then nothing; RSD services moved 57 → 69 → 57 with the debug proxy appearing and disappearing. `ddi_unmount` returned success and every signal returned to baseline |
| 2026-07-26 | iPhone10,1 + iPhone11,8 + iPhone14,5 | 14.2 + 17.0 + 26.5 | USB | Destructive-action confirmations through the interface | Pass | Files delete and Apps uninstall both open the plugin dialog and complete. All three had been dead on the desktop: wry implements no WKWebView JavaScript panel delegate, so `window.confirm` resolved to false and the guard returned early. Frontend-only, so the three systems confirm the fix rather than add generation coverage |
| 2026-07-26 | iPhone10,1 + iPhone11,8 + iPhone14,5 | 14.2 + 17.0 + 26.5 | USB | Folder creation through the interface | Pass | First execution of `afc_mkdir` from the desktop on any device — `window.prompt` returned null before this, so the command was unreachable and the backend path had never run |
| 2026-07-26 | iPhone10,1 + iPhone11,8 + iPhone14,5 | 14.2 + 17.0 + 26.5 | USB | Streaming AFC transfer with progress and cancellation | Pass | Transfers now loop in 1 MB chunks instead of buffering the whole file; progress advances and the interface stays responsive. Cancelling stops the transfer and removes the partial file |
| 2026-07-26 | iPhone10,1 + iPhone11,8 + iPhone14,5 | 14.2 + 17.0 + 26.5 | USB | Files dragged in from Finder | Pass | Dropping into the table uploads to the open folder and dropping onto a folder row uploads into it. The Apps sideload zone had the same defect class: Tauri consumes OS drags before the webview sees them, and the `File.path` it read is an Electron extension WKWebView does not implement |
| 2026-07-28 | iPhone14,5 | 26.5 | usbmuxd network record | Processes read-only protocol proof | Pass | CoreDeviceProxy opened in 420 ms with 62 RSD services. AppService was absent, so the harness used `com.apple.instruments.dtservicehub`; the DVT handshake and DeviceInfo channel returned 220 running processes. No application was launched or stopped |
| YYYY-MM-DD | Device model | Version | USB/Network | Feature name | Pass/Fail/Partial | Error or environment details |

## 9. Update Checklist

At the end of each development cycle:

1. Update build, test, and worktree status in Current Snapshot.
2. Move completed work from Active Work into the milestone history.
3. Record real-device verification with specific environment details.
4. Reassess P0, P1, and P2 priorities and document new risks or decisions.
