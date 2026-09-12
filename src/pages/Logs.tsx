import { createSessionId } from '../lib/session'
import { useEffect, useMemo, useRef, useState } from 'react'
import { Pause, Play, Search, X } from 'lucide-react'
import { initialLogs, liveLogPool, type LogLine } from '../data'
import { api, errorMessage, events } from '../api'

export function Logs({ connected, desktop, udid, onError }: { connected: boolean; desktop: boolean; udid: string; onError: (message: string) => void }) {
  const [logs, setLogs] = useState<LogLine[]>(desktop ? [] : initialLogs)
  const [filter, setFilter] = useState<'all' | 'error' | 'warn' | 'info' | 'debug'>('all')
  const [query, setQuery] = useState('')
  const [useRegex, setUseRegex] = useState(false)
  const [paused, setPaused] = useState(false)
  const [streamState, setStreamState] = useState(desktop ? 'stopped' : 'live')
  const pausedRef = useRef(paused)
  const scrollRef = useRef<HTMLDivElement>(null)
  useEffect(() => { pausedRef.current = paused }, [paused])
  useEffect(() => {
    if (!desktop || !connected) return
    let disposed = false
    let startIssued = false
    const sessionId = createSessionId()
    const stops: (() => void)[] = []
    setLogs([])
    setStreamState('starting')
    const stopSession = () => api.logsStop(sessionId).catch((error) => {
      if (!disposed) onError(errorMessage(error))
    })
    const subscribe = async (subscription: Promise<() => void>) => {
      const stop = await subscription
      if (disposed) stop()
      else stops.push(stop)
    }
    void (async () => {
      try {
        await Promise.all([
          subscribe(events.logLine((line) => {
            if (disposed || pausedRef.current || line.sessionId !== sessionId || line.udid !== udid) return
            const rawLevel = line.level.toUpperCase()
            const level: LogLine['level'] = rawLevel === 'FAULT' ? 'ERROR' : ['INFO', 'DEBUG', 'NOTICE', 'WARN', 'ERROR'].includes(rawLevel) ? rawLevel as LogLine['level'] : 'INFO'
            setLogs((items) => [...items, { time: line.timestamp, level, process: `${line.process}[${line.pid}]`, message: line.message }].slice(-1000))
          })),
          subscribe(events.logStatus((status) => {
            if (disposed || status.sessionId !== sessionId || status.udid !== udid) return
            setStreamState(status.state === 'running' ? 'live' : status.state)
            if (status.state === 'error' && status.message) onError(status.message)
          })),
        ])
        if (disposed) return
        startIssued = true
        await api.logsStart(sessionId, udid)
      } catch (error) {
        if (!disposed) {
          setStreamState('error')
          onError(errorMessage(error))
        }
      } finally {
        // A stop sent during setup may reach Rust before start has registered.
        if (disposed && startIssued) await stopSession()
      }
    })()
    return () => {
      disposed = true
      stops.forEach((stop) => stop())
      if (startIssued) void stopSession()
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
      <div className="log-toolbar">{(['all', 'error', 'warn', 'info', 'debug'] as const).map((item) => <button key={item} className={filter === item ? 'active' : ''} onClick={() => setFilter(item)}>{item[0].toUpperCase() + item.slice(1)}</button>)}<span className="toolbar-spacer" /><small className={paused ? 'paused' : ''}><i />{!connected ? 'stopped' : desktop && streamState !== 'live' ? streamState : paused ? 'paused' : 'live'}</small><button onClick={() => setPaused((value) => !value)}>{paused ? <Play size={13} /> : <Pause size={13} />}{paused ? 'Resume' : 'Pause'}</button><button onClick={() => setLogs([])}>Clear</button></div>
      <div className={`log-search ${regexResult.error ? 'invalid' : ''}`}>
        <label><Search size={15} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder={useRegex ? 'Filter logs with a regular expression…' : 'Filter logs…'} aria-invalid={Boolean(regexResult.error)} />{query && <button className="log-search-clear" onClick={() => setQuery('')} aria-label="Clear log filter"><X size={14} /></button>}</label>
        <button className={useRegex ? 'active' : ''} onClick={() => setUseRegex((value) => !value)} aria-pressed={useRegex} title="Use regular expression matching"><code>.*</code> Regex</button>
        <small title={regexResult.error || undefined}>{regexResult.error ? 'Invalid regular expression' : `${shown.length} / ${logs.length}`}</small>
      </div>
      <div className="log-console" ref={scrollRef}>{shown.map((line, index) => <div key={`${line.time}-${index}`}><time>{line.time}</time><b className={`level-${line.level.toLowerCase()}`}>{line.level}</b><code>{line.process}</code><span>{line.message}</span></div>)}<div className="terminal-cursor"><code>›</code><i /></div></div>
    </section>
  )
}
