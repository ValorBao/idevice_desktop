import { useCallback, useEffect, useRef, useState } from 'react'
import {
  ChevronDown, CircleStop, FolderOpen, Layers, Plus, RefreshCw, Smartphone,
  Sliders, TerminalSquare, Zap,
} from 'lucide-react'
import type { Device } from '../data'
import { api, errorMessage, type DeveloperStatus } from '../api'
import { deviceScreenCache } from '../lib/device'
import type { Connection, WorkbenchMode } from '../types'

interface LeftRailProps {
  device: Device
  deviceCatalog: Device[]
  connection: Connection
  desktop: boolean
  mode: WorkbenchMode
  onSelectMode: (mode: WorkbenchMode) => void
  onSelectDevice: (id: string) => void
  onPairOpen: () => void
  onDisconnect: () => void
  onToast: (msg: string) => void
}

export function LeftRail({
  device,
  deviceCatalog,
  connection,
  desktop,
  mode,
  onSelectMode,
  onSelectDevice,
  onPairOpen,
  onDisconnect,
  onToast,
}: LeftRailProps) {
  const [deviceMenu, setDeviceMenu] = useState(false)
  const [screenImage, setScreenImage] = useState(() => deviceScreenCache.get(device.udid) ?? '')
  const [screenLoading, setScreenLoading] = useState(false)
  const [devStatus, setDevStatus] = useState<DeveloperStatus | null>(null)
  const connected = connection === 'connected'

  const refreshScreen = useCallback(async (autoMount = false) => {
    if (!desktop || !connected) return
    setScreenLoading(true)
    try {
      const img = await api.screenshot(device.udid)
      deviceScreenCache.set(device.udid, img)
      setScreenImage(img)
    } catch {
      if (autoMount) {
        try {
          await api.ddiEnsure(device.udid)
          const img = await api.screenshot(device.udid)
          deviceScreenCache.set(device.udid, img)
          setScreenImage(img)
        } catch {
          // ignore fallback error
        }
      }
    } finally {
      setScreenLoading(false)
    }
  }, [desktop, connected, device.udid])

  useEffect(() => {
    if (!desktop || !connected) {
      setDevStatus(null)
      return
    }
    void api.developerStatus(device.udid)
      .then(setDevStatus)
      .catch(() => setDevStatus(null))
  }, [desktop, connected, device.udid])

  useEffect(() => {
    const cached = deviceScreenCache.get(device.udid)
    if (cached) {
      setScreenImage(cached)
      return
    }
    setScreenImage('')
    void refreshScreen(false)
  }, [device.udid, refreshScreen])

  const modes: { id: WorkbenchMode; label: string; icon: typeof Layers; sub: string }[] = [
    { id: 'inspect', label: 'INSPECT', icon: Sliders, sub: 'Hardware & Health' },
    { id: 'files', label: 'FILES', icon: FolderOpen, sub: 'AFC & Sandboxes' },
    { id: 'apps', label: 'APPS', icon: Layers, sub: 'Manage & JIT' },
    { id: 'watch', label: 'WATCH', icon: TerminalSquare, sub: 'Logs & Probes' },
  ]

  const hasUsb = device.transports?.includes('USB') || device.conn.includes('USB')
  const hasWifi = device.transports?.includes('Wi-Fi') || device.conn.includes('Wi-Fi') || device.conn.includes('Network')
  const hasRemotePairing = device.transports?.includes('RemotePairing') || Boolean(devStatus?.rsdAvailable)

  return (
    <aside className="left-rail">
      {/* Top Device Switcher / Badge */}
      <div className="device-select-wrap">
        <button
          className="device-card"
          onClick={() => setDeviceMenu((v) => !v)}
          aria-expanded={deviceMenu}
          aria-label="Select device"
          title={connected ? device.name : 'Select device'}
        >
          <span className={`device-icon status-${connection}`}>
            <Smartphone size={18} />
            <i />
          </span>
          <span className="device-card-copy">
            <b>{connected ? device.name : connection === 'detected' ? device.model : 'No device'}</b>
            <small>
              {connected
                ? `${device.model} · iOS ${device.ios}`
                : connection === 'detected'
                  ? device.connectable === false
                    ? 'Network only · connect USB'
                    : 'Awaiting trust…'
                  : 'Connect device to begin'}
            </small>
          </span>
          <ChevronDown size={14} />
        </button>

        {deviceMenu && (
          <div className="device-menu">
            {deviceCatalog.map((item) => (
              <button key={item.id} onClick={() => { onSelectDevice(item.id); setDeviceMenu(false) }}>
                <i className={item.id === device.id ? 'selected-dot' : ''} />
                <span>
                  <b>{item.name}</b>
                  <small>{item.conn}{item.ios !== '—' ? ` · iOS ${item.ios}` : ''}</small>
                </span>
              </button>
            ))}
            <hr />
            <button className="accent-action" onClick={() => { onPairOpen(); setDeviceMenu(false) }}>
              <Plus size={15} />Pair new device…
            </button>
            <button className="danger-action" onClick={() => { onDisconnect(); setDeviceMenu(false) }}>
              <CircleStop size={15} />Disconnect device
            </button>
          </div>
        )}
      </div>

      {/* Mini Device Object & Screenshot Stage */}
      <div className="bench-device-stage">
        <div className={`mini-phone-frame ${screenImage ? 'has-screen' : ''}`}>
          {screenImage ? (
            <img src={screenImage} alt="Device mini screen preview" />
          ) : (
            <div className="mini-screen-placeholder">
              <Zap size={16} />
              <small>{connected ? (screenLoading ? 'Loading…' : device.chip) : 'Offline'}</small>
            </div>
          )}
          <span className="mini-phone-notch" />
          {desktop && connected && (
            <button
              className={`mini-phone-refresh ${screenLoading ? 'loading' : ''}`}
              type="button"
              title="Refresh screen capture"
              aria-label="Refresh screen capture"
              disabled={screenLoading}
              onClick={() => void refreshScreen(true)}
            >
              <RefreshCw size={11} />
            </button>
          )}
        </div>

        <div className="bench-device-info">
          <div className="device-tag">
            <span>{connected ? 'LIVE HARDWARE' : 'DISCONNECTED'}</span>
            <code>{device.udid.slice(0, 8)}…{device.udid.slice(-4)}</code>
          </div>
        </div>
      </div>

      {/* Hardware / Session Lab Probes */}
      <div className="session-probes">
        <span className="probe-heading">SESSION TELEMETRY</span>
        <div className="probe-list">
          <div className="probe-row" title="Device pairing status">
            <span className="probe-name">PAIR</span>
            <span className={`probe-led ${connected ? 'led-green' : 'led-dim'}`} />
            <span className="probe-state">{connected ? 'trusted' : 'unpaired'}</span>
          </div>
          <div className="probe-row" title="Developer Disk Image">
            <span className="probe-name">DDI</span>
            <span className={`probe-led ${devStatus ? (devStatus.ddiMounted ? 'led-green' : 'led-dim') : (connected ? 'led-green' : 'led-dim')}`} />
            <span className="probe-state">{devStatus ? (devStatus.ddiMounted ? 'mounted' : 'not mounted') : (connected ? 'mounted' : 'not mounted')}</span>
          </div>
          <div className="probe-row" title="Developer Mode Status">
            <span className="probe-name">DEV MODE</span>
            <span className={`probe-led ${devStatus ? (devStatus.developerMode ? 'led-green' : 'led-dim') : (connected ? 'led-green' : 'led-dim')}`} />
            <span className="probe-state">{devStatus ? (devStatus.developerMode === null ? '—' : devStatus.developerMode ? 'active' : 'off') : (connected ? 'active' : 'off')}</span>
          </div>
          <div className="probe-row" title="RSD / CoreDevice Tunnel">
            <span className="probe-name">RSD / TUNNEL</span>
            <span className={`probe-led ${devStatus ? (devStatus.rsdAvailable ? 'led-green' : 'led-dim') : (connected ? 'led-green' : 'led-dim')}`} />
            <span className="probe-state">{devStatus ? (devStatus.rsdAvailable ? 'ready' : 'idle') : (connected ? 'ready' : 'idle')}</span>
          </div>
        </div>

        {/* Transport indicators */}
        <div className="probe-transports">
          <span className={`transport-chip ${hasUsb ? 'active' : ''}`}>USB</span>
          <span className={`transport-chip ${hasWifi ? 'active' : ''}`}>Wi-Fi</span>
          <span className={`transport-chip ${hasRemotePairing ? 'active' : ''}`}>RemotePairing</span>
        </div>
      </div>

      {/* 4 Primary Workbenches */}
      <div className="bench-nav-wrap">
        <span className="probe-heading">STATIONS</span>
        <nav className={`bench-nav ${connected ? '' : 'nav-disabled'}`}>
          {modes.map(({ id, label, icon: Icon, sub }) => (
            <button
              key={id}
              className={`bench-nav-button ${mode === id ? 'active' : ''}`}
              onClick={() => onSelectMode(id)}
              aria-label={label}
              title={`${label} · ${sub}`}
            >
              <Icon size={16} />
              <div className="nav-copy">
                <b>{label}</b>
                <small>{sub}</small>
              </div>
            </button>
          ))}
        </nav>
      </div>

      <div className="left-rail-footer">
        <i />
        <span>
          <b>VOID STATION</b>
          <small>v0.0.4 · macOS ground</small>
        </span>
      </div>
    </aside>
  )
}
