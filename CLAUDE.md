# CLAUDE.md

macOS desktop GUI for iOS device tooling. React + Tauri 2 + Rust, wrapping
[`jkcoxson/idevice`](https://github.com/jkcoxson/idevice).

See `docs/PROJECT.md` for architecture and `docs/PROGRESS.md` for status. This file
covers only what is easy to get wrong.

## Commands

```bash
npm run dev              # browser demo mode — mock data, no device
npm run desktop:dev      # Tauri desktop — real devices (use this for anything device-related)
npm test                 # Vitest frontend regression suite
npm run build            # tsc -b && vite build
cargo test  --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt   --manifest-path src-tauri/Cargo.toml --check
```

## Two runtimes, one UI

Every page branches on a `desktop` boolean. When false, it renders mock data from
`src/data.ts`; when true, it calls Rust. A change to any page has to handle both paths.

**Demo-mode output is never real-device verification.** Confirming something in the
browser says nothing about whether the device transport works — say which mode you
verified in.

`useDeviceTask` (`src/lib/hooks.ts`) wraps the guard-and-report pattern: it skips the
call in demo mode and routes thrown errors to the page's toast. Use it for device
operations rather than repeating the try/catch.

## Browser APIs that are silently dead on the desktop

This class of bug has already shipped four broken controls. Each one type-checks,
reads correctly, works in demo mode, and fails on the desktop **without throwing** —
so the action just quietly does not happen and review cannot see it.

**`window.confirm` / `window.prompt` / `window.alert`.** wry implements no WKWebView
JavaScript panel delegate, so WebKit resolves them with its cancel result: `confirm`
returns false, `prompt` returns null, `alert` never appears.

- Confirming something → `dialogs.confirmDestructive` in `src/api.ts`.
- Asking for text → the `PromptModal` component. The dialog plugin has no text input,
  so an in-app modal is the only option.

**HTML5 file drops.** Tauri consumes OS drags before the webview sees them, so
`ondrop` never fires for them and `dataTransfer.files` stays empty. It is also the
only source of a real path: a browser `File` carries no filesystem location, and the
`File.path` property is an Electron extension WKWebView does not implement.

- Dropped files → the `useFileDrop` hook, which wraps Tauri's `onDragDropEvent`.
  That event is window-wide, so the hook hit-tests the pointer to scope a drop.

Any new dialog plugin call also needs its permission in
`src-tauri/capabilities/default.json`; `confirm` and `ask` both go through
`dialog:allow-message`.

When a control depends on a browser API, check how it behaves under Tauri before
assuming demo mode proved anything.

## `src/api.ts` is an enforced contract

`src-tauri/src/types.rs` pulls `src/api.ts` in with `include_str!` and compares the
field names serde emits against the TypeScript declarations. Changing a Rust response
struct without updating `api.ts` fails `cargo test` with a field-set mismatch — the
error is the contract test, not a broken build.

## iOS developer services are generation-specific

Anything touching a developer service selects its transport through
`developer_generation()` in `src-tauri/src/device_version.rs`. Never assume one
protocol across versions.

| Generation | Versions | Transport |
| --- | --- | --- |
| `Legacy` | iOS 16 and earlier | Lockdown developer services, `DeveloperDiskImage.dmg` |
| `CoreDeviceRemote` | iOS 17.0–17.3 | RemotePairing + RSD/DVT, personalized DDI |
| `CoreDeviceLockdown` | iOS 17.4+ | Lockdown CoreDeviceProxy + RSD/DVT, personalized DDI |

Verification is tracked per generation, not per iOS release: iOS 14, 15 and 16 all
take the `Legacy` branch, so one verified 14.2 device covers them. The exception is
Developer Mode, which only exists from iOS 16.

## Long-running tasks must be cancellable

`AppState.tasks` holds a `CancellationToken` per key. `replace_task` cancels the
previous holder of a key, and switching or disconnecting a device calls
`cancel_device_tasks`. Any new long-running work — logs, JIT, location, streaming —
registers there, or the session leaks past the device it belongs to.

## Conventions

- The `idevice` dependency is pinned to a git revision
  (`8eed181f39a16ea70380ec8c3cff6bed07a1ef69`). Do not bump it casually; the upgrade
  process is in `docs/PROJECT.md`.
- Destructive actions — uninstall, delete, unpair — require explicit confirmation.
- The UI is fixed to one dark theme. `.theme-clean.mode-dark` in `styles.css` carries
  the base variables; `device-lab.css` layers over it.
- Keep "integrated in code", "build passed", and "verified on a real device" as
  separate claims. Do not report the first as the third.
