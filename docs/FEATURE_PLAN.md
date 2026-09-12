# idevice_desktop Feature Delivery Plan

> Last updated: 2026-08-29
> Principle: ship complete user workflows, not protocol exposure.

This plan turns the capability backlog into a sequence of small, usable features. A
backend command existing is not progress a user can rely on. A feature enters the
interface only when its main path, failure states, cleanup, tests, and real-device
result are all understood.

## 1. Product Rules

### Keep the default path obvious

- Each state has at most one primary action.
- Common settings are chosen automatically. Advanced parameters stay behind an
  **Advanced** disclosure and are never required for the first successful run.
- The interface uses user language first. Service and protocol names remain
  available as secondary diagnostic context.
- A capability is added to navigation only when it works. Do not ship disabled
  “coming soon” tabs.

### Make availability explicit

- Detect pairing, connection route, iOS generation, Developer Mode, DDI state,
  permissions, and required services before starting an operation.
- If a generation is unsupported, say so directly and explain the supported path.
  Do not leave a control enabled and wait for a protocol error.
- If the user can repair a prerequisite, provide one next action such as
  **Mount Developer Image** or **Connect over USB**.
- Browser demo behavior demonstrates layout and state only. It never counts as
  desktop or real-device verification.

### Make every operation finish cleanly

- Operations longer than one second show progress or a live running state.
- Continuous and long-running work always has a visible **Stop** or **Cancel** action.
- Device switching, disconnect, page exit, and window exit stop the task and release
  its service connection.
- Partial local or device files are removed after cancellation or failure.
- Destructive actions name the target and require confirmation.

### Represent every state

Every feature explicitly handles:

1. loading;
2. empty result;
3. unsupported device or system;
4. missing prerequisite;
5. permission or trust denial;
6. running and cancelling;
7. device disconnect during work;
8. success with an inspectable or exportable result;
9. failure with a useful next step.

## 2. Definition of Done

A visible feature is complete only when all of these gates pass:

1. **Outcome** — its user outcome can be stated in one sentence.
2. **Feasibility** — the real backend path is exercised through a harness on every
   applicable developer-service generation before the full UI is built.
3. **Simple UI** — the default workflow needs no protocol knowledge and no more
   configuration than the operation genuinely requires.
4. **Lifecycle** — cancellation and cleanup pass on stop, page exit, device switch,
   disconnect, and failure.
5. **Safety** — destructive behavior has proportional confirmation and validation.
6. **Frontend tests** — primary interaction, rejection, error, and cleanup states are
   covered in Vitest for desktop and demo branches where both exist.
7. **Rust tests** — parsing, validation, transport selection, and task-state decisions
   are covered without pretending mocks prove the device protocol.
8. **CI** — Frontend and Rust GitHub Actions checks are green.
9. **Hardware** — date, device, iOS generation, connection, result, and relevant error
   are recorded in `PROGRESS.md`.
10. **Documentation** — `CAPABILITY_MATRIX.md` and user-facing limitations describe
    the result honestly.

Until all ten pass, work may exist behind a harness or internal command, but it is
not added to normal navigation and is not called Covered.

## 3. Delivery Shape

Each major feature is delivered in three bounded slices:

1. **Protocol proof** — a harness and backend types establish support, prerequisites,
   cancellation, and generation boundaries. No visible dead UI.
2. **Complete workflow** — the smallest useful interface, frontend tests, errors,
   and cleanup are added together.
3. **Hardware acceptance** — the workflow is exercised through the actual desktop
   interface, gaps are fixed, and only then is coverage status upgraded.

One feature is active at a time. A new feature does not begin while the current one
has an exposed but unverified main path.

## 4. Ordered Roadmap

### Foundation: finish the current surface

Before adding navigation, close the highest-value acceptance gaps in existing pages:

- exercise Location through the desktop interface on Legacy and iOS 17+;
- exercise JIT through the interface rather than only a harness;
- verify large crash-report preview behavior and mid-operation disconnects;
- verify sleeping-device and cold-start association behavior;
- add regression cases when any of these sessions exposes a defect.

This is complete when every current page has a recorded main path and cleanup path,
or an explicit limitation that cannot presently be exercised. Record each hardware
session in [`ACCEPTANCE_LOG.md`](ACCEPTANCE_LOG.md).

### Feature 1: Processes

**Outcome:** find a running application or process and stop or launch it without
using a command line.

Processes becomes the second tab beside the existing live Logs workflow. Once both
exist, the navigation label changes from **Logs** to **Monitor**. Do not add empty
Performance or Network tabs early.

Smallest useful workflow:

- searchable process list with name, pid, application identity, and running state;
- manual refresh plus a restrained automatic refresh;
- launch an eligible application;
- stop a process with a target-specific confirmation;
- clear prerequisite state for Developer Mode and DDI;
- an **Advanced** menu for signals only after ordinary stop is reliable.

Not in the first slice:

- an interactive terminal;
- arbitrary standard-input forwarding;
- process automation or saved scripts;
- pretending the Legacy generation supports listing if its instruments service
  remains unresponsive.

Acceptance focus:

- prove the process list and stop path first on CoreDeviceRemote and
  CoreDeviceLockdown;
- probe Legacy separately and show an honest unsupported state if the verified
  service cannot provide the workflow;
- ensure refresh and control cannot target the previously selected device.

### Feature 2: Network Capture

**Outcome:** capture device traffic into a PCAP file that opens in Wireshark.

This becomes a **Network** tab in Monitor only after the complete capture-to-file
round trip passes.

Smallest useful workflow:

- **Start Capture** with a safe default of all available device traffic;
- live duration, packet count, byte count, and output size;
- **Stop and Save** plus **Cancel and Delete**;
- optional process or interface filter under **Advanced**;
- disk-space and destination validation before capture;
- a privacy note explaining that packet contents may contain sensitive data.

Not in the first slice:

- a full packet decoder;
- a live Wireshark replacement;
- multiple simultaneous captures.

Acceptance focus:

- saved PCAP opens and contains packets;
- cancellation deletes the temporary file;
- device disconnect finalizes or clearly marks the partial capture;
- long captures do not accumulate data in application memory.

### Feature 3: Performance

**Outcome:** identify which process is consuming CPU or memory over time.

This becomes a **Performance** tab in Monitor after process identity is already
stable from Feature 1.

Smallest useful workflow:

- top processes by CPU and memory;
- select one process to see a short rolling time series;
- pause, resume, and export CSV;
- a visible sample interval with a sensible default;
- bounded in-memory history.

Not in the first slice:

- energy, thermal, graphics, and every sysmontap field at once;
- customizable dashboards;
- permanent recording.

Acceptance focus:

- values remain associated with the correct pid as processes start and exit;
- pause and page exit stop sampling;
- high-frequency samples do not freeze rendering;
- missing fields are shown as unavailable rather than zero.

### Feature 4: Live Screen

**Outcome:** view the current device screen and take a still image.

This stays a separate visual tool rather than another Monitor tab.

Smallest useful workflow:

- start and stop live viewing;
- connection and frame-rate state;
- take a still image using the existing screenshot path;
- scale-to-fit without changing device orientation;
- pause automatically when the page is hidden or the device disconnects.

Recording, input injection, and remote control remain out of the first slice because
they add separate performance, privacy, and safety requirements.

The first slice is now integrated as a bounded 2 FPS PNG-refresh session. It keeps
one screenshot service open, supports Legacy Screenshotr plus modern DVT/RSD,
validates every PNG and retains only the latest raw frame for still export. The
interface registers listeners before startup, preserves the last frame after Stop,
supports fit and 100% scale, and stops on page exit, hidden window, device switch,
or disconnect. Automated checks pass; hardware acceptance and the upstream HEVC
display stream remain separate follow-up work.

### Feature 5: Provisioning Profiles

**Outcome:** understand which signing profiles are installed and which require
attention without changing device configuration.

The read-only first slice is integrated. It lists profiles through Misagent over
USB Lockdown or modern RSD, extracts the embedded plist locally, and normalizes
name, UUID, team, application identifier, platform, device scope, debug entitlement,
type, creation date, and expiration state. Search and attention/development/
distribution filters are available, and an unreadable profile remains visible as
an error row rather than making the whole list fail. Parsing is bounded to 512
profiles and 8 MB per profile. Automated checks pass; hardware acceptance remains
pending. This slice extracts metadata from the device-returned profile but does not
independently verify its CMS signer chain.

Installation and removal remain out of this slice. Both mutate signing state and
need file validation, a precise target confirmation, post-operation refresh, and a
tested recovery path before controls are exposed.

### Feature 6: Notification Observation

**Outcome:** watch selected device-state notifications without posting commands or
collecting unrelated events.

The read-only first slice is integrated as a Monitor tab. Listening starts only
after the user explicitly selects one or more common or custom notification names.
The backend validates and deduplicates at most 32 names, routes Notification Proxy
through USB Lockdown or the appropriate modern RSD tunnel, and attaches a session
identifier to every state change and event so a cancelled listener cannot overwrite
a restarted one. Pause, page exit, device switch, and disconnect stop the device
task. The interface retains the newest 500 names, supports filtering and clearing,
and has bounded synthetic demo events.

The pinned upstream Notification Proxy client relays the notification name only;
there is no payload in this workflow. Posting is intentionally absent and would
require a separate allowlist and safety design. Automated checks pass; hardware
acceptance remains pending.

### Feature 7: Pasteboard Text and Image Transfer

**Outcome:** move deliberate plain-text snippets or bounded images to or from a
modern device without silently inspecting or synchronizing clipboard contents.

The first slice is integrated as a dedicated Developer page for iOS 17 and later.
Device text is read only after the user presses the read control. The backend first
requests promised metadata for every pasteboard item, then resolves only a supported
plain-text UTI whose advertised size is at most 1 MB. Empty, non-text, invalid
UTF-8, unknown-size, and oversized states remain explicit instead of being coerced
into text.

Writing accepts at most 1 MB of UTF-8 text and requires a confirmation naming the
target device and exact character/byte count because it replaces the general
pasteboard. Both directions use the generation-appropriate RemotePairing or
CoreDeviceProxy RSD tunnel, time out after 45 seconds, and are cancelled when the
device session changes. iOS 16 and earlier show an explicit unsupported error.

The image mode follows the same explicit lifecycle for PNG and JPEG only. Device
reads request promised metadata first and refuse unknown-size or larger-than-12-MB
items without resolving them. Local writes require an absolute selected file,
matching extension and signature, bounded encoded size, dimensions no larger than
8,192 per side, and a 32-megapixel preview budget. A device-bound preparation token
retains the validated bytes only until the user clears the selection, leaves the
page, changes devices, or completes a confirmed write. The preview and confirmation
name the file, target, dimensions, MIME type, and byte size. TIFF, transcoding,
background monitoring, and automatic host clipboard access are absent. Automated
checks pass; hardware acceptance remains pending.

### Feature 8: XCTest Runner Preflight and Run Planning

**Outcome:** identify a usable XCTest runner and every missing prerequisite before
the application is allowed to launch a test process.

The read-only preflight slice is integrated as Test Lab. It queries Installation
Proxy once for all registered applications, retains at most 128 `.xctrunner` or
`-Runner` candidates and 512 ordinary user/debuggable target applications, and
never fetches icons or application bytes. Each runner reports its executable,
required application path/container metadata, `get-task-allow` entitlement, and
whether it appears to be WebDriverAgent. Search, runner selection, and an optional
target selection are available in both desktop and bounded demonstration modes.

The page combines the selected candidate with iOS generation, Developer Mode,
Developer Disk Image, and RSD readiness. Legacy Lockdown TestManager/DVT and iOS
17.4+ CoreDeviceProxy/RSD are represented as executable routes. iOS 17.0–17.3 is
explicitly blocked because the application has not yet adapted XCTest to its
RemotePairing/RSD tunnel; the interface does not mistake a mounted image for a
working execution route. Refresh is cancellable on a device-session change and
times out after 45 seconds.

The plan editor records Standard XCTest or WDA bridge intent, an optional target,
deduplicated include/skip identifiers, and a bounded timeout. Validation re-reads
Installation Proxy metadata instead of trusting the visible snapshot, rejects a
removed or no-longer-debuggable runner/target, prevents overlapping filters, and
applies separate Standard XCTest and WDA timeout limits. WDA plans accept only a
WDA-classified runner and clear target and filter fields. The validated preview is
discarded whenever any input changes.

This slice deliberately exposes no Run button. Listener-first event output,
wall-clock timeout enforcement, deterministic Stop and cleanup behavior, WDA
readiness, and localhost HTTP/MJPEG bridging must be delivered together before
process launch is enabled. Automated checks pass; hardware acceptance remains
pending.

### Later, driven by validated demand

1. Provisioning profile install and confirmed removal after the read-only workflow is accepted.
2. Complete XCTest and WDA execution only when launch parameters, event output,
   timeout, Stop, RemotePairing support, cleanup, readiness, and port bridging can
   be presented as one understandable workflow.
3. Accept the new Apple Account Personal Signing flow on a dedicated account and
   device before release. Cover 2FA, multiple-team behavior, certificate limits,
   Keychain denial, Anisette/network failures, export/install cleanup, and sign-out.
   `.p12` import and broader bundle behavior remain separate safety designs.

High-risk activation, backup/restore, restore mode, HID injection, and Preboard do
not enter the near-term roadmap. They require dedicated safety designs and must not
appear as convenient quick actions.

## 5. Planning Decision

The current-surface acceptance pass remains open for its recorded hardware gaps.
The Processes protocol proof has produced real lists on both modern generations and
a complete production AppService launch/stop cleanup result on iOS 17.0, so its
smallest useful Monitor workflow is integrated. DVT stop remains explicitly read-only
after an identity-checked `killPid:` failed to terminate the designated test app.

On 2026-08-09 the user chose to stop additional boundary testing and continue new
feature development. Performance moved ahead of Network Capture and its smallest
CPU/memory workflow is integrated: dynamic sysmontap schemas, stable process identity,
bounded rolling history, pause/resume, filtering, missing-value handling, and CSV
export. Network Capture then followed with streaming PCAP output, live statistics,
optional PID/interface filters, disk safeguards, atomic stop-and-save, and
cancel-and-delete cleanup. Both workflows pass their automated frontend and Rust
checks. Live Screen followed as a separate visual tool using a persistent Screenshotr
or DVT screenshot session, bounded PNG events, current-frame export, and automatic
cleanup. It also passes automated checks. Hardware acceptance is intentionally still
pending. Provisioning Profiles then added a separate read-only Misagent workflow
with local signed-plist parsing, expiry warnings, search, filters, and malformed-row
isolation. It passes automated checks but has not been accepted on hardware; install
and removal are intentionally absent. Notification Observation then added explicit
read-only subscriptions, a bounded/filterable name timeline, generation-aware
transport routing, session isolation, and automatic cleanup. It passes automated
checks but has not been accepted on hardware; payload display and posting are
intentionally absent. Capability coverage remains Partial and no real-device result
is implied. Pasteboard transfer followed with explicit reads, confirmed writes,
promised-metadata filtering, a 1 MB UTF-8 text limit, modern-generation RSD routing,
and device-session cancellation. The workflow then added PNG/JPEG reads and writes
with a 12 MB encoded limit, safe dimension budget, local preview, device-bound
preparation tokens, and cleanup on clear, page exit, device change, or successful
write. It passes automated checks but has not been accepted on hardware; background
monitoring, host clipboard access, TIFF, and transcoding are intentionally absent.
Test Lab then added bounded, read-only runner and target discovery plus explicit
checks for runner metadata, debug entitlement, Developer Mode, DDI, and the selected
generation's TestManager/DVT route. It passes automated checks but does not launch
tests; the iOS 17.0–17.3 RemotePairing adapter, lifecycle/event controls, WDA bridge,
and hardware acceptance remain pending.

On 2026-08-29 development moved to the 0.0.3 release branch and returned to
hardware acceptance. The first Performance desktop run exposed a real Sysmontap
configuration defect: output frequency had been coupled to the requested sample
period, and system-only protocol rows erased the visible process list. After
separating those concerns, the rebuilt app sustained 26 one-second samples with 80
visible process CPU/memory rows on iOS 17.0. The same session accepted the Live
Screen main preview, read-only Lockdown profile inspection, and Test Lab's
empty-runner/unsupported-route state. Network Capture, Notifications, and Pasteboard
were intentionally limited to their safe initial states so no packet file,
subscription, or clipboard access occurred without a dedicated acceptance action.

On 2026-08-30 development moved to 0.0.4 for a bounded Personal Signing Assistant.
The first slice takes an IPA and provisioning profile, validates archive paths and
size, checks bundle ID, expiry, and the selected device, compares the SHA-1 hashes of
the profile's embedded developer certificates against valid macOS Keychain code-
signing identities, specializes wildcard application/keychain entitlements, signs in
a UUID-scoped temporary workspace, verifies the result with `codesign`, and atomically
exports a new IPA before offering the existing device installer. The source IPA is
never overwritten. The workflow refuses symbolic links, app extensions, nested or
Watch applications, expired profiles, unauthorized devices, and mismatched identities.
The page now also defaults to an iLoader-compatible Apple Account mode backed by
`isideload`. It requests the password only for the current login, clears the field
immediately, zeroizes the Rust-side value, never persists the password, and stores
certificate/Anisette material only in macOS Keychain. After 2FA it discovers the
first developer team, registers the selected device, creates or reuses development
assets, signs, exports, and hands the output to the existing installer. The route is
explicitly marked unofficial because it uses Apple private endpoints and the
community `ani.sidestore.io` service. Automated checks and browser visual acceptance
pass; real-account login and signing remain a release gate. Distribution is also
blocked because the resolved `nab138/apple-crates` `apple-codesign` package declares
no license; it must be licensed or replaced before packaging. `.p12` import remains
out of scope.
