# idevice_desktop

A macOS developer tool for iPhone and iPad, built with React, Tauri 2, and [`jkcoxson/idevice`](https://github.com/jkcoxson/idevice). The Rust dependency is pinned to commit `8eed181f39a16ea70380ec8c3cff6bed07a1ef69` so upstream API changes cannot break the build unexpectedly.

The project aims to make device operations that currently require the `idevice-tools` command line discoverable, configurable, executable, and understandable through a graphical interface.

This is an independent project and is not an official `jkcoxson/idevice` application.

## Release

The current release is [0.0.2 Developer Preview](https://github.com/ValorBao/idevice_desktop/releases/tag/v0.0.2); see the [release notes](docs/RELEASE_NOTES_0.0.2.md).

The `0.0.3` release candidate is tracked in [its draft release notes](docs/RELEASE_NOTES_0.0.3.md). Development of the next `0.0.4` iteration is tracked in [its draft release notes](docs/RELEASE_NOTES_0.0.4.md).

Developer Preview builds are unsigned and unnotarized Apple Silicon builds.

## Documentation

- [Project overview and architecture](docs/PROJECT.md)
- [Development progress and roadmap](docs/PROGRESS.md)
- [Feature delivery plan](docs/FEATURE_PLAN.md)
- [`idevice-tools` GUI coverage matrix](docs/CAPABILITY_MATRIX.md)

## Integrated Device Features

- unified usbmuxd and Bonjour discovery, hot-plug monitoring, transport merging, paired TCP Lockdown fallback, selection, pairing, unpairing, and disconnect
- Lockdown device overview, pairing information, battery information, and AFC storage capacity
- Diagnostics Relay queries for battery, MobileGestalt, IORegistry, NAND, and Wi-Fi data
- AFC file browsing, upload, download, directory creation, and recursive removal
- Installation Proxy user-application listing with icons and filtering, IPA installation, uninstallation, and progress events
- Personal Signing Assistant with an iLoader-compatible Apple Account + 2FA flow for automatic device registration, certificate/profile creation, signed IPA export, and optional installation; the existing local IPA/profile/Keychain workflow remains available
- Crash report listing, filtering, text preview, and export
- Live structured OS Trace logs with pause, filter, and clear controls
- searchable iOS 17+ process monitoring with application launch, AppService-only confirmed stop, stale-PID protection, and explicit DVT-stop/Legacy support boundaries
- iOS 17+ Performance monitoring with live system/process CPU, memory footprint, stable process history, pause/resume, filtering, bounded retention, and CSV export
- streaming device packet capture to Wireshark-compatible PCAP with optional PID/interface filters, live statistics, disk safeguards, atomic save, and cancel-and-delete cleanup
- read-only notification observation with explicit presets or custom subscription names, a searchable bounded timeline, pause/clear controls, and stale-session isolation
- Live Screen PNG preview with start/stop, measured frame rate, fit/100% scaling, current-frame export, and automatic session cleanup
- read-only provisioning profile inspection with search, signing scope, device coverage, debug entitlement, and expiration warnings
- explicit iOS 17+ Pasteboard transfer for bounded UTF-8 text and PNG/JPEG images, with manual reads, local image preview, per-write confirmation, and no background clipboard monitoring
- read-only XCTest Test Lab preflight and run-plan validation with bounded runner/target discovery, metadata and entitlement checks, test filters, timeout and WDA bridge intent, plus explicit generation-specific route limitations
- Developer Mode and Developer Disk Image mounting and unmounting
- device-targeted iOS 17+ RemotePairing/CoreDevice RSD tunnels, application launch, debug proxy attachment, and JIT sessions
- interactive Leaflet location selection with DVT/RSD and legacy Lockdown simulation transports

Running the project in a regular browser automatically uses design demonstration data. Running it through Tauri automatically switches to commands backed by a real device.

## Development

Requirements include Node.js, Rust, a working usbmuxd service, and the platform development tools required by Tauri.

```bash
npm install
npm test
npm run desktop:dev
```

To preview only the frontend design:

```bash
npm run dev
```

## Build

```bash
npm run build
npm run desktop:build
```

To build only the macOS DMG:

```bash
npm run desktop:build -- --bundles dmg
```

## Developer Preview Limitations

- Builds support Apple Silicon only and are not signed with an Apple Developer ID or notarized.
- USB-to-network Lockdown fallback is verified on an iPhone XR running iOS 17.0.
- USB discovery, nested crash-report export, legacy screenshots, OS Trace, diagnostics, AFC file round trips, application listing with icons, and legacy location set/clear are verified on an iPhone10,1 running iOS 14.2.
- The published 0.0.1 build can fail to read crash reports over an iOS 17 network route and to clear a legacy simulated location. Both issues are fixed in 0.0.2.
- Validation covers one device per developer-service generation: iOS 14.2, 17.0, and 26.5. iOS 15 and 16 share the Legacy branch with 14.2 and are covered by it, except for Developer Mode, which arrived in iOS 16 and no verified device exercises.
- Performance uses DVT Sysmontap on iOS 17 and later. Its interface and lifecycle pass automated checks, but its live metrics have not yet been accepted on hardware; iOS 16 and earlier show an explicit unavailable state.
- Network Capture uses USB pcapd on every supported generation and RSD pcapd on modern network routes. The file lifecycle passes automated checks, but the capture-to-Wireshark round trip has not yet been accepted on hardware.
- Live Screen refreshes the established Screenshotr or DVT screenshot service at a target of 2 PNG frames per second. Its lifecycle, frame validation, and export pass automated checks, but the preview has not yet been accepted on hardware and is not a smooth-video or remote-control path.
- Provisioning Profiles reads Misagent over USB Lockdown or modern RSD and parses normalized metadata locally. The first slice is deliberately read-only; profile installation/removal and hardware acceptance remain pending.
- Personal Signing Assistant 0.0.4 defaults to an Apple Account flow built on `isideload`: the password is held only for the current login attempt, cleared from the interface immediately, never persisted by idevice_desktop, and the active session lasts only until sign-out or app exit. Certificate material and Anisette state use macOS Keychain. This unofficial compatibility route contacts Apple private developer endpoints and `https://ani.sidestore.io`, so Apple or the community service can change or block it. The separate local-profile mode retains its stricter single-bundle limits and never asks for an Apple Account password. Neither route has completed real-account export/install acceptance.
- The resolved `isideload` signing stack currently includes `nab138/apple-crates`' `apple-codesign` package, which declares no license. The 0.0.4 development code must not be packaged or distributed until that dependency is licensed or replaced; see `THIRD_PARTY_NOTICES.md`.
- Notification Observation reads names relayed by Notification Proxy over USB Lockdown or modern RSD. It does not expose payloads or notification posting, and hardware acceptance remains pending.
- Pasteboard transfer uses the iOS 17+ CoreDevice/RSD service. Text is limited to 1 MB; PNG/JPEG images are limited to 12 MB and a safe preview dimension budget. Reads resolve only a supported item with a known bounded size, image writes are validated and previewed locally, every write requires confirmation, and hardware acceptance remains pending.
- Test Lab reads installed `.xctrunner` and optional target metadata, combines that with Developer Mode, DDI, and RSD readiness, and validates a non-executing plan containing mode, target, bounded include/skip filters, timeout, and WDA bridge intent. It never starts a test process. XCTest event output, runtime timeout enforcement, deterministic Stop/cleanup, WDA networking, the iOS 17.0–17.3 RemotePairing adapter, and hardware acceptance remain pending.
- Sleeping-device behavior, the first-time trust prompt on an unauthorized host, a JIT attach on iOS 17.4 or later, and reports larger than the 4 MB preview limit still require validation.

## Developer Feature Notes

- Initial pairing requires USB and approval of the trust prompt on the device.
- iOS 16 and earlier require a matching `DeveloperDiskImage.dmg` and `.signature` file.
- Personalized mounting on iOS 17 and later requires an image, `BuildManifest.plist`, and trust cache.
- A JIT session launches the selected application and keeps the debug proxy attached. Disabling JIT, changing devices, or leaving the page ends the session.

## Credits

Core device communication is provided by [`jkcoxson/idevice`](https://github.com/jkcoxson/idevice), maintained by Jackson Coxson and its contributors. Apple Account authentication and automatic personal signing use [`nab138/isideload`](https://github.com/nab138/isideload), following the user flow demonstrated by [`nab138/iloader`](https://github.com/nab138/iloader). Their open-source work makes these device and signing workflows possible.

## Licensing

idevice_desktop is available under the [MIT License](LICENSE). Separate copyright and license notices for `idevice`, `isideload`, iLoader-derived implementation references, and the resolved dependency inventory are included in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
