# idevice_desktop 0.0.3 Developer Preview

> Draft — in development. This version has not been published.

0.0.3 expands the device lab with monitoring and read-only developer workflows,
then moves the project from code integration into real-device acceptance.

## Highlights

- Performance shows live system and per-process CPU and memory, bounded history,
  stable process identity, interval controls, pause/resume, and CSV export.
- Network Capture streams device packets to a Wireshark-compatible PCAP with
  optional PID/interface filters, disk safeguards, atomic save, and cancel/delete.
- Live Screen provides a bounded 2 FPS PNG preview with fit/100% scale and current
  frame export.
- Provisioning Profiles provides read-only signing, entitlement, device-scope, and
  expiry inspection.
- Notification Observation provides explicit read-only subscriptions and a bounded
  searchable event-name timeline.
- Pasteboard provides deliberate bounded UTF-8 and PNG/JPEG transfer on iOS 17+,
  without background clipboard monitoring.
- Test Lab inspects installed XCTest runners and target applications and validates a
  non-executing Standard XCTest or WDA plan.

## Performance Reliability Fix

The first iOS 17.0 desktop run exposed a protocol defect that automated tests could
not reproduce. Sysmontap's output-frequency field was incorrectly coupled to the
visible sample interval, so system CPU changed while process rows stayed empty.
System-only protocol rows were also presented as zero-process snapshots.

0.0.3 keeps output frequency independent from sampling interval, carries the latest
system CPU into the next process snapshot, ignores system-only rows for the process
table, and allows a longer bounded gap before reporting a stalled stream. A new
read-only harness records the device-provided schema and six bounded rows.

## Validation So Far

On an iPhone11,8 running iOS 17.0 through USB and RemotePairing/RSD:

- Performance advanced from 7 to 26 one-second samples with 80 visible processes
  and live CPU/memory values.
- Live Screen reached frame 25 at a measured 1.9 FPS for 828 × 1792 PNG frames and
  entered the hidden-window paused state.
- Provisioning Profiles parsed one installed profile and displayed its normalized
  signing and expiry metadata.
- Test Lab loaded 23 optional target apps, found no installed runner, and displayed
  the correct RemotePairing execution limitation without starting a process.
- Network Capture, Notification Observation, and Pasteboard remained idle until an
  explicit user action. No packet capture, subscription, clipboard read, or write
  occurred during this acceptance session.
- The unsigned 8.3 MB Apple Silicon DMG passes `hdiutil verify`; its app reports
  version 0.0.3, requires macOS 11.0, and contains byte-identical project and
  third-party license files.

## Remaining Release Gates

- Verify Performance interval changes, pause/resume, CSV export, and page-exit
  cleanup, plus the iOS 17.4+ CoreDeviceLockdown route.
- Open a saved PCAP in Wireshark and verify stop, cancel, disconnect, and partial-file
  cleanup.
- Verify Live Screen Save Frame, explicit Stop, page-exit cleanup, and the Legacy
  Screenshotr route.
- Exercise deliberate test notification and pasteboard content with explicit user
  authorization.
- Install a dedicated signed `.xctrunner` for Test Lab metadata acceptance; XCTest
  execution remains outside this release until its full lifecycle is implemented.
- Complete the remaining real-device workflow checks above, then review and publish
  the already verified unsigned Apple Silicon DMG.

The published release remains 0.0.2 until these gates are closed or explicitly
accepted as documented limitations.
