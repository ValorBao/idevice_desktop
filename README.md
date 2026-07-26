# idevice desktop

A macOS developer tool for iPhone and iPad, built with React, Tauri 2, and [`jkcoxson/idevice`](https://github.com/jkcoxson/idevice). The Rust dependency is pinned to commit `8eed181f39a16ea70380ec8c3cff6bed07a1ef69` so upstream API changes cannot break the build unexpectedly.

The project aims to make device operations that currently require the `idevice-tools` command line discoverable, configurable, executable, and understandable through a graphical interface.

This is an independent project and is not an official `jkcoxson/idevice` application.

## Release

The current published release is [0.0.1 Developer Preview](https://github.com/ValorBao/idevice_desktop/releases/tag/v0.0.1). The source tree is prepared as the 0.0.2 release candidate; see the [draft release notes](docs/RELEASE_NOTES_0.0.2.md).

Developer Preview builds are unsigned and unnotarized Apple Silicon builds.

## Documentation

- [Project overview and architecture](docs/PROJECT.md)
- [Development progress and roadmap](docs/PROGRESS.md)
- [`idevice-tools` GUI coverage matrix](docs/CAPABILITY_MATRIX.md)

## Integrated Device Features

- unified usbmuxd and Bonjour discovery, hot-plug monitoring, transport merging, paired TCP Lockdown fallback, selection, pairing, unpairing, and disconnect
- Lockdown device overview, pairing information, battery information, and AFC storage capacity
- Diagnostics Relay queries for battery, MobileGestalt, IORegistry, NAND, and Wi-Fi data
- AFC file browsing, upload, download, directory creation, and recursive removal
- Installation Proxy user-application listing with icons and filtering, IPA installation, uninstallation, and progress events
- Crash report listing, filtering, text preview, and export
- Live structured OS Trace logs with pause, filter, and clear controls
- Developer Mode and Developer Disk Image mounting and unmounting
- device-targeted iOS 17+ RemotePairing/CoreDevice RSD tunnels, application launch, debug proxy attachment, and JIT sessions
- interactive Leaflet location selection with DVT/RSD and legacy Lockdown simulation transports

Running the project in a regular browser automatically uses design demonstration data. Running it through Tauri automatically switches to commands backed by a real device.

## Development

Requirements include Node.js, Rust, a working usbmuxd service, and the platform development tools required by Tauri.

```bash
npm install
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
- The published 0.0.1 build can fail to read crash reports over an iOS 17 network route and to clear a legacy simulated location. Both issues are fixed in the 0.0.2 release candidate.
- Validation covers one device per developer-service generation: iOS 14.2, 17.0, and 26.5. iOS 15 and 16 share the Legacy branch with 14.2 and are covered by it, except for Developer Mode, which arrived in iOS 16 and no verified device exercises.
- Sleeping-device behavior, the first-time trust prompt on an unauthorized host, a JIT attach on iOS 17.4 or later, and reports larger than the 4 MB preview limit still require validation.

## Developer Feature Notes

- Initial pairing requires USB and approval of the trust prompt on the device.
- iOS 16 and earlier require a matching `DeveloperDiskImage.dmg` and `.signature` file.
- Personalized mounting on iOS 17 and later requires an image, `BuildManifest.plist`, and trust cache.
- A JIT session launches the selected application and keeps the debug proxy attached. Disabling JIT, changing devices, or leaving the page ends the session.

## Credits

Core device communication is provided by [`jkcoxson/idevice`](https://github.com/jkcoxson/idevice), maintained by Jackson Coxson and its contributors. Their continued research and open-source work on modern iOS device protocols and developer services makes this project possible.

## Licensing

idevice desktop is available under the [MIT License](LICENSE). `idevice` is also available under the MIT License; its separate copyright and license notice is included in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
