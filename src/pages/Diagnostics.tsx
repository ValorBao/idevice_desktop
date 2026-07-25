import { useEffect, useMemo, useState } from 'react'
import { Search } from 'lucide-react'
import { gestalt, ioTree, type Device } from '../data'
import { api, errorMessage } from '../api'
import { flatten } from '../lib/format'

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
