# idevice desktop

A macOS developer tool for iPhone and iPad, built with React, Tauri 2, and [`jkcoxson/idevice`](https://github.com/jkcoxson/idevice). The Rust dependency is pinned to commit `8eed181f39a16ea70380ec8c3cff6bed07a1ef69` so upstream API changes cannot break the build unexpectedly.

The project aims to make device operations that currently require the `idevice-tools` command line discoverable, configurable, executable, and understandable through a graphical interface.

This is an independent project and is not an official `jkcoxson/idevice` application.

## Documentation

- [Project overview and architecture](docs/PROJECT.md)
- [Development progress and roadmap](docs/PROGRESS.md)
- [`idevice-tools` GUI coverage matrix](docs/CAPABILITY_MATRIX.md)

## Integrated Device Features

- usbmuxd device discovery, hot-plug monitoring, selection, pairing, unpairing, and disconnect
- Lockdown device overview, pairing information, battery information, and AFC storage capacity
- Diagnostics Relay queries for battery, MobileGestalt, IORegistry, NAND, and Wi-Fi data
- AFC file browsing, upload, download, directory creation, and recursive removal
- Installation Proxy application listing, IPA installation, uninstallation, and progress events
- Live structured OS Trace logs with pause, filter, and clear controls
- Developer Mode and Developer Disk Image mounting and unmounting
- iOS 17+ CoreDevice/RSD software tunnels, application launch, debug proxy attachment, and JIT sessions
- DVT/RSD and legacy Lockdown location simulation transports

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

## Developer Feature Notes

- Initial pairing requires USB and approval of the trust prompt on the device.
- iOS 16 and earlier require a matching `DeveloperDiskImage.dmg` and `.signature` file.
- Personalized mounting on iOS 17 and later requires an image, `BuildManifest.plist`, and trust cache.
- A JIT session launches the selected application and keeps the debug proxy attached. Disabling JIT, changing devices, or leaving the page ends the session.

## Credits

Core device communication is provided by [`jkcoxson/idevice`](https://github.com/jkcoxson/idevice), maintained by Jackson Coxson and its contributors. Their continued research and open-source work on modern iOS device protocols and developer services makes this project possible.

## Licensing

idevice desktop is available under the [MIT License](LICENSE). `idevice` is also available under the MIT License; its separate copyright and license notice is included in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
