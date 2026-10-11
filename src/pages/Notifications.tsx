import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { BellRing, Pause, Play, Plus, Search, Trash2 } from 'lucide-react'
import {
  api,
  errorMessage,
  events,
  type NotificationObservationEvent,
  type NotificationObservationStatus,
} from '../api'
import { createSessionId } from '../lib/session'
import { on, useDeviceEvents } from '../lib/useDeviceEvents'
import { useInterval } from '../lib/useInterval'

const MAX_HISTORY = 500
const MAX_SUBSCRIPTIONS = 32

const PRESETS = [
  { name: 'com.apple.mobile.application_installed', label: 'Application installed', detail: 'A device application was installed.' },
  { name: 'com.apple.mobile.application_uninstalled', label: 'Application uninstalled', detail: 'A device application was removed.' },
  { name: 'com.apple.mobile.lockdown.device_name_changed', label: 'Device name changed', detail: 'The device name changed in Settings.' },
  { name: 'com.apple.mobile.lockdown.timezone_changed', label: 'Timezone changed', detail: 'The device timezone changed.' },
  { name: 'com.apple.mobile.lockdown.disk_usage_changed', label: 'Disk usage changed', detail: 'The device reported a storage change.' },
  { name: 'com.apple.mobile.lockdown.activation_state', label: 'Activation state', detail: 'The activation state changed.' },
  { name: 'com.apple.mobile.lockdown.trusted_host_attached', label: 'Trusted host attached', detail: 'A trusted host connected to the device.' },
  { name: 'com.apple.mobile.lockdown.host_detached', label: 'Host detached', detail: 'A host disconnected from the device.' },
] as const

const time = (timestampMs: number) => {
  const date = new Date(timestampMs)
  const clock = date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })
  return `${clock}.${String(date.getMilliseconds()).padStart(3, '0')}`
}

export function Notifications({
  connected,
  desktop,
  udid,
  onToast,
}: {
  connected: boolean
  desktop: boolean
  udid: string
  onToast: (message: string) => void
}) {
  const [selected, setSelected] = useState<Set<string>>(() => new Set())
  const [customNames, setCustomNames] = useState<string[]>([])
  const [customName, setCustomName] = useState('')
  const [history, setHistory] = useState<NotificationObservationEvent[]>([])
  const [query, setQuery] = useState('')
  const [status, setStatus] = useState<NotificationObservationStatus>({
    sessionId: '',
    state: 'idle',
    message: null,
    transport: null,
    subscriptions: [],
  })
  const mountedRef = useRef(true)
  const requestedRef = useRef(false)
  const activeSessionRef = useRef('')
  const demoSubscriptionsRef = useRef<string[]>([])
  const demoSequenceRef = useRef(0)

  useEffect(() => {
    mountedRef.current = true
    return () => { mountedRef.current = false }
  }, [])

  const appendEvent = useCallback((event: NotificationObservationEvent) => {
    if (!mountedRef.current || event.sessionId !== activeSessionRef.current) return
    setHistory((current) => [event, ...current].slice(0, MAX_HISTORY))
  }, [])

  const listenersReady = useDeviceEvents(desktop, () => ({
    listeners: [
      on(events.notificationObservationEvent, appendEvent),
      on(events.notificationObservationStatus, (next) => {
        if (next.sessionId !== activeSessionRef.current) return
        if (next.state === 'error') requestedRef.current = false
        const state = next.state === 'stopped' && !requestedRef.current ? 'paused' : next.state
        setStatus({ ...next, state })
        if (next.state === 'error' && next.message) onToast(next.message)
      }),
    ],
    stop: () => { if (requestedRef.current) void api.notificationObservationStop() },
  }), (message) => {
    setStatus((current) => ({ ...current, state: 'error', message }))
    onToast(message)
  }, [appendEvent, udid])

  useInterval(!desktop && status.state === 'running', 1_600, () => {
    const subscriptions = demoSubscriptionsRef.current
    if (!subscriptions.length) return
    demoSequenceRef.current += 1
    appendEvent({
      sessionId: activeSessionRef.current,
      sequence: demoSequenceRef.current,
      timestampMs: Date.now(),
      name: subscriptions[(demoSequenceRef.current - 1) % subscriptions.length],
    })
  })

  const active = status.state === 'connecting' || status.state === 'running'
  const subscriptions = useMemo(() => Array.from(selected), [selected])
  const shown = useMemo(() => {
    const needle = query.trim().toLowerCase()
    return needle ? history.filter((event) => event.name.toLowerCase().includes(needle)) : history
  }, [history, query])
  const uniqueCount = useMemo(() => new Set(history.map((event) => event.name)).size, [history])

  const toggle = (name: string) => {
    if (active) return
    setSelected((current) => {
      const next = new Set(current)
      if (next.has(name)) next.delete(name)
      else if (next.size < MAX_SUBSCRIPTIONS) next.add(name)
      return next
    })
  }

  const addCustom = () => {
    const name = customName.trim()
    if (!name) return
    // eslint-disable-next-line no-control-regex -- control characters are exactly what is rejected
    if (new TextEncoder().encode(name).length > 200 || /[\u0000-\u001f\u007f]/.test(name)) {
      onToast('Notification names must be at most 200 visible bytes')
      return
    }
    if (selected.size >= MAX_SUBSCRIPTIONS && !selected.has(name)) {
      onToast('Listen to at most 32 notification names at once')
      return
    }
    if (!PRESETS.some((preset) => preset.name === name)) {
      setCustomNames((current) => current.includes(name) ? current : [...current, name])
    }
    setSelected((current) => new Set(current).add(name))
    setCustomName('')
  }

  const start = async () => {
    if (!subscriptions.length || active) return
    if (desktop && !connected) {
      onToast('Connect a device before listening for notifications')
      return
    }
    const sessionId = createSessionId()
    activeSessionRef.current = sessionId
    requestedRef.current = true
    demoSubscriptionsRef.current = subscriptions
    setStatus({ sessionId, state: 'connecting', message: null, transport: null, subscriptions })
    try {
      if (desktop) {
        await api.notificationObservationStart(subscriptions, sessionId, udid)
      } else if (mountedRef.current) {
        setStatus({
          sessionId,
          state: 'running',
          message: null,
          transport: 'Notification Proxy · demonstration',
          subscriptions,
        })
      }
    } catch (error) {
      if (activeSessionRef.current !== sessionId) return
      requestedRef.current = false
      const message = errorMessage(error)
      setStatus({ sessionId, state: 'error', message, transport: null, subscriptions })
      onToast(message)
    }
  }

  const pause = async () => {
    if (!active) return
    requestedRef.current = false
    try {
      if (desktop) await api.notificationObservationStop()
      if (mountedRef.current) setStatus((current) => ({ ...current, state: 'paused', message: null }))
    } catch (error) {
      requestedRef.current = true
      onToast(errorMessage(error))
    }
  }

  const statusLabel = status.state === 'running'
    ? 'listening'
    : status.state === 'connecting'
      ? 'connecting'
      : status.state === 'paused'
        ? 'paused'
        : status.state

  return (
    <section className="notifications-page">
      <div className="notifications-toolbar">
        {active
          ? <button className="primary-button" onClick={() => void pause()}><Pause size={14} /> Pause</button>
          : <button
              className="primary-button"
              disabled={!subscriptions.length || !listenersReady || (desktop && !connected)}
              onClick={() => void start()}
            ><Play size={14} /> Start Listening</button>}
        <button disabled={!history.length} onClick={() => setHistory([])}><Trash2 size={14} /> Clear History</button>
        <span />
        <small className={status.state}><i />{statusLabel}</small>
      </div>

      {status.state === 'error' && <div className="process-error" role="alert">{status.message ?? 'Notification observation failed.'}</div>}

      <div className="notifications-summary">
        <div className="card"><small>Events retained</small><b>{history.length}</b><span>latest {MAX_HISTORY} maximum</span></div>
        <div className="card"><small>Unique names</small><b>{uniqueCount}</b><span>in retained history</span></div>
        <div className="card"><small>Subscriptions</small><b>{status.subscriptions.length || subscriptions.length}</b><span>{active ? 'locked while listening' : 'chosen explicitly'}</span></div>
        <div className="card"><small>Transport</small><b>{status.transport ? 'Connected' : '—'}</b><span>{status.transport ?? 'not connected'}</span></div>
      </div>

      <div className="notifications-layout">
        <aside className="notifications-subscriptions card">
          <header>
            <div><small>Subscription list</small><h3>Choose notification names</h3></div>
            <span>{subscriptions.length}/{MAX_SUBSCRIPTIONS}</span>
          </header>
          <div className="notifications-presets">
            {PRESETS.map((preset) => (
              <label key={preset.name}>
                <input type="checkbox" checked={selected.has(preset.name)} disabled={active} onChange={() => toggle(preset.name)} />
                <span><b>{preset.label}</b><small>{preset.detail}</small><code>{preset.name}</code></span>
              </label>
            ))}
            {customNames.map((name) => (
              <label key={name}>
                <input type="checkbox" checked={selected.has(name)} disabled={active} onChange={() => toggle(name)} />
                <span><b>Custom notification</b><code>{name}</code></span>
              </label>
            ))}
          </div>
          <form className="notifications-custom" onSubmit={(event) => { event.preventDefault(); addCustom() }}>
            <label htmlFor="custom-notification-name">Custom notification name</label>
            <div>
              <input
                id="custom-notification-name"
                value={customName}
                disabled={active}
                onChange={(event) => setCustomName(event.target.value)}
                placeholder="com.example.notification"
              />
              <button type="submit" aria-label="Add custom notification" disabled={active || !customName.trim()}><Plus size={14} /></button>
            </div>
          </form>
        </aside>

        <div className="notifications-events card">
          <header>
            <div><BellRing size={16} /><span><small>Observed events</small><b>Notification timeline</b></span></div>
            <label><Search size={14} /><input aria-label="Filter notifications" value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Filter names" /></label>
          </header>
          <div className="notifications-event-heading"><span>Time</span><span>Notification name</span><span>Sequence</span></div>
          <div className="notifications-event-list">
            {shown.map((event) => (
              <div className="notifications-event-row" key={`${event.sessionId}-${event.sequence}`}>
                <time dateTime={new Date(event.timestampMs).toISOString()}>{time(event.timestampMs)}</time>
                <code>{event.name}</code>
                <span>#{event.sequence}</span>
              </div>
            ))}
            {!shown.length && <div className="notifications-empty">
              <BellRing size={24} />
              <b>{history.length ? 'No events match this filter' : 'No notification events yet'}</b>
              <p>{active ? 'The listener is ready. Matching device events will appear here.' : 'Select one or more names, then start listening.'}</p>
            </div>}
          </div>
          <footer>
            <span>{shown.length} shown</span>
            <span>Newest first · auto-stops on page exit</span>
          </footer>
        </div>
      </div>

      <div className="notifications-readonly">
        This is a read-only observer. The service relays notification names only, so no payload is shown, and this application never posts notifications to the device.
      </div>
    </section>
  )
}
