import { useCallback, useEffect, useState } from 'react'
import { Check, RefreshCw } from 'lucide-react'
import type { Device } from '../data'
import { api, errorMessage, type DeviceOverview } from '../api'
import type { Page } from '../types'
import { deviceScreenCache } from '../lib/device'
import { bytes } from '../lib/format'
import { StatCard } from '../components/StatCard'

export function Overview({ device, desktop, onNavigate, onError }: { device: Device; desktop: boolean; onNavigate: (page: Page) => void; onError: (message: string) => void }) {
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
