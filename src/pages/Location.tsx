import { useCallback, useEffect, useRef, useState } from 'react'
import L, { type Map as LeafletMap, type Marker as LeafletMarker } from 'leaflet'
import 'leaflet/dist/leaflet.css'
import { MapPin } from 'lucide-react'
import { presets } from '../data'
import { api } from '../api'
import { useDeviceTask } from '../lib/hooks'

export function Location({ desktop, udid, onToast }: { desktop: boolean; udid: string; onToast: (message: string) => void }) {
  const [presetId, setPresetId] = useState('sf')
  const [simulating, setSimulating] = useState(!desktop)
  const [custom, setCustom] = useState<{ lat: number; lng: number } | null>(null)
  const [transport, setTransport] = useState(desktop ? '' : 'DVT/RSD')
  const simulatingRef = useRef(simulating)
  const mapElementRef = useRef<HTMLDivElement>(null)
  const mapRef = useRef<LeafletMap | null>(null)
  const markerRef = useRef<LeafletMarker | null>(null)
  const runTask = useDeviceTask(desktop, onToast)
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
  const toggleSimulation = () => runTask(async () => {
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
  }, () => setSimulating((value) => !value))
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
