import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import {
  Activity, AppWindow, BadgeCheck, Beaker, Bug, Check, ChevronDown, CircleStop, ClipboardPaste, Code2, FolderOpen,
  KeyRound, MapPin, Plus, ScreenShare, Smartphone, TerminalSquare,
} from 'lucide-react'
import { devices, type Device } from './data'
import { api, errorMessage, events, isDesktopRuntime, type DeviceSummary } from './api'
import type { Connection, Page } from './types'
import { summaryToDevice } from './lib/device'
import { TitleBar } from './components/TitleBar'
import { Onboarding } from './components/Onboarding'
import { PairModal } from './components/PairModal'
import { Overview } from './pages/Overview'
import { Diagnostics } from './pages/Diagnostics'
import { Files } from './pages/Files'
import { Apps } from './pages/Apps'
import { CrashReports } from './pages/CrashReports'
import { Monitor } from './pages/Monitor'
import { Developer } from './pages/Developer'
import { Location } from './pages/Location'
import { LiveScreen } from './pages/LiveScreen'
import { Profiles } from './pages/Profiles'
import { PersonalSigning } from './pages/PersonalSigning'
import { Pasteboard } from './pages/Pasteboard'
import { TestLab } from './pages/TestLab'

const pageMeta: Record<Page, [string, string]> = {
  overview: ['Overview', 'idevice · lockdown query'],
  diagnostics: ['Diagnostics', 'com.apple.mobile.diagnostics_relay'],
  files: ['Files', 'com.apple.afc'],
  apps: ['Apps', 'com.apple.mobile.installation_proxy'],
  crashes: ['Crash Reports', 'com.apple.crashreportcopymobile'],
  logs: ['Monitor', 'Processes, performance, network capture, and live device logs'],
  screen: ['Live Screen', 'Live PNG device preview and still-frame capture'],
  developer: ['Debug Tools', 'com.apple.dt.* services'],
  profiles: ['Provisioning Profiles', 'Read-only Misagent signing and expiry inspection'],
  signing: ['Personal Signing Assistant', 'Apple Account or local Keychain identity · signed IPA export'],
  pasteboard: ['Pasteboard', 'Explicit bounded CoreDevice text and image transfer'],
  xctest: ['Test Lab', 'Read-only XCTest runner and developer-service preflight'],
  location: ['Location', 'com.apple.dt.simulatelocation'],
}

const navItems = [
  { id: 'overview', label: 'Overview', icon: AppWindow },
  { id: 'diagnostics', label: 'Diagnostics', icon: Activity },
  { id: 'files', label: 'Files', icon: FolderOpen, suffix: 'AFC' },
  { id: 'apps', label: 'Apps', icon: AppWindow },
  { id: 'crashes', label: 'Crash Reports', icon: Bug },
  { id: 'logs', label: 'Monitor', icon: TerminalSquare },
  { id: 'screen', label: 'Live Screen', icon: ScreenShare },
] as const

function App() {
  const desktop = useMemo(isDesktopRuntime, [])
  const [page, setPage] = useState<Page>('overview')
  const [deviceCatalog, setDeviceCatalog] = useState<Device[]>(desktop ? [] : devices)
  const [deviceId, setDeviceId] = useState(desktop ? '' : 'd1')
  const deviceIdRef = useRef(deviceId)
  const [connection, setConnection] = useState<Connection>(desktop ? 'none' : 'connected')
  const [deviceMenu, setDeviceMenu] = useState(false)
  const [pairOpen, setPairOpen] = useState(false)
  const [toast, setToast] = useState('')
  const mountedRef = useRef(true)
  const lifecycleRef = useRef(0)
  const refreshRunningRef = useRef(false)
  const refreshPendingRef = useRef(false)
  const device = deviceCatalog.find((item) => item.id === deviceId) ?? deviceCatalog[0] ?? devices[0]
  const connected = connection === 'connected'
  useEffect(() => { deviceIdRef.current = deviceId }, [deviceId])
  useEffect(() => {
    mountedRef.current = true
    lifecycleRef.current += 1
    return () => {
      mountedRef.current = false
      lifecycleRef.current += 1
    }
  }, [])

  const refreshDevices = useCallback(async () => {
    if (!desktop || !mountedRef.current) return
    refreshPendingRef.current = true
    if (refreshRunningRef.current) return

    refreshRunningRef.current = true
    const lifecycle = lifecycleRef.current
    const lifecycleIsCurrent = () => mountedRef.current && lifecycleRef.current === lifecycle
    try {
      while (refreshPendingRef.current && lifecycleIsCurrent()) {
        refreshPendingRef.current = false
        try {
          const found = await api.deviceList()
          if (!lifecycleIsCurrent()) return
          // A device event arrived while this snapshot was loading. Do not let
          // the older catalog take over the session; fetch the latest one.
          if (refreshPendingRef.current) continue

          const catalog = found.map(summaryToDevice)
          setDeviceCatalog(catalog)
          if (!found.length) {
            setDeviceId('')
            setConnection('none')
            setPage('overview')
            await api.deviceDisconnect().catch((error) => {
              if (lifecycleIsCurrent()) setToast(errorMessage(error))
            })
            continue
          }

          const current = found.find((item) => item.id === deviceIdRef.current)
          const target = current ?? found.find((item) => item.paired && item.connectable) ?? found[0]
          if (target.paired && target.connectable) {
            if (target.id !== deviceIdRef.current) await api.deviceSelect(target.id)
            if (!lifecycleIsCurrent()) return
            if (refreshPendingRef.current) continue
            setDeviceId(target.id)
            setConnection('connected')
          } else {
            setDeviceId(target.id)
            setConnection('detected')
            setPage('overview')
            await api.deviceDisconnect().catch((error) => {
              if (lifecycleIsCurrent()) setToast(errorMessage(error))
            })
          }
        } catch (error) {
          if (!lifecycleIsCurrent()) return
          if (refreshPendingRef.current) continue
          setConnection('none')
          setToast(errorMessage(error))
        }
      }
    } finally {
      refreshRunningRef.current = false
      // StrictMode can remount while the first mount still has a listing in
      // flight. The remounted effect marks a refresh pending; start it after
      // the obsolete runner releases the serialization lock.
      if (refreshPendingRef.current && mountedRef.current) void refreshDevices()
    }
  }, [desktop])

  useEffect(() => {
    if (!desktop) return
    let disposed = false
    let unlisten: (() => void) | undefined
    void events.deviceChanged(() => { if (!disposed) void refreshDevices() })
      .then((stop) => {
        if (disposed) stop()
        else unlisten = stop
      })
      .catch((error) => { if (!disposed) setToast(errorMessage(error)) })
    void api.deviceMonitorStart()
      .then(() => { if (!disposed) return refreshDevices() })
      .catch((error) => { if (!disposed) setToast(errorMessage(error)) })
    return () => {
      disposed = true
      unlisten?.()
      void api.deviceMonitorStop()
    }
  }, [desktop, refreshDevices])

  // Mount the developer disk image as soon as a device is usable, so no page has
  // to offer a mount control. This runs in the background: pages stay
  // interactive while it works, and only a failure is reported. A stale
  // selection drops its result rather than reporting against the new device.
  useEffect(() => {
    if (!desktop || !connected || !device?.udid) return
    let disposed = false
    void api.ddiEnsure(device.udid).catch((error) => {
      if (!disposed) setToast(errorMessage(error))
    })
    return () => { disposed = true }
  }, [desktop, connected, device?.udid])

  useEffect(() => {
    if (!toast) return
    const timer = window.setTimeout(() => setToast(''), 2500)
    return () => window.clearTimeout(timer)
  }, [toast])

  const selectDevice = async (id: string) => {
    setDeviceId(id)
    setDeviceMenu(false)
    if (!desktop) {
      setConnection('connected')
      return
    }
    try {
      await api.deviceSelect(id)
      setConnection('connected')
    } catch (error) {
      setConnection('detected')
      setToast(errorMessage(error))
    }
  }

  const disconnect = async () => {
    if (desktop) await api.deviceDisconnect().catch((error) => setToast(errorMessage(error)))
    setConnection('none')
    setDeviceMenu(false)
    setPage('overview')
  }

  const finishPairing = async () => {
    try {
      if (desktop) {
        const paired = await api.devicePair(device.udid)
        await api.deviceSelect(paired.udid)
        await refreshDevices()
      }
      setConnection('connected')
      setPairOpen(false)
      setToast(`${device.name} paired`)
    } catch (error) {
      setToast(errorMessage(error))
    }
  }

  return (
    <div className="desktop theme-clean mode-dark device-lab">
      <div className="window-shell">
        <TitleBar device={device} connection={connection} />
        <div className="window-body">
          <aside className="sidebar">
            <div className="device-select-wrap">
              <button className="device-card" onClick={() => setDeviceMenu((value) => !value)} aria-expanded={deviceMenu} aria-label="Select device" title={connected ? device.name : 'Select device'}>
                <span className={`device-icon status-${connection}`}><Smartphone size={19} /><i /></span>
                <span className="device-card-copy">
                  <b>{connected ? device.name : connection === 'detected' ? device.model : 'No device'}</b>
                  <small>{connected ? device.model : connection === 'detected' ? device.connectable === false ? 'Network only · connect USB once' : 'Awaiting trust…' : 'Connect to begin'}</small>
                </span>
                <ChevronDown size={14} />
              </button>
              {deviceMenu && (
                <div className="device-menu">
                  {deviceCatalog.map((item) => (
                    <button key={item.id} onClick={() => void selectDevice(item.id)}>
                      <i className={item.id === deviceId ? 'selected-dot' : ''} />
                      <span><b>{item.name}</b><small>{item.conn}{item.ios !== '—' ? ` · iOS ${item.ios}` : ''}</small></span>
                    </button>
                  ))}
                  <hr />
                  <button className="accent-action" onClick={() => { setPairOpen(true); setDeviceMenu(false) }}><Plus size={15} />Pair new device…</button>
                  <button className="danger-action" onClick={() => void disconnect()}><CircleStop size={15} />Disconnect device</button>
                </div>
              )}
            </div>

            <nav className={connected ? '' : 'nav-disabled'}>
              <span className="nav-heading">Device</span>
              {navItems.map(({ id, label, icon: Icon, ...item }) => (
                <button key={id} className={page === id ? 'active' : ''} onClick={() => setPage(id)} aria-label={label} title={label}>
                  <Icon size={17} /><span>{label}</span>{'suffix' in item && <small>{item.suffix}</small>}
                </button>
              ))}
              <span className="nav-heading developer-heading">Developer</span>
              <button className={page === 'developer' ? 'active' : ''} onClick={() => setPage('developer')} aria-label="Debug Tools" title="Debug Tools"><Code2 size={17} /><span>Debug Tools</span></button>
              <button className={page === 'profiles' ? 'active' : ''} onClick={() => setPage('profiles')} aria-label="Provisioning Profiles" title="Provisioning Profiles"><BadgeCheck size={17} /><span>Profiles</span></button>
              <button className={page === 'signing' ? 'active' : ''} onClick={() => setPage('signing')} aria-label="Personal Sign" title="Personal Sign"><KeyRound size={17} /><span>Personal Sign</span></button>
              <button className={page === 'pasteboard' ? 'active' : ''} onClick={() => setPage('pasteboard')} aria-label="Pasteboard" title="Pasteboard"><ClipboardPaste size={17} /><span>Pasteboard</span></button>
              <button className={page === 'xctest' ? 'active' : ''} onClick={() => setPage('xctest')} aria-label="Test Lab" title="Test Lab"><Beaker size={17} /><span>Test Lab</span></button>
              <button className={page === 'location' ? 'active' : ''} onClick={() => setPage('location')} aria-label="Location" title="Location"><MapPin size={17} /><span>Location</span></button>
            </nav>

            <div className="sidebar-footer">
              <i />
              <span><b>Trusted & Paired</b><small>{device.udid.slice(0, 8)}…{device.udid.slice(-6)}</small></span>
            </div>
          </aside>

          <main className="main-panel">
            {page !== 'overview' && (
              <header className="page-header">
                <div><h1>{pageMeta[page][0]}</h1><p>{pageMeta[page][1]}</p></div>
                <div className="header-spacer" />
                <div className="header-device-state">
                  <span className="header-state-dot" />
                  <span>{connected ? device.conn : 'Offline'}</span>
                  <i />
                  <span>{connected ? `iOS ${device.ios}` : 'No session'}</span>
                </div>
              </header>
            )}

            <div className="page-scroll" key={`${page}:${device.udid}`}>
              {connected && <>
                {page === 'overview' && <Overview device={device} desktop={desktop} onError={setToast} />}
                {page === 'diagnostics' && <Diagnostics device={device} desktop={desktop} onError={setToast} />}
                {page === 'files' && <Files desktop={desktop} udid={device.udid} onToast={setToast} />}
                {page === 'apps' && <Apps desktop={desktop} udid={device.udid} onToast={setToast} />}
                {page === 'crashes' && <CrashReports desktop={desktop} udid={device.udid} onToast={setToast} />}
                {page === 'logs' && <Monitor connected={connected} desktop={desktop} udid={device.udid} onError={setToast} />}
                {page === 'screen' && <LiveScreen desktop={desktop} udid={device.udid} onToast={setToast} />}
                {page === 'developer' && <Developer desktop={desktop} device={device} onToast={setToast} />}
                {page === 'profiles' && <Profiles desktop={desktop} udid={device.udid} onToast={setToast} />}
                {page === 'signing' && <PersonalSigning desktop={desktop} udid={device.udid} deviceName={device.name} onToast={setToast} />}
                {page === 'pasteboard' && <Pasteboard desktop={desktop} udid={device.udid} deviceName={device.name} onToast={setToast} />}
                {page === 'xctest' && <TestLab desktop={desktop} udid={device.udid} onToast={setToast} />}
                {page === 'location' && <Location desktop={desktop} udid={device.udid} onToast={setToast} />}
              </>}
            </div>

            {!connected && (
              <Onboarding
                state={connection}
                device={device}
                desktop={desktop}
                onDetect={() => desktop ? void refreshDevices() : setConnection('detected')}
                onPair={() => void finishPairing()}
                onCancel={() => setConnection('none')}
              />
            )}
          </main>
        </div>
      </div>
      {pairOpen && <PairModal onClose={() => setPairOpen(false)} onPair={() => void finishPairing()} />}
      {toast && <div className="toast"><span><Check size={12} /></span>{toast}</div>}
    </div>
  )
}

export default App
