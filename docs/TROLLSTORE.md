# TrollStore install and computer-side IPA install

> Status: the Apps workbench can restore the helper over USB after confirmation,
> and can offer one IPA to an installed TrollStore. On 2026-10-07, iPhone11,8 /
> iOS 17.0 (21A329) with TrollStore installed passed the transport spike:
> `devicectl --terminate-existing --payload-url` delivered
> `apple-magnifier://install?url=…` to TrollStore, and TrollStore fetched a
> cleartext `http://` LAN URL from this Mac (a 404, so nothing installed). ATS
> did not block it. A full IPA install and the helper restore have not been
> run on hardware.
> Decision: the IPA reaches the phone by a local server that TrollStore downloads
> from. The user does not copy the file into the Files app by hand.

This document is the product and safety design for two workflows:

1. Put a TrollStore helper on a supported device in one action.
2. After TrollStore is installed, choose an IPA on the Mac and have TrollStore
   install it, without a manual copy through the Files app.

The bytes of an IPA still end up on the phone. TrollStore runs on the device and
can only install a file it can read. What this design removes is the manual hop:
Finder copy, Files app, then "Install from the share sheet". The application
transfers the IPA and asks TrollStore to install it.

## What this is not

- Not an installer for iOS 17.0.1 or later. TrollStore itself does not support
  those versions, and this application must not offer a control that pretends
  otherwise.
- Not a normal Installation Proxy install. That path already exists and requires
  a signature iOS will accept. TrollStore is only the path for an IPA that route
  will not keep installed.
- Not silent. Current TrollStore still asks on the phone unless the user has
  changed its own install-alert setting. The Mac cannot dismiss that alert.

## Devices we can actually try

| Device | iOS | Helper install | IPA via TrollStore |
| --- | --- | --- | --- |
| iPhone11,8 | 17.0 (21A329) | In range | Only after TrollStore is installed and its URL scheme is turned on |
| iPhone10,1 | 14.2 | Out of range for this installer | Same, and we have no supported way to get TrollStore onto it from this app |
| iPhone14,5 | 26.5 | Unsupported | Unsupported |

The helper-install version window is the check in
[TrollRestore](https://github.com/JJTech0130/TrollRestore)'s `trollstore.py`,
not the shorter range in that repository's introduction. It allows iOS 15.0
through 16.6.x, iOS 16.7 only when the build is 20H18, and iOS 17.0 of any
build. 16.7.1 and later 16.7 builds, 17.0.1, and anything older than 15.0 are
refused. TrollRestore's own notes say 15.0 through 15.1.1 restored badly in
testing; the status page repeats that caution and still does not start a
restore. iOS 15 and 16 are not "covered by the 14.2 device" here: the installer
is a different mechanism from developer services, and it needs its own version
check against the build number, not `developer_generation()`.

## 1. One-action helper install

### What the user gets

A paired, unlocked device on a supported build. The user picks a removable
system app, defaulting to Tips, confirms, and the application restores a
helper into that app's container. The device reboots. After reboot the user
opens that app, which is now TrollHelper, and finishes installing TrollStore
there. The last confirmation stays on the phone. The Mac action stops at
"helper is in place, device is rebooting".

The replaced app is not restored by us. The only recovery is deleting it and
downloading it again from the App Store. The confirmation dialog says that in
those words, names the app, and names the iOS build it read from the device.

### How it maps onto code we already depend on

[TrollRestore](https://github.com/JJTech0130/TrollRestore) (MIT) builds a small
backup and restores it with `mobilebackup2` and `RestoreSystemFiles`. It uses
the backup path handling tracked as CVE-2024-44252, which Apple fixed in later
systems. That is why the version window is closed: a newer phone rejects the
restore, and we do not look for a replacement technique.

The pinned `idevice` already speaks the restore session
(`Mobilebackup2Client::restore_from_path`, including `RestoreSystemFiles`).
It does not build the sparse backup TrollRestore writes. That writer is a
port of TrollRestore's backup construction, kept behind the version gate, not
a general "write this file anywhere on the device" tool. The helper binary is
`PersistenceHelper_Embedded` from the current TrollStore release. It is
downloaded when the user runs the action, checked against a pinned SHA-256 and
size, and not stored in this repository. Same rule as the Developer Disk Image:
unpinned bytes do not ship inside the app.

Find My has to be off or the restore is refused by the device. The dialog says
so before the restore starts. If the device returns that error, the message
points at Settings → [name] → Find My and does not retry on its own.

### Gates before the button does anything

- USB, paired, unlocked. A network record is refused. TrollRestore is a USB
  backup session.
- ProductVersion and build are on the allowlist above. Anything else hides the
  action and says which builds work.
- The chosen app is a removable system app installed on this device. A typed
  name that does not resolve does not start a restore.
- Explicit confirmation, the destructive-dialog path, not `window.confirm`.
- One shot per device. The task registry owns the session. Disconnect cancels
  the host side; a restore already accepted by the device is not promised to
  stop, and the dialog says that.
- The helper download fails closed on a hash or length mismatch.

### After reboot

The page tells the user to open the replaced app and install TrollStore, then
to install a persistence helper into an app they can reinstall. It does not
poll for success across the reboot. A later detection pass can look for
`com.opa334.TrollStore` once the device is back, and only then offer section 2.

## 2. Choose an IPA on the Mac

### What the user gets

TrollStore is already installed. Its URL scheme is on (TrollStore settings;
it ships off). The user picks an IPA. The application serves that file on the
local network and opens

`apple-magnifier://install?url=<url-of-that-file>`

on the device. TrollStore downloads it and shows its own install confirmation.
The Mac shows progress only for the download it served. It cannot see the
install result, so the page says to finish the alert on the phone. When the
download ends, or the user cancels, or they leave the page, the server stops
and the file is no longer offered.

This matches TrollStore's own contract. `TSSettingsListController` documents
`apple-magnifier://install?url=<IPA_URL>`, and `TSSceneDelegate` handles that
URL both at cold launch and while TrollStore is already open.
`handleAppInstallFromRemoteURL` downloads with `NSURLSession` and then calls
`presentInstallationAlertIfEnabledForFile`. The default alert setting asks
every time. Setting it to skip remote installs is a TrollStore preference we
do not change from the Mac.

`apple-magnifier://enable-jit?bundle-id=` is the same scheme and is out of
scope here.

### Trigger

`devicectl device process launch` accepts `--payload-url`, described as a URL
handed to the application at launch. The candidate is launching
`com.opa334.TrollStore` with the `apple-magnifier://install?url=…` payload.
The pinned `idevice` launch API has arguments, environment, and platform
options, and no payload-URL parameter. The first implementation spike finds
whether CoreDevice accepts that URL on launch, and whether TrollStore treats
it as `openURLContexts`. Until that passes on iPhone11,8, the button stays
out of the interface.

If a direct launch does not deliver the URL, the fallback is still a URL open,
not an AFC copy into Files. A second spike can try launching the system URL
handler. It does not add a "copy to the phone, then install" step.

### Why a local cleartext server may fail

TrollStore's download uses the shared `NSURLSession` and its Info.plist sets
no App Transport Security exception. iOS applies ATS to that session, so
`http://` to the Mac is expected to fail, including an address on the LAN.
That has not been observed on our hardware. The spike records the device
error before any UI promises a local HTTP server.

If ATS rejects HTTP, the design choice is local HTTPS with a certificate the
device trusts, still bound to the LAN address of this Mac, still torn down
with the page. Installing that trust is a one-time device step and needs its
own confirmation. A public tunnel is not the fallback: the IPA would leave
the machine.

The phone has to reach the Mac. USB does not give the phone a route to the
Mac's loopback. The server binds to the Mac's LAN address, and the page
refuses to start when no such address is shared with the phone. A USB-only
phone is told to join the same network. A reverse usbmux tunnel is a later
option, not part of the first version.

### Gates

- `com.opa334.TrollStore` is installed. Otherwise the page offers section 1
  when the build allows it, and a plain refusal when it does not.
- URL scheme enabled. We cannot read that preference. The first failed open
  tells the user where the switch is, once, instead of retrying.
- The chosen path is one IPA, a regular file, size shown before start. No
  directory serving, no directory listing, one token in the URL so a guess
  does not fetch the file.
- The server accepts a connection only long enough for that download, from
  the local network, and shuts down on cancel, success, failure, device
  change, and page exit.
- The task registry holds the server. Demo mode serves nothing and launches
  nothing.

## 3. Safety rules that apply to both

- Demo mode shows a sample status and does not call the device. The restore and the IPA server do not exist in either mode yet.
- Unsupported builds never get an enabled button.
- Confirmation names the device, the build, and the concrete effect (which
  app is replaced, or which IPA will be offered to TrollStore).
- No background retry after a failed restore.
- Logs may contain the bundle id and the result. They do not contain the
  served URL's token or the IPA path beyond the file name.
- Neither workflow is a developer-service generation feature. Do not describe
  a 14.2 pass as covering the helper installer.

## 4. Verification before any of this is called done

Helper install, on iPhone11,8 / 17.0 only, with Find My off and a sacrificial
removable app (not the only copy of something the user needs):

- A 14.2 and a 26.5 device show the refusal and do not open a backup session.
- The supported device downloads the pinned helper, restores, and reboots.
- After reboot the replaced app opens as the helper.
- A hash mismatch does not start the restore.
- Find My left on produces the settings message.

IPA install, only after the helper path has produced a working TrollStore:

- With the URL scheme off, the failure tells the user to turn it on.
- With the scheme on, TrollStore downloads the served IPA. Record whether ATS
  blocked HTTP. If it did, stop and do the HTTPS design before writing the
  page.
- Cancel and leaving the page stop the server. Another device cannot fetch
  the URL afterwards.
- The install alert on the phone is expected. Skipping it is a TrollStore
  setting, not a success condition of this app.

## 5. Build order

1. Version and installed-app gates, with unit tests and no device writes.
   The Apps workbench TrollStore tab reads this and never starts a restore.
2. Spike the payload URL and ATS behaviour on iPhone11,8. Write the result
   into `PROGRESS.md` before building the page.
3. Helper restore, behind confirmation, on the 17.0 device.
4. The IPA server and launch, using whichever transport the spike showed
   actually reaches TrollStore.

Step 2 can fail the HTTP idea. If it does, section 2 stays unshipped until
the replacement transport is designed. Step 3 does not depend on it.
