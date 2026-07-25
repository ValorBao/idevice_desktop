import { useCallback, useEffect, useRef, useState } from 'react'
import type { Device } from '../data'
import { api, dialogs, errorMessage, events, type DeveloperStatus, type InstalledApp } from '../api'

export function Developer({ desktop, device, onToast }: { desktop: boolean; device: Device; onToast: (message: string) => void }) {
  const [jit, setJit] = useState(!desktop)
  const [jitInfo, setJitInfo] = useState<{ bundleId: string; pid: number } | null>(desktop ? null : { bundleId: 'app.crosscode.ios', pid: 1294 })
  const [status, setStatus] = useState<DeveloperStatus>({ developerMode: desktop ? null : true, ddiMounted: !desktop, ddiImages: null, rsdAvailable: !desktop })
  const [apps, setApps] = useState<InstalledApp[]>([])
  const [bundleId, setBundleId] = useState('')
  const [ddiProgress, setDdiProgress] = useState<number | null>(null)
  const jitRef = useRef(jit)

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
    events.ddiProgress((progress) => setDdiProgress(progress.percent)).then((stop) => { unlisten = stop })
    return () => unlisten?.()
  }, [desktop])

  const toggleJit = async () => {
    if (!desktop) return setJit((value) => !value)
    try {
      if (jit) {
        await api.jitStop()
        setJit(false)
        setJitInfo(null)
      } else {
        if (!bundleId) return onToast('Choose a user application first')
        const session = await api.jitStart(bundleId, device.udid)
        setJit(true)
        setJitInfo(session)
      }
    } catch (error) { onToast(errorMessage(error)) }
  }

  const developerAction = async (action: 'reveal' | 'enable' | 'accept') => {
    if (!desktop) return onToast('Developer Mode controls are available in the desktop app')
    try {
      if (action === 'reveal') await api.developerReveal(device.udid)
      if (action === 'enable') await api.developerEnable(device.udid)
      if (action === 'accept') await api.developerAccept(device.udid)
      onToast(action === 'reveal' ? 'Developer Mode is now visible in Settings' : 'Developer Mode request sent')
      await refresh()
    } catch (error) { onToast(errorMessage(error)) }
  }

  const mountDdi = async () => {
    if (!desktop) return onToast('DDI mounting is available in the desktop app')
    try {
      const image = await dialogs.file('Developer Disk Image', ['dmg'])
      if (!image || Array.isArray(image)) return
      const major = Number.parseInt(device.ios.split('.')[0] ?? '0', 10)
      setDdiProgress(0)
      if (major >= 17) {
        const manifest = await dialogs.file('Build Manifest', ['plist'])
        const trust = await dialogs.file('Trust cache', ['trustcache', 'img4'])
        if (!manifest || Array.isArray(manifest) || !trust || Array.isArray(trust)) return setDdiProgress(null)
        await api.ddiMount({ imagePath: image, manifestPath: manifest, trustCachePath: trust }, device.udid)
      } else {
        const signature = await dialogs.file('Disk Image Signature', ['signature'])
        if (!signature || Array.isArray(signature)) return setDdiProgress(null)
        await api.ddiMount({ imagePath: image, signaturePath: signature }, device.udid)
      }
      setDdiProgress(100)
      onToast('Developer Disk Image mounted')
      await refresh()
    } catch (error) { setDdiProgress(null); onToast(errorMessage(error)) }
  }

  const autoMountDdi = async () => {
    if (!desktop) return onToast('DDI mounting is available in the desktop app')
    try {
      setDdiProgress(0)
      await api.ddiMountAuto(device.udid)
      setDdiProgress(100)
      onToast('Developer Disk Image mounted automatically')
      await refresh()
    } catch (error) { setDdiProgress(null); onToast(errorMessage(error)) }
  }

  const unmountDdi = async () => {
    if (!desktop) return onToast('DDI mounting is available in the desktop app')
    try {
      await api.ddiUnmount(device.udid)
      onToast('Developer Disk Image unmounted')
      await refresh()
    } catch (error) { onToast(errorMessage(error)) }
  }
  return (
    <section className="developer-page page-padding">
      <div className="dev-top-grid">
        <div className="card jit-card"><div><h2>Enable JIT</h2><p>Launch a debuggable app, attach debugserver, and keep its JIT entitlement active.</p>{desktop && (apps.length
          ? <select value={bundleId} onChange={(event) => setBundleId(event.target.value)}>{apps.map((app) => <option key={app.bundleId} value={app.bundleId}>{app.name} · {app.bundleId}</option>)}</select>
          : <p className="jit-empty">No installed app allows debugging. Attaching requires the <code>get-task-allow</code> entitlement, which App Store and TestFlight builds never carry. Install a development-signed or sideloaded build to use JIT.</p>)}</div><button className={`toggle ${jit ? 'on' : ''}`} onClick={() => void toggleJit()} disabled={desktop && !jit && !apps.length}><span /></button><small><i className={jit ? 'good-dot' : ''} />{jit ? `debugserver attached${jitInfo ? ` · pid ${jitInfo.pid}` : ''}` : 'no process attached'}</small></div>
        <div className="card ddi-card"><h2>Developer Disk Image</h2><p><span>Status</span><b className={status.ddiMounted ? 'good' : ''}>{status.ddiMounted ? 'Mounted' : 'Not mounted'}</b></p><p><span>Developer Mode</span><b>{status.developerMode === null ? 'Unknown' : status.developerMode ? 'Enabled' : 'Disabled'}</b></p><p><span>RSD</span><b>{status.rsdAvailable ? 'Available' : 'Unavailable'}</b></p>{ddiProgress !== null && <div className="progress"><span style={{ width: `${ddiProgress}%` }} /></div>}<div className="dev-actions"><button className="primary-button" onClick={() => void autoMountDdi()}>Auto Mount DDI</button><button onClick={() => void mountDdi()}>Choose files</button><button onClick={() => void unmountDdi()} disabled={!status.ddiMounted}>Unmount</button></div></div>
      </div>
      <div className="developer-mode-actions"><button onClick={() => void developerAction('reveal')}>Show Developer Mode setting</button><button onClick={() => void developerAction('enable')}>Enable Developer Mode</button><button onClick={() => void developerAction('accept')}>Accept after reboot</button></div>
      <div className="service-grid">{[['RSD Tunnel', status.rsdAvailable ? 'remoted handshake available' : 'requires iOS 17+ and developer services', status.rsdAvailable], ['Developer Image', status.ddiMounted ? 'developer services ready' : 'mount an image for legacy services', status.ddiMounted], ['Debug Proxy', jit ? `attached to ${jitInfo?.bundleId ?? 'process'}` : 'idle · ready to attach', jit]].map(([name, detail, good]) => <div className="card" key={String(name)}><b><i className={good ? 'good-dot' : 'warn-dot'} />{name}</b><small>{detail}</small></div>)}</div>
      <div className="debug-console"><header><i />debugserver · com.apple.debugserver.DVTSecureSocketProxy</header><div>{jitInfo ? <><p><code>process </code>launch &quot;{jitInfo.bundleId}&quot;</p><p>Process {jitInfo.pid} launched and attached</p><p>memory limit disabled · JIT session active ✓</p></> : <p>No active debug session.</p>}<p className="terminal-cursor"><code>(lldb)</code><i /></p></div></div>
    </section>
  )
}
