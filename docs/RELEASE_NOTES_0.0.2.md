# idevice desktop 0.0.2 Developer Preview

This patch turns the first preview into a substantially more reliable device lab. It fixes several workflows that appeared to work in browser demo mode but silently failed in the Tauri desktop application, and it publishes the real-device hardening completed since 0.0.1.

## Highlights

- Files now supports responsive, streaming uploads and downloads with progress, cancellation, and partial-file cleanup.
- Files can receive files dragged from Finder, including drops directly onto a destination folder.
- File deletion, folder creation, app uninstallation, and IPA drag-and-drop now work in the desktop application.
- Crash reports on iOS 17 network routes use the RemotePairing/RSD shim instead of the Lockdown route that could end with `UnexpectedEof`.
- Legacy location simulation reconnects before clearing when the device closes the original service connection.
- The interface now uses the single dark Device Lab visual direction, with a clearer device state, refreshed Overview, and reduced-motion support.

## Device and Developer Workflows

- Unified USB, Bonjour, and RemotePairing discovery keeps transports for the same device together and preserves a usable network route after USB disconnects.
- JIT now lists applications that are actually debuggable, reports rejected attaches as failures, cleans up failed sessions, and supports the legacy iOS 16-and-earlier debugserver path.
- Developer Disk Image state and mount/unmount behavior have been hardened for both legacy and personalized-image generations.
- App listings include icons, while file-sharing discovery checks every application type.
- AFC adds rename and empty-file creation, keeps entries whose optional metadata cannot be read, and records mutation outcomes in the application log.

## Validation

The release build passed the project’s frontend production build, Rust checks, unit tests, formatting, and strict linting. Real-device sessions cover:

- iPhone10,1 on iOS 14.2 for the Legacy generation.
- iPhone11,8 on iOS 17.0 for the CoreDevice RemotePairing generation.
- iPhone14,5 on iOS 26.5 for the CoreDevice Lockdown generation.

The recorded checks include discovery and routing, pairing, screenshots, crash reports, diagnostics, AFC transfers, app listing, logs, location simulation, DDI lifecycle, JIT transport, and destructive-action confirmations. See [PROGRESS.md](PROGRESS.md) for the device-by-device evidence and remaining gaps.

## Compatibility and Known Limitations

- macOS 11.0 or later on Apple Silicon.
- The build is unsigned and unnotarized because the project does not have an Apple Developer certificate.
- iOS 16 Developer Mode enable-and-reboot behavior has not been verified on hardware.
- Sleeping-device behavior, a first-time trust prompt on a previously unauthorized host, reports larger than the 4 MB preview limit, and a successful JIT attach on iOS 17.4 or later remain unverified.
- Location map tiles require internet access; coordinate presets remain available without them.
- Developer features require the appropriate Developer Disk Image.

## Installing the Unsigned Preview

1. Download and open `idevice_0.0.2_aarch64.dmg`.
2. Drag `idevice` to Applications.
3. In Finder, Control-click the application, choose **Open**, then confirm **Open**. macOS remembers this choice for later launches.

Do not bypass Gatekeeper globally. Review the source and release provenance before running an unsigned build.
