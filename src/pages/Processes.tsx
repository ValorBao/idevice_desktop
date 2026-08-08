import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Play, RefreshCw, Search, Square } from 'lucide-react'
import { demoProcesses, installedApps } from '../data'
import {
  api,
  dialogs,
  errorMessage,
  type InstalledApp,
  type ProcessSnapshot,
  type ProcessSummary,
} from '../api'

const demoSnapshot: ProcessSnapshot = {
  processes: demoProcesses,
  transport: 'DVT DeviceInfo · demonstration',
  available: true,
  supportsLaunch: true,
  supportsStop: true,
  limitation: null,
}

const demoApps: InstalledApp[] = installedApps
  .filter((app) => !app.system)
  .map((app) => ({
    bundleId: app.bundle,
    name: app.name,
    version: app.version,
    sizeBytes: 0,
    system: false,
    iconDataUrl: null,
    raw: null,
  }))

export function Processes({ desktop, udid, onToast }: { desktop: boolean; udid: string; onToast: (message: string) => void }) {
  const [snapshot, setSnapshot] = useState<ProcessSnapshot | null>(desktop ? null : demoSnapshot)
  const [apps, setApps] = useState<InstalledApp[]>(desktop ? [] : demoApps)
  const [bundleId, setBundleId] = useState(desktop ? '' : demoApps[0]?.bundleId ?? '')
  const [query, setQuery] = useState('')
  const [loading, setLoading] = useState(desktop)
  const [refreshing, setRefreshing] = useState(false)
  const [launching, setLaunching] = useState(false)
  const [stoppingPid, setStoppingPid] = useState<number | null>(null)
  const [loadError, setLoadError] = useState('')
  const mountedRef = useRef(true)
  const requestRunningRef = useRef(false)

  useEffect(() => {
    mountedRef.current = true
    return () => { mountedRef.current = false }
  }, [])

  const loadProcesses = useCallback(async (reportError = true) => {
    if (!desktop || requestRunningRef.current) return
    requestRunningRef.current = true
    setRefreshing(true)
    try {
      const next = await api.processesList(udid)
      if (!mountedRef.current) return
      setSnapshot(next)
      setLoadError('')
    } catch (error) {
      if (!mountedRef.current) return
      const message = errorMessage(error)
      setLoadError(message)
      if (reportError) onToast(message)
    } finally {
      requestRunningRef.current = false
      if (mountedRef.current) {
        setLoading(false)
        setRefreshing(false)
      }
    }
  }, [desktop, onToast, udid])

  useEffect(() => {
    if (!desktop) return
    let disposed = false
    const loadLaunchableApps = async () => {
      const [userApps, debuggableApps] = await Promise.allSettled([
        api.appsList(udid),
        api.appsDebuggable?.(udid) ?? Promise.resolve([]),
      ])
      if (userApps.status === 'rejected' && debuggableApps.status === 'rejected') {
        throw userApps.reason
      }
      const loaded = [
        ...(userApps.status === 'fulfilled' ? userApps.value.filter((app) => !app.system) : []),
        ...(debuggableApps.status === 'fulfilled' ? debuggableApps.value : []),
      ]
      const seen = new Set<string>()
      return loaded.filter((app) => {
        if (seen.has(app.bundleId)) return false
        seen.add(app.bundleId)
        return true
      })
    }
    void loadLaunchableApps()
      .then((launchable) => {
        if (disposed) return
        setApps(launchable)
        setBundleId((current) => launchable.some((app) => app.bundleId === current) ? current : launchable[0]?.bundleId ?? '')
      })
      .catch((error) => { if (!disposed) onToast(errorMessage(error)) })
    return () => { disposed = true }
  }, [desktop, onToast, udid])

  useEffect(() => {
    if (!desktop) return
    void loadProcesses()
    const timer = window.setInterval(() => { void loadProcesses(false) }, 5000)
    return () => window.clearInterval(timer)
  }, [desktop, loadProcesses])

  const shown = useMemo(() => {
    const needle = query.trim().toLowerCase()
    if (!needle) return snapshot?.processes ?? []
    return (snapshot?.processes ?? []).filter((process) =>
      `${process.name} ${process.pid} ${process.executablePath ?? ''}`.toLowerCase().includes(needle),
    )
  }, [query, snapshot])

  const launch = async () => {
    if (!bundleId || !snapshot?.supportsLaunch || launching) return
    if (!desktop) {
      const app = demoApps.find((item) => item.bundleId === bundleId)
      const pid = Math.max(...(snapshot.processes.map((process) => process.pid)), 1000) + 1
      setSnapshot({
        ...snapshot,
        processes: [{ pid, name: app?.name ?? bundleId, executablePath: `/private/var/${bundleId}.app/App`, isApplication: true, canStop: true, identity: `demo:${pid}:${bundleId}` }, ...snapshot.processes],
      })
      onToast(`${app?.name ?? bundleId} launched as pid ${pid}`)
      return
    }
    setLaunching(true)
    try {
      const result = await api.processLaunch(bundleId, udid)
      onToast(`${result.bundleId} launched as pid ${result.pid}`)
      await loadProcesses(false)
    } catch (error) {
      onToast(errorMessage(error))
    } finally {
      if (mountedRef.current) setLaunching(false)
    }
  }

  const stop = async (process: ProcessSummary) => {
    const confirmed = await dialogs.confirmDestructive(
      `Stop “${process.name}” (pid ${process.pid})? Unsaved work in this application may be lost.`,
      'Stop Process',
    )
    if (!confirmed) return
    if (!desktop) {
      setSnapshot((current) => current ? { ...current, processes: current.processes.filter((item) => item.identity !== process.identity) } : current)
      onToast(`${process.name} stopped`)
      return
    }
    setStoppingPid(process.pid)
    try {
      await api.processStop(process.pid, process.identity, udid)
      onToast(`${process.name} stopped`)
      await loadProcesses(false)
    } catch (error) {
      onToast(errorMessage(error))
    } finally {
      if (mountedRef.current) setStoppingPid(null)
    }
  }

  if (loading && !snapshot) {
    return <section className="processes-page"><div className="process-state card">Reading running processes…</div></section>
  }

  if (snapshot && !snapshot.available) {
    return (
      <section className="processes-page">
        <div className="process-state card"><b>Processes unavailable on this device</b><p>{snapshot.limitation}</p><code>{snapshot.transport}</code></div>
      </section>
    )
  }

  return (
    <section className="processes-page">
      <div className="process-toolbar">
        <label><Search size={15} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search name, path, or pid…" /></label>
        <button onClick={() => void loadProcesses()} disabled={!desktop || refreshing} title="Refresh process list"><RefreshCw size={14} className={refreshing ? 'spinning' : ''} />Refresh</button>
        <small>{snapshot?.transport ?? 'Unavailable'}</small>
      </div>
      <div className="process-launch card">
        <div><b>Launch application</b><small>Starts an installed user application without replacing an existing process.</small></div>
        <select value={bundleId} onChange={(event) => setBundleId(event.target.value)} disabled={!apps.length || !snapshot?.supportsLaunch} aria-label="Application to launch">
          {apps.map((app) => <option key={app.bundleId} value={app.bundleId}>{app.name} · {app.bundleId}</option>)}
        </select>
        <button className="primary-button" onClick={() => void launch()} disabled={!bundleId || !snapshot?.supportsLaunch || launching}><Play size={14} />{launching ? 'Launching…' : 'Launch'}</button>
      </div>
      {snapshot?.limitation && <div className="process-notice" role="status">{snapshot.limitation}</div>}
      {loadError && <div className="process-error" role="alert">{loadError} <button onClick={() => void loadProcesses()}>Retry</button></div>}
      <div className="process-table card">
        <header><span>Process</span><span>PID</span><span>Kind</span><span>Action</span></header>
        <div>
          {shown.map((process) => (
            <div className="process-row" key={process.identity}>
              <span><b>{process.name}</b><small>{process.executablePath ?? 'Executable path unavailable'}</small></span>
              <code>{process.pid}</code>
              <small>{process.canStop ? 'User application' : process.isApplication ? 'System application' : 'System process'}</small>
              {process.canStop && snapshot?.supportsStop
                ? <button className="process-stop" onClick={() => void stop(process)} disabled={stoppingPid !== null}><Square size={12} />{stoppingPid === process.pid ? 'Stopping…' : 'Stop'}</button>
                : <span className="process-protected">{process.canStop ? 'Read only' : 'Protected'}</span>}
            </div>
          ))}
          {!shown.length && <div className="process-empty">{query ? 'No process matches this search.' : 'The device returned no running processes.'}</div>}
        </div>
        <footer><span>{shown.length} / {snapshot?.processes.length ?? 0} processes</span><span>Automatically refreshes every 5 seconds</span></footer>
      </div>
    </section>
  )
}
