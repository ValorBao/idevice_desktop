# idevice desktop Feature Delivery Plan

> Last updated: 2026-07-26
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

### Later, driven by validated demand

1. Provisioning profile inspection, expiry warnings, install, and confirmed removal.
2. Notification observation with an explicit subscription list.
3. Pasteboard text transfer with privacy guidance; image transfer later.
4. XCTest and WDA only after runner selection, logs, cancellation, and port bridging
   can be presented as one understandable workflow.

High-risk activation, backup/restore, restore mode, HID injection, and Preboard do
not enter the near-term roadmap. They require dedicated safety designs and must not
appear as convenient quick actions.

## 5. Planning Decision

The next implementation cycle is the **current-surface acceptance pass**, followed
by the **Processes protocol proof**. No Process navigation or mock-only controls are
added until a real process list has been obtained on supported hardware and its
cleanup behavior is known.
