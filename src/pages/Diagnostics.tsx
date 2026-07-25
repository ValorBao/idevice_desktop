import { useEffect, useId, useMemo, useState } from 'react'
import { Search } from 'lucide-react'
import { gestalt, ioTree, type Device } from '../data'
import { api, errorMessage } from '../api'
import { flatten } from '../lib/format'

function StatValue({ value }: { value: string }) {
  const parsed = useMemo(() => {
    const match = value.match(/^(\d+(?:\.\d+)?)(.*)$/)
    if (!match) return null
    return { target: parseFloat(match[1]), suffix: match[2], decimals: match[1].includes('.') ? match[1].split('.')[1].length : 0 }
  }, [value])
  const [progress, setProgress] = useState(0)
  useEffect(() => {
    if (!parsed || window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      setProgress(1)
      return
    }
    setProgress(0)
    let frame = 0
    const start = performance.now()
    const tick = (now: number) => {
      const linear = Math.min((now - start) / 700, 1)
      setProgress(1 - Math.pow(1 - linear, 3))
      if (linear < 1) frame = requestAnimationFrame(tick)
    }
    frame = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(frame)
  }, [parsed])
  if (!parsed) return <strong>{value}</strong>
  return <strong>{(parsed.target * progress).toFixed(parsed.decimals)}{parsed.suffix}</strong>
}

function Sparkline({ points }: { points: number[] }) {
  const gradientId = useId()
  const min = Math.min(...points)
  const range = (Math.max(...points) - min) || 1
  const coords = points.map((point, index): [number, number] => [(index / (points.length - 1)) * 100, 24 - ((point - min) / range) * 19])
  // Catmull-Rom → cubic bezier so the line flows instead of kinking at each sample
  const line = coords.reduce((path, [x, y], index) => {
    if (index === 0) return `M ${x.toFixed(2)} ${y.toFixed(2)}`
    const [previousX, previousY] = coords[index - 1]
    const [beforeX, beforeY] = coords[index - 2] ?? coords[index - 1]
    const [nextX, nextY] = coords[index + 1] ?? coords[index]
    const control1 = [previousX + (x - beforeX) / 6, previousY + (y - beforeY) / 6]
    const control2 = [x - (nextX - previousX) / 6, y - (nextY - previousY) / 6]
    return `${path} C ${control1[0].toFixed(2)} ${control1[1].toFixed(2)}, ${control2[0].toFixed(2)} ${control2[1].toFixed(2)}, ${x.toFixed(2)} ${y.toFixed(2)}`
  }, '')
  return (
    <svg className="stat-spark" viewBox="0 0 100 28" preserveAspectRatio="none" aria-hidden="true">
      <defs>
        <linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="var(--accent)" stopOpacity=".22" />
          <stop offset="1" stopColor="var(--accent)" stopOpacity="0" />
        </linearGradient>
      </defs>
      <path className="spark-fill" d={`${line} L 100 28 L 0 28 Z`} fill={`url(#${gradientId})`} stroke="none" />
      <path className="spark-line" d={line} pathLength={1} fill="none" stroke="var(--accent)" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  )
}

const batterySparks: Record<string, number[]> = {
  Health: [97, 97, 96, 96, 95, 95, 95, 94, 94, 94],
  'Cycle count': [148, 160, 171, 180, 188, 196, 201, 206, 209, 212],
  Temperature: [29.8, 30.4, 31.9, 32.6, 31.8, 30.9, 31.4, 31.0, 31.6, 31.2],
  Voltage: [3.92, 3.98, 4.05, 4.10, 4.06, 4.12, 4.15, 4.10, 4.16, 4.18],
}

export function Diagnostics({ device, desktop, onError }: { device: Device; desktop: boolean; onError: (message: string) => void }) {
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
        <div className="diagnostic-stats">{batteryStats.map(([label, value, note]) => <div className="card" key={label}><span className="stat-label">{label}</span><StatValue value={value} /><small className={label === 'Health' ? 'good' : ''}>{note}</small><Sparkline points={batterySparks[label]} /></div>)}</div>
        <div className="chart-card card"><b>Capacity over last 30 charge cycles</b><div className="bar-chart">{chart.map((height, index) => <span key={index} data-value={`${height}%`} style={{ height: `${height}%`, opacity: .45 + index * .022, animationDelay: `${index * 18}ms` }} />)}</div></div>
      </>}
      {!desktop && tab === 'gestalt' && <div className="table-card card"><div className="search-row"><Search size={15} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Filter keys…" /><small>{filteredGestalt.length} keys</small></div>{filteredGestalt.map(([key, value]) => <div className="key-value-row" key={key}><code>{key}</code><span>{value}</span></div>)}</div>}
      {!desktop && tab === 'io' && <div className="io-tree card">{ioTree.map(([prefix, name, meta], index) => <div key={`${name}-${index}`}><span>{prefix}</span><b>{name}</b><small>{meta && `  ${meta}`}</small></div>)}</div>}
      {!desktop && (tab === 'nand' || tab === 'wifi') && <div className="empty-card card">Connect the desktop app to query this diagnostic.</div>}
      {desktop && <div className="table-card card"><div className="search-row"><Search size={15} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Filter keys…" /><small>{loading ? 'querying…' : `${realRows.length} values`}</small></div>{!loading && realRows.map(([key, value]) => <div className="key-value-row" key={key}><code>{key}</code><span>{value}</span></div>)}</div>}
    </section>
  )
}
