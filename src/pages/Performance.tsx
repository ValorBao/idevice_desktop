import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Download, Pause, Play, Search } from 'lucide-react'
import {
  api,
  dialogs,
  errorMessage,
  events,
  type PerformanceExportRow,
  type PerformanceSample,
  type PerformanceStatus,
} from '../api'
import { on, useDeviceEvents } from '../lib/useDeviceEvents'
import { useInterval } from '../lib/useInterval'
import { demoProcesses } from '../data'

const MAX_HISTORY = 180
const DEFAULT_INTERVAL_MS = 1_000

const bytes = (value: number | null) => {
  if (value === null) return '—'
  if (value < 1024) return `${value} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  let size = value / 1024
  let unit = 0
  while (size >= 1024 && unit < units.length - 1) {
    size /= 1024
    unit += 1
  }
  return `${size < 10 ? size.toFixed(1) : size.toFixed(0)} ${units[unit]}`
}

const percent = (value: number | null) => value === null ? '—' : `${value.toFixed(1)}%`

const demoSample = (sequence: number, intervalMs: number): PerformanceSample => {
  const cpu = [34, 18, 9, 4, 1.4, 0.6]
  const memory = [384, 246, 172, 84, 41, 24]
  const processes = demoProcesses.slice(0, 6).map((process, index) => ({
    pid: process.pid,
    name: process.name,
    identity: process.identity,
    cpuPercent: Math.max(0, cpu[index] + Math.sin(sequence / 2 + index) * (5 - index * 0.5)),
    memoryBytes: Math.round((memory[index] + Math.sin(sequence / 4 + index) * 8) * 1024 * 1024),
  }))
  return {
    sequence,
    timestampMs: Date.now(),
    intervalMs,
    transport: 'DVT Sysmontap · demonstration',
    systemCpuPercent: 31 + Math.sin(sequence / 3) * 8,
    processes,
  }
}

const csvField = (value: string) => /[",\r\n]/.test(value) ? `"${value.replace(/"/g, '""')}"` : value

const browserCsv = (rows: PerformanceExportRow[]) => [
  'timestamp_ms,pid,name,identity,cpu_percent,memory_bytes',
  ...rows.map((row) => [
    row.timestampMs,
    row.pid,
    csvField(row.name),
    csvField(row.identity),
    row.cpuPercent ?? '',
    row.memoryBytes ?? '',
  ].join(',')),
].join('\n') + '\n'

function Sparkline({ values, color, ceiling }: { values: Array<number | null>; color: string; ceiling: number }) {
  const paths = useMemo(() => {
    const width = 360
    const height = 90
    const max = Math.max(ceiling, ...values.flatMap((value) => value === null ? [] : [value]), 1)
    const segments: string[] = []
    let points: string[] = []
    values.forEach((value, index) => {
      if (value === null) {
        if (points.length) segments.push(points.join(' '))
        points = []
        return
      }
      const x = values.length <= 1 ? width : (index / (values.length - 1)) * width
      const y = height - Math.min(value / max, 1) * (height - 8) - 4
      points.push(`${x.toFixed(1)},${y.toFixed(1)}`)
    })
    if (points.length) segments.push(points.join(' '))
    return segments
  }, [ceiling, values])

  return (
    <svg className="performance-sparkline" viewBox="0 0 360 90" preserveAspectRatio="none" aria-hidden="true">
      <line x1="0" y1="85" x2="360" y2="85" />
      <line x1="0" y1="45" x2="360" y2="45" />
      {paths.map((points, index) => <polyline key={`${index}-${points}`} points={points} style={{ stroke: color }} />)}
    </svg>
  )
}

export function Performance({ desktop, udid, onToast }: { desktop: boolean; udid: string; onToast: (message: string) => void }) {
  const [history, setHistory] = useState<PerformanceSample[]>(desktop ? [] : [demoSample(1, DEFAULT_INTERVAL_MS)])
  const [status, setStatus] = useState<PerformanceStatus>({
    state: desktop ? 'connecting' : 'running',
    message: null,
    transport: desktop ? null : 'DVT Sysmontap · demonstration',
    intervalMs: DEFAULT_INTERVAL_MS,
  })
  const [intervalMs, setIntervalMs] = useState(DEFAULT_INTERVAL_MS)
  const [query, setQuery] = useState('')
  const [selectedIdentity, setSelectedIdentity] = useState('')
  const mountedRef = useRef(true)
  const runningRequestedRef = useRef(true)
  const intervalRef = useRef(intervalMs)
  const demoSequenceRef = useRef(1)

  useEffect(() => { intervalRef.current = intervalMs }, [intervalMs])
  useEffect(() => {
    mountedRef.current = true
    return () => { mountedRef.current = false }
  }, [])

  const appendSample = useCallback((sample: PerformanceSample) => {
    if (!mountedRef.current) return
    setHistory((samples) => [...samples, sample].slice(-MAX_HISTORY))
    setSelectedIdentity((current) => current || sample.processes[0]?.identity || '')
  }, [])

  useDeviceEvents(desktop, () => {
    runningRequestedRef.current = true
    return {
      listeners: [
        on(events.performanceSample, appendSample),
        on(events.performanceStatus, (next) => {
          if (next.state === 'error' || next.state === 'unavailable') runningRequestedRef.current = false
          setStatus(next.state === 'stopped' ? { ...next, state: 'paused' } : next)
          if (next.state === 'error' && next.message) onToast(next.message)
        }),
      ],
      // Pausing during listener registration must not start a late stream.
      start: () => (runningRequestedRef.current ? api.performanceStart(udid, intervalRef.current) : undefined),
      stop: () => {
        runningRequestedRef.current = false
        void api.performanceStop()
      },
    }
  }, (message) => {
    setStatus((current) => ({ ...current, state: 'error', message }))
    onToast(message)
  }, [appendSample, udid])

  useInterval(!desktop && status.state === 'running', intervalMs, () => {
    demoSequenceRef.current += 1
    appendSample(demoSample(demoSequenceRef.current, intervalRef.current))
  })

  const latest = history.length ? history[history.length - 1] : null
  const shown = useMemo(() => {
    const needle = query.trim().toLowerCase()
    if (!needle) return latest?.processes ?? []
    return (latest?.processes ?? []).filter((process) =>
      `${process.name} ${process.pid}`.toLowerCase().includes(needle),
    )
  }, [latest, query])
  const selected = useMemo(() => {
    if (!selectedIdentity) return null
    for (let index = history.length - 1; index >= 0; index -= 1) {
      const process = history[index].processes.find((item) => item.identity === selectedIdentity)
      if (process) return process
    }
    return null
  }, [history, selectedIdentity])
  const selectedHistory = useMemo(() => history.flatMap((sample) => {
    const process = sample.processes.find((item) => item.identity === selectedIdentity)
    return process ? [{ timestampMs: sample.timestampMs, process }] : []
  }), [history, selectedIdentity])
  const selectedLatest = latest?.processes.find((process) => process.identity === selectedIdentity) ?? null
  const exportRows: PerformanceExportRow[] = useMemo(() => selectedHistory.map(({ timestampMs, process }) => ({
    timestampMs,
    pid: process.pid,
    name: process.name,
    identity: process.identity,
    cpuPercent: process.cpuPercent,
    memoryBytes: process.memoryBytes,
  })), [selectedHistory])
  const cpuValues = selectedHistory.map(({ process }) => process.cpuPercent)
  const memoryValues = selectedHistory.map(({ process }) => process.memoryBytes === null ? null : process.memoryBytes / 1024 / 1024)

  const pause = async () => {
    if (status.state !== 'running' && status.state !== 'connecting') return
    runningRequestedRef.current = false
    try {
      if (desktop) await api.performanceStop()
      if (mountedRef.current) setStatus((current) => ({ ...current, state: 'paused', message: null }))
    } catch (error) {
      runningRequestedRef.current = true
      onToast(errorMessage(error))
    }
  }

  const resume = async () => {
    if (status.state === 'unavailable') return
    runningRequestedRef.current = true
    setStatus((current) => ({ ...current, state: desktop ? 'connecting' : 'running', message: null, intervalMs }))
    try {
      if (desktop) await api.performanceStart(udid, intervalMs)
    } catch (error) {
      runningRequestedRef.current = false
      const message = errorMessage(error)
      setStatus((current) => ({ ...current, state: 'error', message }))
      onToast(message)
    }
  }

  const exportSelected = async () => {
    if (!selected || !exportRows.length) return
    const safeName = selected.name.replace(/[^a-z0-9._-]+/gi, '-').replace(/^-|-$/g, '') || `pid-${selected.pid}`
    const filename = `performance-${safeName}-${selected.pid}.csv`
    try {
      if (desktop) {
        const target = await dialogs.saveFile(filename)
        if (!target) return
        await api.performanceExportCsv(target, exportRows)
      } else {
        const url = URL.createObjectURL(new Blob([browserCsv(exportRows)], { type: 'text/csv;charset=utf-8' }))
        const anchor = document.createElement('a')
        anchor.href = url
        anchor.download = filename
        anchor.click()
        URL.revokeObjectURL(url)
      }
      onToast(`Exported ${exportRows.length} performance samples`)
    } catch (error) {
      onToast(errorMessage(error))
    }
  }

  const running = status.state === 'running' || status.state === 'connecting'
  const statusLabel = status.state === 'connecting'
    ? 'connecting'
    : status.state === 'running'
      ? 'live'
      : status.state === 'unavailable'
        ? 'unavailable'
        : status.state

  return (
    <section className="performance-page">
      <div className="performance-toolbar">
        <label><Search size={15} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Filter processes…" /></label>
        <select aria-label="Sample interval" value={intervalMs} disabled={running} onChange={(event) => {
          const next = Number(event.target.value)
          setIntervalMs(next)
          setStatus((current) => ({ ...current, intervalMs: next }))
        }}>
          <option value={500}>500 ms</option>
          <option value={1000}>1 second</option>
          <option value={2000}>2 seconds</option>
        </select>
        <button onClick={() => void (running ? pause() : resume())} disabled={status.state === 'unavailable'}>
          {running ? <Pause size={13} /> : <Play size={13} />}{running ? 'Pause' : 'Resume'}
        </button>
        <button onClick={() => void exportSelected()} disabled={!exportRows.length}><Download size={13} />Export CSV</button>
        <small className={status.state}><i />{statusLabel}</small>
      </div>

      {(status.state === 'error' || status.state === 'unavailable') && (
        <div className={status.state === 'error' ? 'process-error' : 'process-notice'} role="status">
          {status.message ?? 'Performance sampling is unavailable.'}
        </div>
      )}

      <div className="performance-summary">
        <div className="card"><small>System CPU</small><b>{percent(latest?.systemCpuPercent ?? null)}</b><span>latest sample</span></div>
        <div className="card"><small>Processes</small><b>{latest?.processes.length ?? '—'}</b><span>top CPU and memory consumers</span></div>
        <div className="card"><small>Interval</small><b>{intervalMs < 1000 ? `${intervalMs} ms` : `${intervalMs / 1000} s`}</b><span>{history.length} / {MAX_HISTORY} samples retained</span></div>
        <div className="card"><small>Transport</small><b>{latest ? 'Sysmontap' : '—'}</b><span title={status.transport ?? latest?.transport}>{status.transport ?? latest?.transport ?? 'Connecting…'}</span></div>
      </div>

      <div className="performance-layout">
        <div className="performance-table card">
          <header><span>Process</span><span>PID</span><span>CPU</span><span>Memory</span></header>
          <div>
            {shown.map((process) => (
              <button key={process.identity} className={process.identity === selectedIdentity ? 'performance-row selected' : 'performance-row'} onClick={() => setSelectedIdentity(process.identity)}>
                <span><b>{process.name}</b><small>{process.identity}</small></span>
                <code>{process.pid}</code>
                <strong>{percent(process.cpuPercent)}</strong>
                <strong>{bytes(process.memoryBytes)}</strong>
              </button>
            ))}
            {!shown.length && <div className="process-empty">{latest ? 'No process matches this filter.' : 'Waiting for the first performance sample…'}</div>}
          </div>
          <footer><span>{shown.length} / {latest?.processes.length ?? 0} processes</span><span>history is bounded in memory</span></footer>
        </div>

        <aside className="performance-detail card">
          {selected ? <>
            <header><div><small>Selected process</small><h3>{selected.name}</h3><code>pid {selected.pid}</code></div><span className={selectedLatest ? 'active' : 'exited'}>{selectedLatest ? 'running' : 'exited'}</span></header>
            <div className="performance-metric-heading"><span><small>CPU</small><b>{percent(selectedLatest?.cpuPercent ?? null)}</b></span><em>{selectedHistory.length} samples</em></div>
            <Sparkline values={cpuValues} color="var(--accent)" ceiling={100} />
            <div className="performance-metric-heading"><span><small>Memory footprint</small><b>{bytes(selectedLatest?.memoryBytes ?? null)}</b></span><em>MiB</em></div>
            <Sparkline values={memoryValues} color="#b69cff" ceiling={64} />
            <p>History follows the process identity, not only its PID, so PID reuse starts a new series. Missing device fields remain unavailable instead of appearing as zero.</p>
          </> : <div className="performance-detail-empty"><b>Select a process</b><p>Choose a row to inspect its rolling CPU and memory history.</p></div>}
        </aside>
      </div>
    </section>
  )
}
