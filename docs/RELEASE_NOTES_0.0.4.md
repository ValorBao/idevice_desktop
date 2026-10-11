# idevice_desktop 0.0.4 — Draft Release Notes

> Development build. Not yet published or accepted as a release.
> The signing stack now resolves the LGPL-2.1-or-later `apple-codesign-quick` library; packaging still requires the complete dependency inventory to be regenerated and real-account signing acceptance.

## Personal Signing Assistant

- Adds a dedicated Personal Sign page with Apple Account and Local Profile modes.
- Adds an iLoader-compatible Apple Account login backed by `isideload`, including a six-digit 2FA prompt, automatic developer-team discovery, current-device registration, development certificate/profile acquisition, IPA signing, export, and installation handoff.
- Keeps the previous local mode for selecting an IPA and provisioning profile.
- Checks archive safety, the main app bundle identifier, profile expiry, current-device coverage, and supported bundle shape before signing.
- Reads valid code-signing identities from macOS Keychain and accepts only an identity whose certificate fingerprint is embedded in the selected provisioning profile.
- Replaces the embedded profile, specializes wildcard application and keychain entitlements, signs frameworks and the main app in a disposable workspace, verifies the result with `codesign`, and exports a new IPA without changing the source.
- Offers the existing signed-IPA installation path after a successful export.
- Emits visible progress for preflight, workspace preparation, signing, verification, packaging, and installation.

## Privacy and Safety Boundary

- The Apple Account password is requested only by the Apple Account mode, cleared from the interface as login begins, held in zeroizing Rust memory for that login attempt, and never persisted by idevice_desktop. Verification codes are accepted only while a login request is waiting and expire after three minutes.
- Certificate material and Anisette state are stored only through macOS Keychain. There is no plaintext-storage fallback and the Apple Account password is not saved to Keychain.
- The Apple Account mode is an unofficial compatibility flow: it contacts Apple private developer endpoints and the community Anisette service `https://ani.sidestore.io`. It may stop working when Apple or that service changes.
- The first release does not import `.p12` files or handle certificate passwords.
- App extensions, nested applications, Watch applications, symbolic links, unsafe archive paths, archives expanding beyond 4 GB, expired profiles, bundle/profile mismatches, device/profile mismatches, and non-matching Keychain identities are blocked.
- Temporary signing files are removed after success or failure, and the original IPA is never overwritten.
- The selected profile's entitlement set replaces the source app entitlements; the preflight warns that capabilities absent from the profile may not work.

## Task Lifecycle Hardening

- Log data and status events carry the device UDID and a per-mount session ID. Delayed subscriptions are released on unmount, old sessions cannot stop newer streams, and stream errors/stops are reflected in the interface. Log connection setup is cancellable with a 30-second timeout.
- Apple Account signing has an operation-specific Cancel action and filters progress/results from obsolete operations. Leaving the page, switching devices, or signing out cancels the current signing operation. Cancellation keeps the backend operation slot occupied until its worker unwinds, so retries cannot overlap cleanup.
- Account status/sign-out no longer wait on the signing lock. Stages have bounded waits: inspection/copy/packaging 120 seconds, session acquisition 30 seconds, team lookup/device registration 60 seconds, signing 600 seconds. Upstream synchronous work can only observe cancellation or timeout after returning; completed Apple registrations are retained.
- Packaging cancellation kills and reaps the ZIP child before deleting temporary files. Successful export replaces the destination by rename without first deleting the previous export.
- Automated regression coverage includes StrictMode/late subscription cleanup, device/session isolation, cancellation before startup, operation ownership, subprocess termination, and temporary-file cleanup. Real Apple Account export/install and real-device log lifecycle acceptance remain pending.

## Validation

- Frontend production build passes.
- 95 frontend tests pass, including password-field clearing, 2FA gating, account-signing request construction, local file selection, preflight, blockers, export, install handoff, demo isolation, and navigation.
- 130 Rust tests pass, including account status/error boundaries, identity parsing, bundle wildcard matching, entitlement specialization, and the TypeScript/Rust serialization contract.
- Normal and 820×650 browser-demo visual checks pass with no console warnings or errors.
- The production preflight harness parsed a real IPA and a bounded temporary wildcard profile, passed its archive/bundle/device/expiry checks, and correctly stopped at the missing matching Keychain identity. It did not export, install, or execute the IPA.
- Real Apple Account login, 2FA, certificate/profile creation, IPA export, and installation were not attempted because no test credentials were supplied. Real local IPA export and installation with a matching personal development identity also remain pending.
