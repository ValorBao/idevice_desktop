import { useCallback, useEffect, useRef, useState } from 'react'
import { RefreshCw, Zap } from 'lucide-react'
import type { Device } from '../data'
import { api, errorMessage, type DeviceOverview } from '../api'
import { deviceScreenCache } from '../lib/device'
import { bytes } from '../lib/format'

export function Overview({ device, desktop, onError }: { device: Device; desktop: boolean; onError: (message: string) => void }) {
  const [overview, setOverview] = useState<DeviceOverview | null>(null)
  const [screenImage, setScreenImage] = useState(() => deviceScreenCache.get(device.udid) ?? '')
  const [screenError, setScreenError] = useState('')
  const [screenLoading, setScreenLoading] = useState(false)
  const [phoneRotation, setPhoneRotation] = useState({ x: 0, y: 0 })
  const [phoneDragging, setPhoneDragging] = useState(false)
  const dragStart = useRef<{ x: number; y: number; rx: number; ry: number } | null>(null)
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
  const startPhoneRotation = (event: React.PointerEvent<HTMLDivElement>) => {
    if (event.target instanceof Element && event.target.closest('button')) return
    event.currentTarget.setPointerCapture(event.pointerId)
    dragStart.current = { x: event.clientX, y: event.clientY, rx: phoneRotation.x, ry: phoneRotation.y }
    setPhoneDragging(true)
  }
  const movePhoneRotation = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!dragStart.current) return
    const deltaX = event.clientX - dragStart.current.x
    const deltaY = event.clientY - dragStart.current.y
    setPhoneRotation({
      x: Math.max(-18, Math.min(18, dragStart.current.rx - deltaY * .16)),
      y: Math.max(-38, Math.min(38, dragStart.current.ry + deltaX * .2)),
    })
  }
  const stopPhoneRotation = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!dragStart.current) return
    dragStart.current = null
    setPhoneDragging(false)
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId)
  }
  return (
    <section className="cockpit">
      <div className="cockpit-noise" />
      <div className="cockpit-word" aria-hidden="true">DEVICE</div>

      <header className="cockpit-header">
        <div>
          <small>CONNECTED OBJECT</small>
          <h1>{device.name}</h1>
        </div>
        <div className="cockpit-live"><i /> LIVE</div>
      </header>

      <div className="cockpit-stage">
        <aside className="cockpit-model">
          <span>HARDWARE</span>
          <strong>{overview?.productType ?? device.model}</strong>
          <small>{overview?.hardwareModel ?? device.modelId}</small>
          <div className="cockpit-rule"><i /></div>
          <code>{device.udid}</code>
        </aside>

        <div
          className={`cockpit-device ${phoneDragging ? 'is-dragging' : ''}`}
          style={{ '--phone-rx': `${phoneRotation.x}deg`, '--phone-ry': `${phoneRotation.y}deg` } as React.CSSProperties}
          title="Drag to rotate · double-click to reset"
          onPointerDown={startPhoneRotation}
          onPointerMove={movePhoneRotation}
          onPointerUp={stopPhoneRotation}
          onPointerCancel={stopPhoneRotation}
          onDoubleClick={() => setPhoneRotation({ x: 0, y: 0 })}
        >
          <div className="cockpit-orbit cockpit-orbit-one" />
          <div className="cockpit-orbit cockpit-orbit-two" />
          <div className={`phone-frame cockpit-phone ${screenImage ? 'has-screen' : ''}`} title={screenError || 'Device screen'}>
            {screenImage ? <img src={screenImage} alt="Current device screen" /> : <div className="cockpit-screen"><Zap size={25} /><small>{screenStatus}</small></div>}
            <span className="phone-notch" />
            {desktop && <button className={`phone-refresh ${screenLoading ? 'loading' : ''}`} type="button" title="Refresh device screen" aria-label="Refresh device screen" disabled={screenLoading} onClick={() => void refreshScreen(true)}><RefreshCw size={13} /></button>}
          </div>
          <div className="cockpit-signal"><i /><span>{overview?.connection ?? device.conn}</span><b>LOCKDOWN / ACTIVE</b></div>
        </div>

        <aside className="cockpit-readout">
          <div><span>OS</span><strong>{overview?.productVersion ?? device.ios}</strong><small>{overview?.buildVersion ?? device.build}</small></div>
          <div><span>POWER</span><strong>{battery}<sup>%</sup></strong><small>{batteryHealth || '—'} health / {cycles || '—'} cycles</small></div>
          <div><span>FREE</span><strong>{bytes(storageFree)}</strong><small>of {bytes(storageTotal)}</small></div>
          <div className="cockpit-storage"><i style={{ width: `${storagePercent}%` }} /></div>
        </aside>
      </div>

    </section>
  )
}
