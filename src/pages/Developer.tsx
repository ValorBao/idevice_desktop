import { useCallback, useEffect, useRef, useState } from 'react'
import type { Device } from '../data'
import { api, dialogs, errorMessage, events, type DeveloperStatus, type InstalledApp } from '../api'
import { useDeviceTask } from '../lib/hooks'

export function Developer({ desktop, device, onToast }: { desktop: boolean; device: Device; onToast: (message: string) => void }) {
  const [jit, setJit] = useState(!desktop)
  const [jitInfo, setJitInfo] = useState<{ bundleId: string; pid: number } | null>(desktop ? null : { bundleId: 'app.crosscode.ios', pid: 1294 })
  const [status, setStatus] = useState<DeveloperStatus>({ developerMode: desktop ? null : true, ddiMounted: !desktop, ddiImages: null, rsdAvailable: !desktop })
  const [apps, setApps] = useState<InstalledApp[]>([])
  // iOS 16 and earlier cannot launch the app for you: the instruments service
  // that would do it never answers, so JIT attaches to a running process.
  const attachesToRunningApp = Number.parseInt(device.ios.split('.')[0] ?? '0', 10) < 17
  const [bundleId, setBundleId] = useState('')
  const [ddiProgress, setDdiProgress] = useState<number | null>(null)
  const jitRef = useRef(jit)
  const runTask = useDeviceTask(desktop, onToast)
  // Developer Mode gates every developer service from iOS 16 on, so it is the
  // blocker to show first: downloading an image for a device that cannot mount
  // it yet only wastes the download.
  const blockedByDeveloperMode = desktop && status.developerMode === false
  // The one-time image only exists for iOS 16 and earlier; iOS 17 and later
  // build their own on the device, so there is nothing to offer them.
  const needsDownload = attachesToRunningApp && desktop && !status.ddiMounted && !blockedByDeveloperMode

  const refresh = useCallback(async () => {
    if (!desktop) return
    try {
      // Debuggable apps are selected by their get-task-allow entitlement, not by
      // application type: TrollStore and sideloaded builds register as System.
      const [nextStatus, nextApps] = await Promise.all([api.developerStatus(device.udid), api.appsDebuggable(device.udid)])
      setStatus(nextStatus)
      setApps(nextApps)
      setBundleId((current) => nextApps.some((app) => app.bundleId === current) ? current : nextApps[0]?.bundleId ?? '')
    } catch (error) { onToast(errorMessage(error)) }
  }, [desktop, device.udid, onToast])
  useEffect(() => { void refresh() }, [refresh])
  useEffect(() => { jitRef.current = jit }, [jit])
  useEffect(() => () => { if (desktop && jitRef.current) void api.jitStop() }, [desktop])
  useEffect(() => {
    if (!desktop) return
    let unlisten: (() => void) | undefined
    let disposed = false
    void events.ddiProgress((progress) => setDdiProgress(progress.percent))
      .then((stop) => { if (disposed) stop(); else unlisten = stop })
    return () => { disposed = true; unlisten?.() }
  }, [desktop])

  // The image is downloaded once and reused, so this runs at most once per Mac.
  // The progress bar is cleared before the error is handed on, so the shared
  // handler still reports it.
  const downloadDdi = () => runTask(async () => {
    setDdiProgress(0)
    try {
      await api.ddiDownload(device.udid)
    } catch (error) {
      setDdiProgress(null)
      throw error
    }
    setDdiProgress(null)
    onToast('Developer Disk Image installed and mounted')
    await refresh()
  }, 'Downloading the Developer Disk Image is available in the desktop app')

  const toggleJit = () => runTask(async () => {
    if (jit) {
      await api.jitStop()
      setJit(false)
      setJitInfo(null)
    } else {
      if (!bundleId) return onToast('Choose a debuggable app first')
      const session = await api.jitStart(bundleId, device.udid)
      setJit(true)
      setJitInfo(session)
    }
  }, () => setJit((value) => !value))

  // Enabling restarts the device, which the request itself gives no warning
  // about, so the restart is confirmed here before anything is sent. The
  // confirmation goes through the dialog plugin: `window.confirm` resolves to
  // false under Tauri and would silently cancel the action.
  const developerAction = (action: 'reveal' | 'enable' | 'accept') => runTask(async () => {
    if (action === 'reveal') {
      await api.developerReveal(device.udid)
      onToast('Developer Mode is now visible in Settings')
    }
    if (action === 'enable') {
      const confirmed = await dialogs.confirmDestructive(
        `Enabling Developer Mode restarts ${device.name}. Save anything open on the device first. After it restarts, unlock it and choose Accept after reboot to finish.`,
        'Restart and enable',
      )
      if (!confirmed) return
      try {
        await api.developerEnable(device.udid)
      } catch (error) {
        // A device with a passcode refuses to have Developer Mode turned on
        // remotely, which is most devices. That is not a failure to report as
        // one: the switch still exists, it just has to be flipped on the
        // device. Verified on iPhone10,4 / iOS 16.7.16.
        if (/passcode/i.test(errorMessage(error))) {
          await api.developerReveal(device.udid).catch(() => {})
          return onToast(`${device.name} has a passcode, so it must be enabled on the device: Settings › Privacy & Security › Developer Mode. It restarts when you turn it on`)
        }
        throw error
      }
      onToast(`${device.name} is restarting · unlock it, then choose Accept after reboot`)
    }
    if (action === 'accept') {
      await api.developerAccept(device.udid)
      onToast('Developer Mode confirmed')
    }
    await refresh()
  }, 'Developer Mode controls are available in the desktop app')

  return (
    <section className="developer-page page-padding">
      <div className="dev-top-grid">
        <div className="card jit-card"><div><h2>Enable JIT</h2><p>{attachesToRunningApp ? 'Open the app on the device first, then attach debugserver to keep its JIT entitlement active.' : 'Launch a debuggable app, attach debugserver, and keep its JIT entitlement active.'}</p>{desktop && (apps.length
          ? <select value={bundleId} onChange={(event) => setBundleId(event.target.value)}>{apps.map((app) => <option key={app.bundleId} value={app.bundleId}>{app.name} · {app.bundleId}</option>)}</select>
          : <p className="jit-empty">No installed app allows debugging. Attaching requires the <code>get-task-allow</code> entitlement, which App Store and TestFlight builds never carry. Install a development-signed or sideloaded build to use JIT.</p>)}</div><button className={`toggle ${jit ? 'on' : ''}`} onClick={() => void toggleJit()} disabled={desktop && !jit && !apps.length} aria-label={jit ? 'Stop JIT session' : 'Start JIT session'}><span /></button><small><i className={jit ? 'good-dot' : ''} />{jit ? `debugserver attached${jitInfo ? ` · pid ${jitInfo.pid}` : ''}` : 'no process attached'}</small></div>
        {/* Mounting itself has no control: it happens when the device is
            selected. The one action left is the first-time download, which
            reaches the network and so is never taken without being asked. */}
        <div className="card ddi-card"><h2>Developer Services</h2><p><span>Developer Disk Image</span><b className={status.ddiMounted ? 'good' : ''}>{status.ddiMounted ? 'Mounted' : 'Not mounted'}</b></p><p><span>Developer Mode</span><b>{status.developerMode === null ? 'Unknown' : status.developerMode ? 'Enabled' : 'Disabled'}</b></p><p><span>RSD</span><b>{status.rsdAvailable ? 'Available' : 'Unavailable'}</b></p>{ddiProgress !== null && <div className="progress"><span style={{ width: `${ddiProgress}%` }} /></div>}
          {blockedByDeveloperMode
            ? <p className="ddi-blocked" role="status"><b>Turn on Developer Mode on {device.name}</b><span>Settings › Privacy &amp; Security › Developer Mode. The device restarts when you turn it on; unlock it afterwards and confirm the prompt. Developer services, including the disk image, cannot start until then.</span></p>
            : needsDownload
              ? <><small className="ddi-note">iOS 16 and earlier need a 19 MB Developer Disk Image. It is downloaded once, kept in your home directory, and reused by every device afterwards.</small><div className="ddi-actions"><button className="primary-button" onClick={() => void downloadDdi()} disabled={ddiProgress !== null}>{ddiProgress !== null ? `Downloading… ${ddiProgress}%` : 'Download and mount'}</button></div></>
              : <small className="ddi-note">{status.ddiMounted ? 'Mounted automatically when this device was selected.' : 'Mounting is automatic on connection. Reconnect the device to retry.'}</small>}
        </div>
      </div>
      <div className="developer-mode-actions"><button onClick={() => void developerAction('reveal')}>Show Developer Mode setting</button><button onClick={() => void developerAction('enable')}>Enable Developer Mode</button><button onClick={() => void developerAction('accept')}>Accept after reboot</button></div>
      <div className="service-grid">{[['RSD Tunnel', status.rsdAvailable ? 'remoted handshake available' : 'requires iOS 17+ and developer services', status.rsdAvailable], ['Developer Image', status.ddiMounted ? 'developer services ready' : 'not mounted · reconnect to retry', status.ddiMounted], ['Debug Proxy', jit ? `attached to ${jitInfo?.bundleId ?? 'process'}` : 'idle · ready to attach', jit]].map(([name, detail, good]) => <div className="card" key={String(name)}><b><i className={good ? 'good-dot' : 'warn-dot'} />{name}</b><small>{detail}</small></div>)}</div>
      <div className="debug-console"><header><i />debugserver · com.apple.debugserver.DVTSecureSocketProxy</header><div>{jitInfo ? <><p><code>process </code>launch &quot;{jitInfo.bundleId}&quot;</p><p>Process {jitInfo.pid} launched and attached</p><p>memory limit disabled · JIT session active ✓</p></> : <p>No active debug session.</p>}<p className="terminal-cursor"><code>(lldb)</code><i /></p></div></div>
    </section>
  )
}
