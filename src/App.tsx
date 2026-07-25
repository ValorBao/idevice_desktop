import { useCallback, useEffect, useMemo, useRef, useState, type DragEvent, type MouseEvent } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import L, { type Map as LeafletMap, type Marker as LeafletMarker } from 'leaflet'
import 'leaflet/dist/leaflet.css'
import {
  Activity, AppWindow, BatteryCharging, Check, ChevronDown, ChevronRight, CircleStop,
  Bug, Code2, Download, File, Folder, FolderOpen, HardDrive, MapPin, Moon, Pause, Play, Plus,
  RefreshCw, Search, Smartphone, Sun, TerminalSquare, Upload, Usb, Wifi, X,
} from 'lucide-react'
import {
  devices, fileSystem, gestalt, initialLogs, installedApps, ioTree, liveLogPool, presets,
  type AppInfo, type Device, type LogLine,
} from './data'
import {
  api, dialogs, errorMessage, events, isDesktopRuntime,
  type CrashReportContent, type CrashReportSummary, type DeveloperStatus, type DeviceOverview, type DeviceSummary, type FileSharingApp, type InstalledApp,
  type RemoteFileEntry,
} from './api'

type Page = 'overview' | 'diagnostics' | 'files' | 'apps' | 'crashes' | 'logs' | 'developer' | 'location'
type UiStyle = 'clean' | 'terminal' | 'apple'
type Appearance = 'dark' | 'light'
type Connection = 'connected' | 'detected' | 'none'

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

const summaryToDevice = (summary: DeviceSummary): Device => ({
  id: summary.id,
  name: summary.name ?? (summary.connectable ? 'Unpaired device' : 'Network iPhone'),
  model: summary.model ?? (summary.connectable ? 'iPhone / iPad' : 'Bonjour device'),
  modelId: summary.model ?? 'Unknown',
  ios: summary.ios ?? '—',
  build: '—',
  udid: summary.udid,
  serial: '—',
  ecid: '—',
  chip: 'Apple Silicon',
  wifi: '—',
  conn: summary.connection,
  battery: 0,
  batteryHealth: 0,
  cycles: 0,
  storageUsed: 0,
  storageTotal: 0,
  transports: summary.transports,
  connectable: summary.connectable,
})

const bytes = (value: number | null | undefined) => {
  if (!value) return '—'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const power = Math.min(units.length - 1, Math.floor(Math.log(value) / Math.log(1024)))
  return `${(value / 1024 ** power).toFixed(power > 2 ? 1 : 0)} ${units[power]}`
}

const displaySizeToBytes = (value?: string) => {
  if (!value) return 0
  const match = value.match(/^([\d.]+)\s*(KB|MB|GB|TB|B)$/i)
  if (!match) return 0
  const power = ['B', 'KB', 'MB', 'GB', 'TB'].indexOf(match[2].toUpperCase())
  return Number(match[1]) * 1024 ** Math.max(0, power)
}

const flatten = (value: unknown, prefix = ''): Array<[string, string]> => {
  if (value === null || value === undefined) return [[prefix || 'value', 'null']]
  if (typeof value !== 'object') return [[prefix || 'value', String(value)]]
  if (Array.isArray(value)) return value.flatMap((item, index) => flatten(item, `${prefix}[${index}]`))
  return Object.entries(value as Record<string, unknown>).flatMap(([key, item]) => flatten(item, prefix ? `${prefix}.${key}` : key))
}

const appColor = (bundleId: string) => {
  let hash = 0
  for (const character of bundleId) hash = (hash * 31 + character.charCodeAt(0)) | 0
  return `hsl(${Math.abs(hash) % 360} 68% 55%)`
}

const deviceScreenCache = new Map<string, string>()

const installedToApp = (app: InstalledApp): AppInfo => ({
  id: app.bundleId,
  name: app.name,
  bundle: app.bundleId,
  version: app.version || '—',
  size: bytes(app.sizeBytes),
  color: appColor(app.bundleId),
  icon: app.iconDataUrl ?? undefined,
  system: app.system,
})

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

function TitleBar({ device, connection }: { device: Device; connection: Connection }) {
  const status = connection === 'connected' ? 'lockdownd · usbmuxd' : connection === 'detected' ? 'device detected · awaiting trust' : 'no device · scanning usbmuxd'
  const startWindowDrag = (event: MouseEvent<HTMLDivElement>) => {
    if (!isDesktopRuntime() || event.button !== 0) return
    const target = event.target
    if (target instanceof Element && target.closest('button')) return
    event.preventDefault()
    void getCurrentWindow().startDragging().catch((error) => console.error('Unable to drag window', error))
  }
  const controlWindow = (action: 'close' | 'minimize' | 'fullscreen') => {
    if (!isDesktopRuntime()) return
    const appWindow = getCurrentWindow()
    const operation = action === 'close'
      ? appWindow.close()
      : action === 'minimize'
        ? appWindow.minimize()
        : appWindow.isFullscreen().then((fullscreen) => appWindow.setFullscreen(!fullscreen))
    void operation.catch((error) => console.error(`Unable to ${action} window`, error))
  }
  return (
    <div className="titlebar" data-tauri-drag-region onMouseDown={startWindowDrag}>
      <div className="traffic-lights">
        <button type="button" aria-label="Close window" title="Close" onClick={() => controlWindow('close')}><i /></button>
        <button type="button" aria-label="Minimize window" title="Minimize" onClick={() => controlWindow('minimize')}><i /></button>
        <button type="button" aria-label="Enter or exit fullscreen" title="Fullscreen" onClick={() => controlWindow('fullscreen')}><i /></button>
      </div>
      <div className="app-title" data-tauri-drag-region><span />idevice <small>— {connection === 'connected' ? device.name : connection === 'detected' ? `${device.model} (untrusted)` : 'no device'}</small></div>
      <div className="titlebar-spacer" data-tauri-drag-region />
      <div className={`connection-status status-${connection}`} data-tauri-drag-region><i />{status}</div>
    </div>
  )
}

function Overview({ device, desktop, onNavigate, onError }: { device: Device; desktop: boolean; onNavigate: (page: Page) => void; onError: (message: string) => void }) {
  const [overview, setOverview] = useState<DeviceOverview | null>(null)
  const [screenImage, setScreenImage] = useState(() => deviceScreenCache.get(device.udid) ?? '')
  const [screenError, setScreenError] = useState('')
  const [screenLoading, setScreenLoading] = useState(false)
  const refreshScreen = useCallback(async (mountIfNeeded = false) => {
    if (!desktop) return
    setScreenLoading(true)
    setScreenError('')
    try {
      const image = await api.screenshot(device.udid)
      deviceScreenCache.set(device.udid, image)
      setScreenImage(image)
    } catch (error) {
      if (!mountIfNeeded) {
        setScreenError(errorMessage(error))
      } else {
        try {
          await api.ddiMountAuto(device.udid)
          const image = await api.screenshot(device.udid)
          deviceScreenCache.set(device.udid, image)
          setScreenImage(image)
        } catch (setupError) {
          setScreenError(errorMessage(setupError))
        }
      }
    } finally {
      setScreenLoading(false)
    }
  }, [desktop, device.udid])
  useEffect(() => {
    if (!desktop) return
    setOverview(null)
    void api.overview(device.udid).then(setOverview).catch((error) => onError(errorMessage(error)))
  }, [desktop, device.udid, onError])
  useEffect(() => {
    const cached = deviceScreenCache.get(device.udid)
    if (cached) {
      setScreenImage(cached)
      setScreenError('')
      return
    }
    setScreenImage('')
    void refreshScreen(true)
  }, [device.udid, refreshScreen])
  const storageTotal = overview?.storage?.totalBytes ?? device.storageTotal * 1024 ** 3
  const storageUsed = overview?.storage?.usedBytes ?? device.storageUsed * 1024 ** 3
  const storageFree = overview?.storage?.freeBytes ?? Math.max(0, storageTotal - storageUsed)
  const storagePercent = storageTotal ? Math.round(storageUsed / storageTotal * 100) : 0
  const battery = overview?.battery.level ?? device.battery
  const batteryHealth = overview?.battery.healthPercent ?? device.batteryHealth
  const cycles = overview?.battery.cycleCount ?? device.cycles
  const screenStatus = screenError
    ? screenError.toLowerCase().includes('locked')
      ? 'Unlock iPhone\nthen refresh'
      : 'DDI not mounted\ntap refresh'
    : screenLoading ? 'Loading…' : device.chip
  const identity = [
    ['UDID', device.udid], ['Serial number', overview?.serialNumber ?? device.serial], ['ECID', overview?.uniqueChipId ?? device.ecid],
    ['Hardware model', overview?.hardwareModel ?? device.modelId], ['Chip / platform', overview?.hardwarePlatform ?? device.chip], ['Wi-Fi address', overview?.wifiAddress ?? device.wifi],
  ]
  const actions: Array<[string, string, string, Page]> = [
    ['▤', 'Browse files', 'AFC filesystem', 'files'], ['◇', 'Manage apps', 'install · uninstall', 'apps'],
    ['≣', 'Stream logs', 'live syslog', 'logs'], ['◆', 'Debug tools', 'JIT · GDB · DDI', 'developer'],
  ]
  return (
    <section className="overview-page content-width">
      <div className="overview-grid">
        <div className="device-visual card">
          <div className={`phone-frame ${screenImage ? 'has-screen' : ''}`} title={screenError || 'Device screen'}>
            {screenImage
              ? <img src={screenImage} alt="Current device screen" />
              : <small className="phone-screen-status">{screenStatus}</small>}
            <span className="phone-notch" />
            {desktop && <button className={`phone-refresh ${screenLoading ? 'loading' : ''}`} type="button" title={screenError ? 'Mount DDI and retry' : 'Refresh device screen'} aria-label={screenError ? 'Mount DDI and retry' : 'Refresh device screen'} disabled={screenLoading} onClick={() => void refreshScreen(true)}><RefreshCw size={11} /></button>}
          </div>
          <div><b>{overview?.productType ?? device.model}</b><small>{overview?.hardwareModel ?? device.modelId}</small></div>
        </div>
        <div className="stats-grid">
          <StatCard label="iOS Version"><strong>{overview?.productVersion ?? device.ios}</strong><small>build {overview?.buildVersion ?? device.build}</small></StatCard>
          <StatCard label="Battery">
            <div className="battery-stat"><div className="battery-ring" style={{ '--battery': `${battery * 3.6}deg` } as React.CSSProperties}><span>{battery}</span></div><div><b>{batteryHealth ? `${batteryHealth}% health` : 'health unavailable'}</b><small>{cycles ? `${cycles} cycles` : 'cycle count unavailable'}</small></div></div>
          </StatCard>
          <StatCard label="Connection"><div className="connected-label"><i />{overview?.connection ?? device.conn}</div><small>lockdown session active</small></StatCard>
          <StatCard label="Storage" wide><div className="storage-label"><span /><small>{bytes(storageUsed)} / {bytes(storageTotal)}</small></div><div className="progress"><span style={{ width: `${storagePercent}%` }} /></div><small>{bytes(storageFree)} available</small></StatCard>
          <StatCard label="Pairing"><div className="trusted"><Check size={16} />Trusted</div><small>record valid</small></StatCard>
        </div>
      </div>
      <div className="identity-card card">
        {identity.map(([key, value]) => <div key={key}><span>{key}</span><code>{value}</code></div>)}
      </div>
      <h2 className="section-label">Quick actions</h2>
      <div className="quick-actions">
        {actions.map(([glyph, title, subtitle, target]) => <button key={target} onClick={() => onNavigate(target)}><code>{glyph}</code><b>{title}</b><small>{subtitle}</small></button>)}
      </div>
    </section>
  )
}

function StatCard({ label, wide, children }: { label: string; wide?: boolean; children: React.ReactNode }) {
  return <div className={`stat-card card ${wide ? 'wide' : ''}`}><span className="stat-label">{label}</span>{children}</div>
}

function Diagnostics({ device, desktop, onError }: { device: Device; desktop: boolean; onError: (message: string) => void }) {
  const [tab, setTab] = useState<'battery' | 'gestalt' | 'io' | 'nand' | 'wifi'>('battery')
  const [query, setQuery] = useState('')
  const [raw, setRaw] = useState<unknown>(null)
  const [loading, setLoading] = useState(false)
  useEffect(() => {
    if (!desktop) return
    setLoading(true)
    void api.diagnostic(tab, device.udid)
      .then(setRaw)
      .catch((error) => onError(errorMessage(error)))
      .finally(() => setLoading(false))
  }, [desktop, tab, device.udid, onError])
  const filteredGestalt = gestalt.filter(([key, value]) => `${key} ${value}`.toLowerCase().includes(query.toLowerCase()))
  const realRows = useMemo(() => flatten(raw).filter(([key, value]) => `${key} ${value}`.toLowerCase().includes(query.toLowerCase())), [raw, query])
  const batteryStats = [
    ['Health', `${device.batteryHealth}%`, 'Good'], ['Cycle count', `${device.cycles}`, 'within spec'],
    ['Temperature', '31.2°', 'nominal'], ['Voltage', '4.18V', 'charging'],
  ]
  const chart = [72, 76, 80, 82, 78, 72, 69, 72, 78, 84, 88, 87, 80, 74, 72, 76, 83, 89, 92, 88, 84, 86, 91, 95]
  return (
    <section className="diagnostics-page page-padding">
      <div className="tabs">
        {(['battery', 'gestalt', 'io', 'nand', 'wifi'] as const).map((item) => <button key={item} className={tab === item ? 'active' : ''} onClick={() => setTab(item)}>{item === 'gestalt' ? 'MobileGestalt' : item === 'io' ? 'IORegistry' : item === 'nand' ? 'NAND' : item === 'wifi' ? 'Wi-Fi' : 'Battery'}</button>)}
      </div>
      {!desktop && tab === 'battery' && <>
        <div className="diagnostic-stats">{batteryStats.map(([label, value, note]) => <div className="card" key={label}><span className="stat-label">{label}</span><strong>{value}</strong><small className={label === 'Health' ? 'good' : ''}>{note}</small></div>)}</div>
        <div className="chart-card card"><b>Capacity over last 30 charge cycles</b><div className="bar-chart">{chart.map((height, index) => <span key={index} style={{ height: `${height}%`, opacity: .45 + index * .022 }} />)}</div></div>
      </>}
      {!desktop && tab === 'gestalt' && <div className="table-card card"><div className="search-row"><Search size={15} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Filter keys…" /><small>{filteredGestalt.length} keys</small></div>{filteredGestalt.map(([key, value]) => <div className="key-value-row" key={key}><code>{key}</code><span>{value}</span></div>)}</div>}
      {!desktop && tab === 'io' && <div className="io-tree card">{ioTree.map(([prefix, name, meta], index) => <div key={`${name}-${index}`}><span>{prefix}</span><b>{name}</b><small>{meta && `  ${meta}`}</small></div>)}</div>}
      {!desktop && (tab === 'nand' || tab === 'wifi') && <div className="empty-card card">Connect the desktop app to query this diagnostic.</div>}
      {desktop && <div className="table-card card"><div className="search-row"><Search size={15} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Filter keys…" /><small>{loading ? 'querying…' : `${realRows.length} values`}</small></div>{!loading && realRows.map(([key, value]) => <div className="key-value-row" key={key}><code>{key}</code><span>{value}</span></div>)}</div>}
    </section>
  )
}

type FileSource = 'media' | 'app'

const protectedMediaRoots = new Set(['Books', 'DCIM', 'PhotoData', 'Purchases', 'iTunes_Control'])
const mediaFolderDescriptions: Record<string, string> = {
  Books: 'Synced books · iOS managed',
  DCIM: 'Photos and camera videos · iOS managed',
  Downloads: 'Device media downloads',
  PhotoData: 'Photos database and thumbnails · iOS managed',
  Purchases: 'Purchased media data · iOS managed',
  iTunes_Control: 'Synced music library data · iOS managed',
}
const demoAppDocumentEntries: RemoteFileEntry[] = [
  { name: 'Exports', path: '/Exports', kind: 'Folder', isDirectory: true, size: 0, modified: '2026-07-18 10:32:00' },
  { name: 'settings.json', path: '/settings.json', kind: 'JSON', isDirectory: false, size: 14682, modified: '2026-07-17 21:08:14' },
  { name: 'session.log', path: '/session.log', kind: 'LOG', isDirectory: false, size: 32768, modified: '2026-07-18 09:46:03' },
]

const mediaPathIsProtected = (path: string) => protectedMediaRoots.has(path.split('/').filter(Boolean)[0] ?? '')

function Files({ desktop, udid, onToast }: { desktop: boolean; udid: string; onToast: (message: string) => void }) {
  const demoSharingApps: FileSharingApp[] = installedApps.filter((app) => !app.system).slice(0, 3).map((app) => ({ bundleId: app.bundle, name: app.name }))
  const [source, setSource] = useState<FileSource>('media')
  const [path, setPath] = useState<string[]>([])
  const [remoteEntries, setRemoteEntries] = useState<RemoteFileEntry[]>([])
  const [remoteSharingApps, setRemoteSharingApps] = useState<FileSharingApp[]>([])
  const sharingApps = desktop ? remoteSharingApps : demoSharingApps
  const [selectedBundle, setSelectedBundle] = useState(desktop ? '' : demoSharingApps[0]?.bundleId ?? '')
  const [selected, setSelected] = useState<RemoteFileEntry | null>(null)
  const currentPath = `/${path.join('/')}`
  const selectedApp = sharingApps.find((app) => app.bundleId === selectedBundle)
  const bundleId = source === 'app' ? selectedBundle || undefined : undefined
  const sourceAvailable = source === 'media' || Boolean(bundleId)
  const currentReadOnly = source === 'media' && mediaPathIsProtected(currentPath)
  const selectedReadOnly = source === 'media' && Boolean(selected && mediaPathIsProtected(selected.path))

  useEffect(() => {
    if (!desktop) return
    void api.fileSharingApps(udid).then((apps) => {
      setRemoteSharingApps(apps)
      setSelectedBundle((current) => apps.some((app) => app.bundleId === current) ? current : apps[0]?.bundleId ?? '')
    }).catch((error) => onToast(errorMessage(error)))
  }, [desktop, udid, onToast])

  const refresh = useCallback(async () => {
    if (!desktop) return
    if (!sourceAvailable) {
      setRemoteEntries([])
      setSelected(null)
      return
    }
    try {
      setRemoteEntries(await api.afcList(currentPath, udid, bundleId))
      setSelected(null)
    } catch (error) {
      setRemoteEntries([])
      onToast(errorMessage(error))
    }
  }, [desktop, sourceAvailable, currentPath, udid, bundleId, onToast])
  useEffect(() => { void refresh() }, [refresh])

  const entries: RemoteFileEntry[] = desktop
    ? remoteEntries
    : source === 'app'
      ? currentPath === '/' ? demoAppDocumentEntries : []
      : (fileSystem[currentPath] ?? []).map((entry) => ({
        name: entry.name, path: `${currentPath === '/' ? '' : currentPath}/${entry.name}`, kind: entry.folder ? 'Folder' : entry.kind ?? 'Document',
        isDirectory: Boolean(entry.folder), size: displaySizeToBytes(entry.size), modified: entry.date,
      }))

  const selectSource = (nextSource: FileSource) => {
    setSource(nextSource)
    setPath([])
    setSelected(null)
  }
  const selectApp = (nextBundle: string) => {
    setSelectedBundle(nextBundle)
    setPath([])
    setSelected(null)
  }
  const remoteChild = (name: string) => `${currentPath === '/' ? '' : currentPath}/${name}`

  const uploadFile = async () => {
    if (!desktop) return onToast('File operations are available in the desktop app')
    if (!sourceAvailable || currentReadOnly) return
    try {
      const chosen = await dialogs.anyFile()
      if (!chosen || Array.isArray(chosen)) return
      const name = chosen.split(/[\\/]/).pop() ?? 'upload.bin'
      await api.afcUpload(chosen, remoteChild(name), udid, bundleId)
      onToast(`${name} uploaded`)
      await refresh()
    } catch (error) { onToast(errorMessage(error)) }
  }
  const downloadFile = async () => {
    if (!desktop || !selected || selected.isDirectory) return
    try {
      const target = await dialogs.saveFile(selected.name)
      if (!target) return
      await api.afcDownload(selected.path, target, udid, bundleId)
      onToast(`${selected.name} downloaded`)
    } catch (error) { onToast(errorMessage(error)) }
  }
  const makeDirectory = async () => {
    if (!desktop) return onToast('Folder creation is available in the desktop app')
    if (!sourceAvailable || currentReadOnly) return
    const name = window.prompt('Folder name')?.trim()
    if (!name || name === '.' || name === '..' || /[\\/]/.test(name)) return
    try {
      await api.afcMkdir(remoteChild(name), udid, bundleId)
      onToast(`${name} created`)
      await refresh()
    } catch (error) { onToast(errorMessage(error)) }
  }
  const removeEntry = async () => {
    if (!desktop || !selected || selectedReadOnly || !window.confirm(`Delete ${selected.name}?`)) return
    try {
      await api.afcRemove(selected.path, selected.isDirectory, udid, bundleId)
      onToast(`${selected.name} removed`)
      await refresh()
    } catch (error) { onToast(errorMessage(error)) }
  }

  const rootLabel = source === 'media' ? 'iPhone Media' : 'Documents'
  const displayedPath = source === 'media'
    ? `/var/mobile/Media${currentPath === '/' ? '' : currentPath}`
    : `${selectedApp?.name ?? 'App'} sandbox/Documents${currentPath === '/' ? '' : currentPath}`

  return (
    <section className="files-page page-padding compact-padding">
      <div className="file-source-bar">
        <div className="file-source-tabs" role="tablist" aria-label="iPhone file source">
          <button role="tab" aria-selected={source === 'media'} className={source === 'media' ? 'active' : ''} onClick={() => selectSource('media')}><HardDrive size={15} />Device Media</button>
          <button role="tab" aria-selected={source === 'app'} className={source === 'app' ? 'active' : ''} onClick={() => selectSource('app')}><AppWindow size={15} />App Documents</button>
        </div>
        {source === 'app' && <label className="file-app-select"><span>File-sharing app</span><select value={selectedBundle} onChange={(event) => selectApp(event.target.value)} disabled={!sharingApps.length}>{sharingApps.length ? sharingApps.map((app) => <option key={app.bundleId} value={app.bundleId}>{app.name} · {app.bundleId}</option>) : <option value="">No compatible apps</option>}</select></label>}
        <div className="file-source-copy"><b>{source === 'media' ? 'AFC media storage' : 'Shared app documents'}</b><small>{source === 'media' ? 'Maps to /var/mobile/Media; this is not the iOS system root.' : 'Only apps that enable iOS File Sharing appear here.'}</small></div>
      </div>
      <div className="breadcrumbs"><span>{source === 'media' ? 'com.apple.afc' : 'house_arrest'}</span><ChevronRight size={13} /><button onClick={() => setPath([])}>{rootLabel}</button>{path.map((part, index) => <span className="crumb-pair" key={`${part}-${index}`}><ChevronRight size={13} /><button onClick={() => setPath(path.slice(0, index + 1))}>{part}</button></span>)}{currentReadOnly && <em className="read-only-badge">Read only</em>}</div>
      <div className="file-actions"><button className="primary-button" onClick={() => void uploadFile()} disabled={!sourceAvailable || currentReadOnly}><Upload size={14} />Upload</button><button onClick={() => void downloadFile()} disabled={!selected || selected.isDirectory}>Download</button><button onClick={() => void makeDirectory()} disabled={!sourceAvailable || currentReadOnly}><Plus size={14} />New folder</button><button className="danger-action" onClick={() => void removeEntry()} disabled={!selected || selectedReadOnly}>Delete</button></div>
      <div className="file-table card">
        <div className="file-head"><span>Name</span><span>Kind</span><span>Modified</span><span>Size</span></div>
        {!sourceAvailable ? <div className="empty-card">No installed apps currently expose Documents through iOS File Sharing.</div> : !entries.length ? <div className="empty-card">This folder is empty.</div> : entries.map((entry) => {
          const description = source === 'media' && currentPath === '/' ? mediaFolderDescriptions[entry.name] : ''
          const readOnlyEntry = source === 'media' && mediaPathIsProtected(entry.path)
          return <button key={entry.name} className={selected?.path === entry.path ? 'selected' : ''} onClick={() => setSelected(entry)} onDoubleClick={() => entry.isDirectory && setPath([...path, entry.name])}><span>{entry.isDirectory ? <Folder size={17} /> : <File size={17} />}<span className="file-name-copy"><b>{entry.name}</b>{description && <small>{description}</small>}</span></span><small>{readOnlyEntry ? `${entry.kind} · read only` : entry.kind}</small><small>{entry.modified}</small><small>{entry.isDirectory ? '—' : bytes(entry.size)}</small></button>
        })}
      </div>
      <div className="table-footer"><span>{entries.length} items</span><span>{displayedPath}</span></div>
    </section>
  )
}

function Apps({ desktop, udid, onToast }: { desktop: boolean; udid: string; onToast: (message: string) => void }) {
  const [query, setQuery] = useState('')
  const [apps, setApps] = useState<AppInfo[]>(desktop ? [] : installedApps)
  const [selectedId, setSelectedId] = useState(desktop ? '' : 'a1')
  const [dragging, setDragging] = useState(false)
  const [install, setInstall] = useState<{ name: string; progress: number; phase: string } | null>(null)
  const selected = apps.find((app) => app.id === selectedId) ?? apps[0]
  const filtered = apps.filter((app) => `${app.name} ${app.bundle}`.toLowerCase().includes(query.toLowerCase()))

  const loadApps = useCallback(async () => {
    if (!desktop) return
    try {
      const loaded = (await api.appsList(udid)).filter((app) => !app.system).map(installedToApp)
      setApps(loaded)
      setSelectedId((current) => loaded.some((app) => app.id === current) ? current : loaded[0]?.id ?? '')
    } catch (error) { onToast(errorMessage(error)) }
  }, [desktop, udid, onToast])

  useEffect(() => { void loadApps() }, [loadApps])
  useEffect(() => {
    if (!desktop) return
    let unlisten: (() => void) | undefined
    events.appProgress((progress) => setInstall({ name: progress.item, progress: progress.percent, phase: progress.operation === 'uninstall' ? 'Uninstalling' : 'Installing' })).then((stop) => { unlisten = stop })
    return () => unlisten?.()
  }, [desktop])

  const startInstall = async (fileName = 'Sideloaded.ipa', suppliedPath?: string) => {
    if (install) return
    if (desktop) {
      try {
        const chosen = suppliedPath ?? await dialogs.ipa()
        if (!chosen || Array.isArray(chosen)) return
        const name = chosen.split(/[\\/]/).pop() ?? 'Application.ipa'
        setInstall({ name, progress: 0, phase: 'Preparing package' })
        await api.appInstall(chosen, udid)
        setInstall(null)
        await loadApps()
        onToast(`${name} installed`)
      } catch (error) {
        setInstall(null)
        onToast(errorMessage(error))
      }
      return
    }
    const safeName = fileName.toLowerCase().endsWith('.ipa') ? fileName : `${fileName}.ipa`
    setInstall({ name: safeName, progress: 0, phase: 'Verifying signature' })
  }

  useEffect(() => {
    if (!install || desktop) return
    const timer = window.setInterval(() => {
      setInstall((current) => {
        if (!current) return null
        const next = Math.min(100, current.progress + 8)
        if (next >= 100) {
          window.clearInterval(timer)
          const name = current.name.replace(/\.ipa$/i, '')
          const app: AppInfo = { id: `x${Date.now()}`, name, bundle: `com.sideload.${name.toLowerCase().replace(/[^a-z0-9]/g, '')}`, version: '1.0.0', size: '18.6 MB', color: '#e0566f', fresh: true }
          setApps((items) => [app, ...items])
          setSelectedId(app.id)
          onToast(`${name} installed`)
          return null
        }
        return { ...current, progress: next, phase: next < 32 ? 'Verifying signature' : next < 68 ? 'Transferring package' : next < 94 ? 'Installing' : 'Finalizing' }
      })
    }, 180)
    return () => window.clearInterval(timer)
  }, [install?.name, onToast, desktop])

  const handleDrop = (event: DragEvent<HTMLDivElement>) => {
    event.preventDefault()
    setDragging(false)
    const file = event.dataTransfer.files[0]
    void startInstall(file?.name ?? 'Sideloaded.ipa', (file as File & { path?: string })?.path)
  }

  const uninstallSelected = async () => {
    if (!selected) return
    if (!desktop) {
      setApps((items) => items.filter((item) => item.id !== selected.id))
      onToast(`${selected.name} uninstalled`)
      return
    }
    if (!window.confirm(`Uninstall ${selected.name}?`)) return
    try {
      setInstall({ name: selected.name, progress: 0, phase: 'Uninstalling' })
      await api.appUninstall(selected.bundle, udid)
      setInstall(null)
      await loadApps()
      onToast(`${selected.name} uninstalled`)
    } catch (error) { setInstall(null); onToast(errorMessage(error)) }
  }

  return (
    <section className="apps-page page-padding compact-padding">
      <div className="apps-list-panel">
        <div className="apps-toolbar"><label><Search size={15} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search installed apps…" /></label><button className="primary-button" onClick={() => void startInstall()}><Upload size={15} />Install signed .ipa</button></div>
        <div className={`drop-zone ${dragging ? 'dragging' : ''}`} onClick={() => void startInstall()} onDragOver={(event) => { event.preventDefault(); setDragging(true) }} onDragLeave={() => setDragging(false)} onDrop={handleDrop}><span><Upload size={19} /></span><div><b>Drop a signed .ipa here to sideload</b><small>Code signature required · verified before transfer</small></div></div>
        {install && <div className="install-card card"><div><b>{install.name}</b><code>{install.progress}%</code></div><div className="progress"><span style={{ width: `${install.progress}%` }} /></div><small>{install.phase}…</small></div>}
        <div className="app-list">{filtered.map((app) => <button key={app.id} className={selectedId === app.id ? 'active' : ''} onClick={() => setSelectedId(app.id)}><AppIcon app={app} /><span><b>{app.name}</b><small>{app.bundle}</small></span><code className={app.fresh ? 'fresh' : ''}>{app.fresh ? 'new' : app.system ? 'system' : 'user'}</code><small className="app-size">{app.size}</small></button>)}</div>
      </div>
      {selected ? <aside className="app-detail card"><AppIcon app={selected} large /><h2>{selected.name}</h2><code>{selected.bundle}</code><div>{[['Version', selected.version], ['Size', selected.size], ['Type', selected.system ? 'System' : 'User'], ['Bundle', selected.bundle.split('.').slice(-1)[0] ?? '']].map(([label, value]) => <p key={label}><span>{label}</span><b>{value}</b></p>)}</div><button className="uninstall-button" onClick={() => void uninstallSelected()} disabled={selected.system}>Uninstall</button></aside> : <aside className="app-detail card empty-card">No applications returned.</aside>}
    </section>
  )
}

function AppIcon({ app, large }: { app: AppInfo; large?: boolean }) {
  return <span className={`app-icon ${large ? 'large' : ''}`} style={{ background: app.color }}>{app.icon ? <img src={app.icon} alt="" /> : app.name[0]}</span>
}

const demoCrashReports: CrashReportSummary[] = [
  { name: 'StikDebug-2026-07-23-184205.ips', path: '/StikDebug-2026-07-23-184205.ips', kind: 'IPS', process: 'StikDebug', sizeBytes: 184326, modified: '2026-07-23 18:42:05' },
  { name: 'JetsamEvent-2026-07-22-091814.ips', path: '/JetsamEvent-2026-07-22-091814.ips', kind: 'IPS', process: 'JetsamEvent', sizeBytes: 42812, modified: '2026-07-22 09:18:14' },
  { name: 'CrossCode-2026-07-20-221037.crash', path: '/Retired/CrossCode-2026-07-20-221037.crash', kind: 'CRASH', process: 'CrossCode', sizeBytes: 96340, modified: '2026-07-20 22:10:37' },
]

const demoCrashContent = `{"app_name":"StikDebug","timestamp":"2026-07-23 18:42:05.00 +0800","app_version":"1.4.2","slice_uuid":"4A9B1E5C-97B2-4D19-97F2-7BD6404C470A","build_version":"142","platform":2,"bundleID":"com.stik.debug","share_with_app_devs":1,"is_first_party":0,"bug_type":"309","os_version":"iPhone OS 17.5.1 (21F90)","incident_id":"A72B817D-8250-42A1-AC3B-9566A69D2F90"}
{
  "uptime" : 42000,
  "procRole" : "Foreground",
  "version" : 2,
  "userID" : 501,
  "modelCode" : "iPhone16,1",
  "coalitionName" : "com.stik.debug",
  "captureTime" : "2026-07-23 18:42:05.128 +0800",
  "exception" : {
    "type" : "EXC_BAD_ACCESS",
    "signal" : "SIGSEGV",
    "subtype" : "KERN_INVALID_ADDRESS at 0x0000000000000010"
  },
  "termination" : {
    "namespace" : "SIGNAL",
    "code" : 11,
    "indicator" : "Segmentation fault: 11"
  },
  "faultingThread" : 0,
  "threads" : [
    {
      "triggered" : true,
      "name" : "com.apple.main-thread",
      "frames" : [
        { "imageOffset" : 42824, "symbol" : "DebugSession.start()", "symbolLocation" : 184 },
        { "imageOffset" : 21760, "symbol" : "AppDelegate.application(_:didFinishLaunchingWithOptions:)", "symbolLocation" : 92 }
      ]
    }
  ]
}`

function CrashReports({ desktop, udid, onToast }: { desktop: boolean; udid: string; onToast: (message: string) => void }) {
  const [reports, setReports] = useState<CrashReportSummary[]>(desktop ? [] : demoCrashReports)
  const [selectedPath, setSelectedPath] = useState(desktop ? '' : demoCrashReports[0].path)
  const [content, setContent] = useState<CrashReportContent | null>(desktop ? null : {
    path: demoCrashReports[0].path,
    content: demoCrashContent,
    truncated: false,
    sizeBytes: demoCrashReports[0].sizeBytes ?? 0,
  })
  const [query, setQuery] = useState('')
  const [kind, setKind] = useState<'all' | 'ips' | 'crash'>('all')
  const [loading, setLoading] = useState(false)
  const selected = reports.find((report) => report.path === selectedPath)

  const loadReports = useCallback(async () => {
    if (!desktop) return
    setLoading(true)
    try {
      const loaded = await api.crashReportsList(udid)
      setReports(loaded)
      setSelectedPath((current) => loaded.some((report) => report.path === current) ? current : loaded[0]?.path ?? '')
    } catch (error) {
      onToast(errorMessage(error))
    } finally {
      setLoading(false)
    }
  }, [desktop, udid, onToast])

  useEffect(() => { void loadReports() }, [loadReports])
  useEffect(() => {
    if (!desktop || !selectedPath) {
      if (!selectedPath) setContent(null)
      return
    }
    let disposed = false
    setContent(null)
    void api.crashReportRead(selectedPath, udid)
      .then((result) => {
        if (disposed) return
        setContent(result)
        setReports((items) => items.map((report) => report.path === result.path ? { ...report, sizeBytes: result.sizeBytes } : report))
      })
      .catch((error) => { if (!disposed) onToast(errorMessage(error)) })
    return () => { disposed = true }
  }, [desktop, selectedPath, udid, onToast])

  const filtered = useMemo(() => reports.filter((report) => {
    const kindMatches = kind === 'all' || report.kind.toLowerCase() === kind
    const queryMatches = `${report.name} ${report.process} ${report.path}`.toLowerCase().includes(query.toLowerCase())
    return kindMatches && queryMatches
  }), [reports, kind, query])
  useEffect(() => {
    if (!filtered.some((report) => report.path === selectedPath)) {
      setSelectedPath(filtered[0]?.path ?? '')
    }
  }, [filtered, selectedPath])

  const exportSelected = async () => {
    if (!selected) return
    if (!desktop) {
      const blob = new Blob([content?.content ?? demoCrashContent], { type: 'text/plain;charset=utf-8' })
      const url = URL.createObjectURL(blob)
      const anchor = document.createElement('a')
      anchor.href = url
      anchor.download = selected.name
      anchor.click()
      URL.revokeObjectURL(url)
      onToast(`${selected.name} exported`)
      return
    }
    try {
      const target = await dialogs.saveFile(selected.name)
      if (!target) return
      await api.crashReportExport(selected.path, target, udid)
      onToast(`${selected.name} exported`)
    } catch (error) {
      onToast(errorMessage(error))
    }
  }

  return (
    <section className="crash-page page-padding compact-padding">
      <div className="crash-toolbar">
        <label><Search size={15} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search reports by app, filename, or path…" />{query && <button onClick={() => setQuery('')} aria-label="Clear report filter"><X size={14} /></button>}</label>
        <div className="crash-kind-filter">{(['all', 'ips', 'crash'] as const).map((item) => <button key={item} className={kind === item ? 'active' : ''} onClick={() => setKind(item)}>{item === 'all' ? 'All' : `.${item}`}</button>)}</div>
        <button className="crash-refresh" onClick={() => void loadReports()} disabled={loading}><RefreshCw className={loading ? 'spinning' : ''} size={14} />Refresh</button>
      </div>
      <div className="crash-workspace">
        <div className="crash-list card">
          <header><span>{filtered.length} reports</span><small>Newest first</small></header>
          <div>
            {filtered.map((report) => <button key={report.path} className={selectedPath === report.path ? 'active' : ''} onClick={() => setSelectedPath(report.path)}><span className="crash-kind"><Bug size={15} /></span><span><b>{report.process}</b><small>{report.name}</small><time>{report.modified}</time></span><code>{report.kind}</code><em>{bytes(report.sizeBytes)}</em></button>)}
            {!filtered.length && <div className="crash-empty">{loading ? 'Loading crash reports…' : reports.length ? 'No reports match this filter.' : 'No crash reports were returned by the device.'}</div>}
          </div>
        </div>
        <div className="crash-preview card">
          {selected ? <>
            <header><div><b>{selected.name}</b><small>{selected.path}</small></div><button className="primary-button" onClick={() => void exportSelected()} disabled={!content}><Download size={14} />Export</button></header>
            <div className="crash-summary"><span><small>Process</small><b>{selected.process}</b></span><span><small>Type</small><b>{selected.kind}</b></span><span><small>Modified</small><b>{selected.modified || '—'}</b></span><span><small>Size</small><b>{bytes(content?.sizeBytes ?? selected.sizeBytes)}</b></span></div>
            {content ? <><pre>{content.content}</pre>{content.truncated && <footer>Preview limited to 4 MB. Export saves the complete report.</footer>}</> : <div className="crash-empty">Loading report contents…</div>}
          </> : <div className="crash-empty">Select a crash report to preview its contents.</div>}
        </div>
      </div>
    </section>
  )
}

function Logs({ connected, desktop, udid, onError }: { connected: boolean; desktop: boolean; udid: string; onError: (message: string) => void }) {
  const [logs, setLogs] = useState<LogLine[]>(desktop ? [] : initialLogs)
  const [filter, setFilter] = useState<'all' | 'error' | 'warn' | 'info' | 'debug'>('all')
  const [query, setQuery] = useState('')
  const [useRegex, setUseRegex] = useState(false)
  const [paused, setPaused] = useState(false)
  const pausedRef = useRef(paused)
  const scrollRef = useRef<HTMLDivElement>(null)
  useEffect(() => { pausedRef.current = paused }, [paused])
  useEffect(() => {
    if (!desktop || !connected) return
    let unlisten: (() => void) | undefined
    events.logLine((line) => {
      if (pausedRef.current) return
      const rawLevel = line.level.toUpperCase()
      const level: LogLine['level'] = rawLevel === 'FAULT' ? 'ERROR' : ['INFO', 'DEBUG', 'NOTICE', 'WARN', 'ERROR'].includes(rawLevel) ? rawLevel as LogLine['level'] : 'INFO'
      setLogs((items) => [...items, { time: line.timestamp, level, process: `${line.process}[${line.pid}]`, message: line.message }].slice(-1000))
    }).then((stop) => { unlisten = stop })
    void api.logsStart(udid).catch((error) => onError(errorMessage(error)))
    return () => {
      unlisten?.()
      void api.logsStop()
    }
  }, [desktop, connected, udid, onError])
  useEffect(() => {
    if (desktop || paused || !connected) return
    const timer = window.setInterval(() => {
      const [level, process, message] = liveLogPool[Math.floor(Math.random() * liveLogPool.length)]
      const now = new Date()
      const time = `${now.toLocaleTimeString('en-GB', { hour12: false })}.${String(now.getMilliseconds()).padStart(3, '0')}`
      setLogs((items) => [...items, { time, level, process, message }].slice(-140))
    }, 1250)
    return () => window.clearInterval(timer)
  }, [paused, connected, desktop])
  useEffect(() => { if (scrollRef.current) scrollRef.current.scrollTop = scrollRef.current.scrollHeight }, [logs])
  const regexResult = useMemo(() => {
    if (!useRegex || !query) return { expression: null, error: '' }
    try {
      return { expression: new RegExp(query, 'i'), error: '' }
    } catch (error) {
      return { expression: null, error: error instanceof Error ? error.message : 'Invalid regular expression' }
    }
  }, [query, useRegex])
  const shown = useMemo(() => {
    const levelMatches = filter === 'all' ? logs : logs.filter((line) => line.level.toLowerCase() === filter)
    if (!query) return levelMatches
    if (useRegex && !regexResult.expression) return []
    const plainQuery = query.toLowerCase()
    return levelMatches.filter((line) => {
      const searchable = `${line.time} ${line.level} ${line.process} ${line.message}`
      return regexResult.expression ? regexResult.expression.test(searchable) : searchable.toLowerCase().includes(plainQuery)
    })
  }, [filter, logs, query, regexResult.expression, useRegex])
  return (
    <section className="logs-page">
      <div className="log-toolbar">{(['all', 'error', 'warn', 'info', 'debug'] as const).map((item) => <button key={item} className={filter === item ? 'active' : ''} onClick={() => setFilter(item)}>{item[0].toUpperCase() + item.slice(1)}</button>)}<span className="toolbar-spacer" /><small className={paused ? 'paused' : ''}><i />{paused ? 'paused' : 'live'}</small><button onClick={() => setPaused((value) => !value)}>{paused ? <Play size={13} /> : <Pause size={13} />}{paused ? 'Resume' : 'Pause'}</button><button onClick={() => setLogs([])}>Clear</button></div>
      <div className={`log-search ${regexResult.error ? 'invalid' : ''}`}>
        <label><Search size={15} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder={useRegex ? 'Filter logs with a regular expression…' : 'Filter logs…'} aria-invalid={Boolean(regexResult.error)} />{query && <button className="log-search-clear" onClick={() => setQuery('')} aria-label="Clear log filter"><X size={14} /></button>}</label>
        <button className={useRegex ? 'active' : ''} onClick={() => setUseRegex((value) => !value)} aria-pressed={useRegex} title="Use regular expression matching"><code>.*</code> Regex</button>
        <small title={regexResult.error || undefined}>{regexResult.error ? 'Invalid regular expression' : `${shown.length} / ${logs.length}`}</small>
      </div>
      <div className="log-console" ref={scrollRef}>{shown.map((line, index) => <div key={`${line.time}-${index}`}><time>{line.time}</time><b className={`level-${line.level.toLowerCase()}`}>{line.level}</b><code>{line.process}</code><span>{line.message}</span></div>)}<div className="terminal-cursor"><code>›</code><i /></div></div>
    </section>
  )
}

function Developer({ desktop, device, onToast }: { desktop: boolean; device: Device; onToast: (message: string) => void }) {
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
      const [nextStatus, nextApps] = await Promise.all([api.developerStatus(device.udid), api.appsList(device.udid)])
      setStatus(nextStatus)
      const userApps = nextApps.filter((app) => !app.system)
      setApps(userApps)
      setBundleId((current) => current || userApps[0]?.bundleId || '')
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
        <div className="card jit-card"><div><h2>Enable JIT</h2><p>Launch a user app, attach debugserver, and keep its JIT entitlement active.</p>{desktop && <select value={bundleId} onChange={(event) => setBundleId(event.target.value)}>{apps.map((app) => <option key={app.bundleId} value={app.bundleId}>{app.name} · {app.bundleId}</option>)}</select>}</div><button className={`toggle ${jit ? 'on' : ''}`} onClick={() => void toggleJit()}><span /></button><small><i className={jit ? 'good-dot' : ''} />{jit ? `debugserver attached${jitInfo ? ` · pid ${jitInfo.pid}` : ''}` : 'no process attached'}</small></div>
        <div className="card ddi-card"><h2>Developer Disk Image</h2><p><span>Status</span><b className={status.ddiMounted ? 'good' : ''}>{status.ddiMounted ? 'Mounted' : 'Not mounted'}</b></p><p><span>Developer Mode</span><b>{status.developerMode === null ? 'Unknown' : status.developerMode ? 'Enabled' : 'Disabled'}</b></p><p><span>RSD</span><b>{status.rsdAvailable ? 'Available' : 'Unavailable'}</b></p>{ddiProgress !== null && <div className="progress"><span style={{ width: `${ddiProgress}%` }} /></div>}<div className="dev-actions"><button className="primary-button" onClick={() => void autoMountDdi()}>Auto Mount DDI</button><button onClick={() => void mountDdi()}>Choose files</button><button onClick={() => void unmountDdi()} disabled={!status.ddiMounted}>Unmount</button></div></div>
      </div>
      <div className="developer-mode-actions"><button onClick={() => void developerAction('reveal')}>Show Developer Mode setting</button><button onClick={() => void developerAction('enable')}>Enable Developer Mode</button><button onClick={() => void developerAction('accept')}>Accept after reboot</button></div>
      <div className="service-grid">{[['RSD Tunnel', status.rsdAvailable ? 'remoted handshake available' : 'requires iOS 17+ and developer services', status.rsdAvailable], ['Developer Image', status.ddiMounted ? 'developer services ready' : 'mount an image for legacy services', status.ddiMounted], ['Debug Proxy', jit ? `attached to ${jitInfo?.bundleId ?? 'process'}` : 'idle · ready to attach', jit]].map(([name, detail, good]) => <div className="card" key={String(name)}><b><i className={good ? 'good-dot' : 'warn-dot'} />{name}</b><small>{detail}</small></div>)}</div>
      <div className="debug-console"><header><i />debugserver · com.apple.debugserver.DVTSecureSocketProxy</header><div>{jitInfo ? <><p><code>process </code>launch &quot;{jitInfo.bundleId}&quot;</p><p>Process {jitInfo.pid} launched and attached</p><p>memory limit disabled · JIT session active ✓</p></> : <p>No active debug session.</p>}<p className="terminal-cursor"><code>(lldb)</code><i /></p></div></div>
    </section>
  )
}

function Location({ desktop, udid, onToast }: { desktop: boolean; udid: string; onToast: (message: string) => void }) {
  const [presetId, setPresetId] = useState('sf')
  const [simulating, setSimulating] = useState(!desktop)
  const [custom, setCustom] = useState<{ lat: number; lng: number } | null>(null)
  const [transport, setTransport] = useState(desktop ? '' : 'DVT/RSD')
  const simulatingRef = useRef(simulating)
  const mapElementRef = useRef<HTMLDivElement>(null)
  const mapRef = useRef<LeafletMap | null>(null)
  const markerRef = useRef<LeafletMarker | null>(null)
  const preset = presets.find((item) => item.id === presetId) ?? presets[0]
  const loc = custom ? { ...custom, name: 'Selected location' } : preset
  const locationChanged = useCallback(() => {
    if (!desktop) setSimulating(true)
    else if (simulatingRef.current) {
      void api.locationStop()
      setSimulating(false)
      setTransport('')
    }
  }, [desktop])
  const selectPoint = useCallback((lat: number, lng: number) => {
    const wrappedLongitude = ((lng + 180) % 360 + 360) % 360 - 180
    setCustom({ lat: Math.max(-90, Math.min(90, lat)), lng: wrappedLongitude })
    locationChanged()
  }, [locationChanged])
  const toggleSimulation = async () => {
    if (!desktop) return setSimulating((value) => !value)
    try {
      if (simulating) {
        await api.locationStop()
        setSimulating(false)
        setTransport('')
        onToast('Location simulation cleared')
      } else {
        const session = await api.locationStart(loc.lat, loc.lng, udid)
        setSimulating(true)
        setTransport(session.transport)
        onToast(`Location applied over ${session.transport}`)
      }
    } catch (error) { onToast(errorMessage(error)) }
  }
  useEffect(() => { simulatingRef.current = simulating }, [simulating])
  useEffect(() => () => { if (desktop && simulatingRef.current) void api.locationStop() }, [desktop])
  useEffect(() => {
    if (!mapElementRef.current || mapRef.current) return
    const initial = presets[0]
    const map = L.map(mapElementRef.current, { zoomControl: true }).setView([initial.lat, initial.lng], 13)
    L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
      maxZoom: 19,
      attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors',
    }).addTo(map)
    const markerIcon = L.divIcon({ className: 'location-map-marker', html: '<span></span>', iconSize: [30, 40], iconAnchor: [15, 40] })
    const marker = L.marker([initial.lat, initial.lng], { icon: markerIcon }).addTo(map)
    map.on('click', ({ latlng }) => selectPoint(latlng.lat, latlng.lng))
    mapRef.current = map
    markerRef.current = marker
    window.setTimeout(() => map.invalidateSize(), 0)
    return () => {
      map.remove()
      mapRef.current = null
      markerRef.current = null
    }
  }, [selectPoint])
  useEffect(() => { markerRef.current?.setLatLng([loc.lat, loc.lng]) }, [loc.lat, loc.lng])
  const selectPreset = (item: typeof presets[number]) => {
    setPresetId(item.id)
    setCustom(null)
    markerRef.current?.setLatLng([item.lat, item.lng])
    mapRef.current?.flyTo([item.lat, item.lng], 13)
    locationChanged()
  }
  return (
    <section className="location-page page-padding compact-padding">
      <div className="location-controls"><div className="card coordinate-card"><h2>Simulated coordinates</h2><label>Latitude<strong>{loc.lat.toFixed(4)}</strong></label><label>Longitude<strong>{loc.lng.toFixed(4)}</strong></label><button className={simulating ? 'stop-button' : 'primary-button'} onClick={() => void toggleSimulation()}>{simulating ? 'Stop simulation' : 'Start simulation'}</button><small><i className={simulating ? 'good-dot' : ''} />{simulating ? `Location override active${transport ? ` · ${transport}` : ''}` : 'Using real GPS'}</small></div><h3 className="section-label">Presets</h3><div className="preset-list">{presets.map((item) => <button key={item.id} className={!custom && presetId === item.id ? 'active' : ''} onClick={() => selectPreset(item)}><MapPin size={14} /><span>{item.name}</span><small>{item.lat.toFixed(2)},{item.lng.toFixed(2)}</small></button>)}</div></div>
      <div className="map-grid" ref={mapElementRef}>
        <div className="map-caption"><b>{loc.name} · {loc.lat.toFixed(4)}, {loc.lng.toFixed(4)}</b><small>drag to pan · scroll to zoom · click to select</small></div>
        <code className="map-service">com.apple.dt.simulatelocation</code>
      </div>
    </section>
  )
}

function Onboarding({ state, device, desktop, onDetect, onPair, onCancel }: { state: Exclude<Connection, 'connected'>; device: Device; desktop: boolean; onDetect: () => void; onPair: () => void; onCancel: () => void }) {
  if (state === 'none') return <div className="onboarding"><div className="scan-icon"><i /><i /><span><Smartphone size={31} /></span></div><h2>No device connected</h2><p>Connect an iPhone or iPad over USB or make it available on the local network. idevice is scanning usbmuxd and Bonjour.</p><code><i />usbmuxd · mobdev2 · RemotePairing</code><button className="primary-button" onClick={onDetect}><Usb size={16} />{desktop ? 'Scan again' : 'Simulate USB connection'}</button><div className="steps"><span className="active">Detect</span><i /><span>Trust</span><i /><span>Ready</span></div></div>
  if (device.connectable === false) return <div className="onboarding"><span className="detected-icon"><Wifi size={31} /><i>!</i></span><h2>{device.name} found on the network</h2><p>Bonjour can see this device, but no paired Lockdown route is available yet.</p><div className="trust-card card"><p><b>1</b><span>Connect the device with a <strong>USB data cable</strong></span></p><p><b>2</b><span>Unlock it and tap <strong>Trust</strong> when prompted</span></p></div><button className="primary-button full-button" onClick={onDetect}>Check connections again</button><button className="text-button" onClick={onCancel}>Cancel</button></div>
  return <div className="onboarding"><span className="detected-icon"><Smartphone size={31} /><i>!</i></span><h2>{device.model} detected</h2><p>Trust this computer to open a lockdown session.</p><div className="trust-card card"><p><b>1</b><span>Tap <strong>Trust</strong> on the device prompt</span></p><p><b>2</b><span>Enter the device passcode</span></p><div className="passcode"><i>•</i><i>•</i><i>•</i><i /></div></div><button className="primary-button full-button" onClick={onPair}>Trust & generate pairing</button><button className="text-button" onClick={onCancel}>Cancel</button></div>
}

function PairModal({ onClose, onPair }: { onClose: () => void; onPair: () => void }) {
  return <div className="modal-backdrop" onMouseDown={onClose}><div className="pair-modal" onMouseDown={(event) => event.stopPropagation()}><header><div><h2>Pair new device</h2><p>Generate a lockdown pairing record over usbmuxd.</p></div><button onClick={onClose}><X size={18} /></button></header><div className="pair-body"><p><b>1</b><span><strong>Connect via USB</strong><small>A new device was detected on the muxer.</small></span></p><p><b>2</b><span><strong>Tap “Trust” on device</strong><small>Then enter the passcode below.</small></span></p><div className="passcode"><i>•</i><i>•</i><i>•</i><i /></div></div><footer><button onClick={onClose}>Cancel</button><button className="primary-button" onClick={onPair}>Generate pairing</button></footer></div></div>
}

export default App
