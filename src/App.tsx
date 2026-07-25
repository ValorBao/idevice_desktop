import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import {
  Activity, AppWindow, Bug, Check, ChevronDown, CircleStop, Code2, FolderOpen,
  MapPin, Moon, Plus, Smartphone, Sun, TerminalSquare,
} from 'lucide-react'
import { devices, type Device } from './data'
import { api, errorMessage, events, isDesktopRuntime, type DeviceSummary } from './api'
import type { Appearance, Connection, Page, UiStyle } from './types'
import { summaryToDevice } from './lib/device'
import { TitleBar } from './components/TitleBar'
import { Onboarding } from './components/Onboarding'
import { PairModal } from './components/PairModal'
import { Overview } from './pages/Overview'
import { Diagnostics } from './pages/Diagnostics'
import { Files } from './pages/Files'
import { Apps } from './pages/Apps'
import { CrashReports } from './pages/CrashReports'
import { Logs } from './pages/Logs'
import { Developer } from './pages/Developer'
import { Location } from './pages/Location'

const pageMeta: Record<Page, [string, string]> = {
  overview: ['Overview', 'idevice · lockdown query'],
  diagnostics: ['Diagnostics', 'com.apple.mobile.diagnostics_relay'],
  files: ['Files', 'com.apple.afc'],
  apps: ['Apps', 'com.apple.mobile.installation_proxy'],
  crashes: ['Crash Reports', 'com.apple.crashreportcopymobile'],
  logs: ['Logs', 'com.apple.syslog_relay'],
  developer: ['Debug Tools', 'com.apple.dt.* services'],
  location: ['Location', 'com.apple.dt.simulatelocation'],
}

const navItems = [
  { id: 'overview', label: 'Overview', icon: AppWindow },
  { id: 'diagnostics', label: 'Diagnostics', icon: Activity },
  { id: 'files', label: 'Files', icon: FolderOpen, suffix: 'AFC' },
  { id: 'apps', label: 'Apps', icon: AppWindow },
  { id: 'crashes', label: 'Crash Reports', icon: Bug },
  { id: 'logs', label: 'Logs', icon: TerminalSquare },
] as const

function App() {
  const desktop = useMemo(isDesktopRuntime, [])
  const [page, setPage] = useState<Page>('overview')
  const [uiStyle, setUiStyle] = useState<UiStyle>('clean')
  const [appearance, setAppearance] = useState<Appearance>('dark')
  const [deviceCatalog, setDeviceCatalog] = useState<Device[]>(desktop ? [] : devices)
  const [deviceId, setDeviceId] = useState(desktop ? '' : 'd1')
  const deviceIdRef = useRef(deviceId)
  const [connection, setConnection] = useState<Connection>(desktop ? 'none' : 'connected')
  const [deviceMenu, setDeviceMenu] = useState(false)
  const [pairOpen, setPairOpen] = useState(false)
  const [toast, setToast] = useState('')
  const device = deviceCatalog.find((item) => item.id === deviceId) ?? deviceCatalog[0] ?? devices[0]
  const connected = connection === 'connected'
  useEffect(() => { deviceIdRef.current = deviceId }, [deviceId])

  const refreshDevices = useCallback(async () => {
    if (!desktop) return
    try {
      const found = await api.deviceList()
      const catalog = found.map(summaryToDevice)
      setDeviceCatalog(catalog)
      if (!found.length) {
        setDeviceId('')
        setConnection('none')
        return
      }
      const current = found.find((item) => item.id === deviceIdRef.current)
      const target = current ?? found.find((item) => item.paired && item.connectable) ?? found[0]
      setDeviceId(target.id)
      if (target.paired && target.connectable) {
        if (target.id !== deviceIdRef.current) await api.deviceSelect(target.id)
        setConnection('connected')
      } else {
        setConnection('detected')
      }
    } catch (error) {
      setConnection('none')
      setToast(errorMessage(error))
    }
  }, [desktop])

  useEffect(() => {
    if (!desktop) return
    let disposed = false
    let unlisten: (() => void) | undefined
    events.deviceChanged(() => { if (!disposed) void refreshDevices() }).then((stop) => { unlisten = stop })
    void api.deviceMonitorStart().then(refreshDevices).catch((error) => setToast(errorMessage(error)))
    return () => {
      disposed = true
      unlisten?.()
      void api.deviceMonitorStop()
    }
  }, [desktop, refreshDevices])

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
    <div className={`desktop theme-${uiStyle} mode-${appearance}`}>
      <div className="window-shell">
        <TitleBar device={device} connection={connection} />
        <div className="window-body">
          <aside className="sidebar">
            <div className="device-select-wrap">
              <button className="device-card" onClick={() => setDeviceMenu((value) => !value)} aria-expanded={deviceMenu}>
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
                <button key={id} className={page === id ? 'active' : ''} onClick={() => setPage(id)}>
                  <Icon size={17} /><span>{label}</span>{'suffix' in item && <small>{item.suffix}</small>}
                </button>
              ))}
              <span className="nav-heading developer-heading">Developer</span>
              <button className={page === 'developer' ? 'active' : ''} onClick={() => setPage('developer')}><Code2 size={17} /><span>Debug Tools</span></button>
              <button className={page === 'location' ? 'active' : ''} onClick={() => setPage('location')}><MapPin size={17} /><span>Location</span></button>
            </nav>

            <div className="sidebar-footer">
              <i />
              <span><b>Trusted & Paired</b><small>{device.udid.slice(0, 8)}…{device.udid.slice(-6)}</small></span>
            </div>
          </aside>

          <main className="main-panel">
            <header className="page-header">
              <div><h1>{pageMeta[page][0]}</h1><p>{pageMeta[page][1]}</p></div>
              <div className="header-spacer" />
              <div className="style-switcher">
                {(['clean', 'terminal', 'apple'] as UiStyle[]).map((style) => (
                  <button key={style} className={uiStyle === style ? 'active' : ''} onClick={() => setUiStyle(style)}>{style[0].toUpperCase() + style.slice(1)}</button>
                ))}
              </div>
              <button className="appearance-button" onClick={() => setAppearance((mode) => mode === 'dark' ? 'light' : 'dark')} aria-label="Toggle appearance">
                {appearance === 'dark' ? <Moon size={17} /> : <Sun size={17} />}
              </button>
            </header>

            <div className="page-scroll">
              {page === 'overview' && <Overview device={device} desktop={desktop} onNavigate={setPage} onError={setToast} />}
              {page === 'diagnostics' && <Diagnostics device={device} desktop={desktop} onError={setToast} />}
              {page === 'files' && <Files desktop={desktop} udid={device.udid} onToast={setToast} />}
              {page === 'apps' && <Apps desktop={desktop} udid={device.udid} onToast={setToast} />}
              {page === 'crashes' && <CrashReports desktop={desktop} udid={device.udid} onToast={setToast} />}
              {page === 'logs' && <Logs connected={connected} desktop={desktop} udid={device.udid} onError={setToast} />}
              {page === 'developer' && <Developer desktop={desktop} device={device} onToast={setToast} />}
              {page === 'location' && <Location desktop={desktop} udid={device.udid} onToast={setToast} />}
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
