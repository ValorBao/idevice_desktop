import { useEffect, useMemo, useRef, useState } from 'react'
import {
  CircleStop,
  FolderOutput,
  Network,
  Play,
  ShieldAlert,
  Trash2,
} from 'lucide-react'
import {
  api,
  dialogs,
  errorMessage,
  events,
  type NetworkCaptureFilter,
  type NetworkCaptureProgress,
  type NetworkCaptureStatus,
} from '../api'
import { byteSize, fileName } from '../lib/format'
import { on, useDeviceEvents } from '../lib/useDeviceEvents'
import { useInterval } from '../lib/useInterval'

const emptyProgress = (outputBytes = 0): NetworkCaptureProgress => ({
  packets: 0,
  bytes: 0,
  outputBytes,
  elapsedMs: 0,
  lastProcess: null,
  lastInterface: null,
})

const idleStatus = (): NetworkCaptureStatus => ({
  state: 'idle',
  message: null,
  destination: '',
  transport: null,
  filter: { pid: null, interfaceName: null },
})

const duration = (milliseconds: number) => {
  const totalSeconds = Math.floor(milliseconds / 1000)
  const hours = Math.floor(totalSeconds / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)
  const seconds = totalSeconds % 60
  return hours
    ? `${hours}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`
    : `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`
}

const captureFilename = () => {
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-').replace('T', '-').replace('Z', '')
  return `idevice-capture-${timestamp}.pcap`
}

const activeState = (state: string) => ['connecting', 'running', 'stopping', 'cancelling'].includes(state)

export function NetworkCapture({ desktop, udid, onToast }: { desktop: boolean; udid: string; onToast: (message: string) => void }) {
  const [status, setStatus] = useState<NetworkCaptureStatus>(idleStatus)
  const [progress, setProgress] = useState<NetworkCaptureProgress>(emptyProgress)
  const [pid, setPid] = useState('')
  const [interfaceName, setInterfaceName] = useState('')
  const mountedRef = useRef(true)
  const ownsCaptureRef = useRef(false)
  const demoStartedRef = useRef(0)

  useEffect(() => {
    mountedRef.current = true
    return () => { mountedRef.current = false }
  }, [])

  const listenersReady = useDeviceEvents(desktop, () => ({
    listeners: [
      on(events.networkCaptureProgress, setProgress),
      on(events.networkCaptureStatus, (next) => {
        setStatus((current) => ({ ...next, transport: next.transport ?? current.transport }))
        if (next.state === 'completed') {
          ownsCaptureRef.current = false
          onToast(next.message ?? `${fileName(next.destination)} saved`)
        } else if (next.state === 'cancelled') {
          ownsCaptureRef.current = false
          onToast('Capture discarded')
        } else if (next.state === 'error') {
          ownsCaptureRef.current = false
          if (next.message) onToast(next.message)
        } else if (next.state === 'stopping' || next.state === 'cancelling') {
          ownsCaptureRef.current = false
        }
      }),
    ],
    stop: () => {
      if (ownsCaptureRef.current) {
        ownsCaptureRef.current = false
        void api.networkCaptureCancel()
      }
    },
  }), onToast, [])

  useInterval(!desktop && status.state === 'running', 500, () => {
    setProgress((current) => {
      const packetIncrease = 4 + Math.floor(Math.random() * 8)
      const byteIncrease = packetIncrease * (320 + Math.floor(Math.random() * 900))
      return {
        packets: current.packets + packetIncrease,
        bytes: current.bytes + byteIncrease,
        outputBytes: current.outputBytes + byteIncrease + packetIncrease * 16,
        elapsedMs: Date.now() - demoStartedRef.current,
        lastProcess: ['MobileSafari', 'AppStore', 'apsd'][current.packets % 3],
        lastInterface: ['en0', 'pdp_ip0'][current.packets % 2],
      }
    })
  })

  const filter = useMemo<NetworkCaptureFilter>(() => ({
    pid: pid.trim() ? Number(pid) : null,
    interfaceName: interfaceName.trim() || null,
  }), [interfaceName, pid])
  const active = activeState(status.state)
  const captureLabel = status.state === 'connecting'
    ? 'connecting'
    : status.state === 'running'
      ? 'capturing'
      : status.state

  const start = async () => {
    const parsedPid = pid.trim() ? Number(pid) : null
    if (parsedPid !== null && (!Number.isInteger(parsedPid) || parsedPid <= 0 || parsedPid > 0xffff_ffff)) {
      onToast('PID must be a whole number greater than zero')
      return
    }
    const filename = captureFilename()
    try {
      const destination = desktop ? await dialogs.pcapDestination(filename) : `/Downloads/${filename}`
      if (!destination || !mountedRef.current) return
      const nextFilter = { pid: parsedPid, interfaceName: interfaceName.trim() || null }
      setProgress(emptyProgress(24))
      setStatus({ state: 'connecting', message: null, destination, transport: null, filter: nextFilter })
      ownsCaptureRef.current = true
      if (desktop) {
        await api.networkCaptureStart(destination, udid, parsedPid ?? undefined, nextFilter.interfaceName ?? undefined)
      } else {
        demoStartedRef.current = Date.now()
        setStatus({
          state: 'running',
          message: null,
          destination,
          transport: 'Pcapd · demonstration',
          filter: nextFilter,
        })
      }
    } catch (error) {
      ownsCaptureRef.current = false
      const message = errorMessage(error)
      if (mountedRef.current) setStatus((current) => ({ ...current, state: 'error', message }))
      onToast(message)
    }
  }

  const stop = async () => {
    if (status.state !== 'running' && status.state !== 'connecting') return
    setStatus((current) => ({ ...current, state: 'stopping', message: null }))
    try {
      if (desktop) {
        await api.networkCaptureStop()
        ownsCaptureRef.current = false
      } else {
        ownsCaptureRef.current = false
        setStatus((current) => ({ ...current, state: 'completed' }))
        onToast(`${fileName(status.destination)} saved in demonstration mode`)
      }
    } catch (error) {
      const message = errorMessage(error)
      if (mountedRef.current) setStatus((current) => ({ ...current, state: 'error', message }))
      onToast(message)
    }
  }

  const cancel = async () => {
    if (!active) return
    setStatus((current) => ({ ...current, state: 'cancelling', message: null }))
    try {
      if (desktop) {
        await api.networkCaptureCancel()
        ownsCaptureRef.current = false
      } else {
        ownsCaptureRef.current = false
        setStatus((current) => ({ ...current, state: 'cancelled' }))
        onToast('Capture discarded')
      }
    } catch (error) {
      const message = errorMessage(error)
      if (mountedRef.current) setStatus((current) => ({ ...current, state: 'error', message }))
      onToast(message)
    }
  }

  return (
    <section className="network-capture-page">
      <div className="network-capture-toolbar">
        {!active && (
          <button className="primary-button" disabled={!listenersReady} onClick={() => void start()}>
            <Play size={14} />Start Capture
          </button>
        )}
        {(status.state === 'running' || status.state === 'connecting') && (
          <button className="primary-button" onClick={() => void stop()}><CircleStop size={14} />Stop &amp; Save</button>
        )}
        {(status.state === 'running' || status.state === 'connecting') && <button className="danger-button" onClick={() => void cancel()}><Trash2 size={14} />Cancel &amp; Delete</button>}
        <span className="network-capture-destination" title={status.destination}>
          <FolderOutput size={14} />{status.destination ? fileName(status.destination) : 'Choose a destination when capture starts'}
        </span>
        <small className={status.state}><i />{captureLabel}</small>
      </div>

      {status.state === 'error' && <div className="process-error" role="status">{status.message ?? 'Network capture failed.'}</div>}
      {status.state === 'completed' && <div className="network-capture-success" role="status">Capture saved to <code>{status.destination}</code></div>}
      {status.state === 'cancelled' && <div className="process-notice" role="status">The partial capture was deleted.</div>}

      <div className="network-capture-summary">
        <div className="card"><small>Packets saved</small><b>{progress.packets.toLocaleString()}</b><span>after active filters</span></div>
        <div className="card"><small>Packet data</small><b>{byteSize(progress.bytes)}</b><span>captured frame bytes</span></div>
        <div className="card"><small>PCAP size</small><b>{byteSize(progress.outputBytes)}</b><span>2 GB safety limit</span></div>
        <div className="card"><small>Elapsed</small><b>{duration(progress.elapsedMs)}</b><span>{status.transport ?? 'not connected'}</span></div>
      </div>

      <div className="network-capture-layout">
        <div className={`network-capture-activity card ${status.state === 'running' ? 'running' : ''}`}>
          <div className="network-capture-visual"><span><Network size={28} /></span><i /><i /><i /></div>
          <div>
            <small>Capture state</small>
            <h3>{status.state === 'idle' ? 'Ready to capture device traffic' : captureLabel}</h3>
            <p>{active
              ? 'Packets are streamed directly to a temporary file. They are not retained in application memory.'
              : status.state === 'completed'
                ? 'The complete PCAP is ready to open in Wireshark or another packet analyzer.'
                : 'Start with all device traffic, or narrow the saved packets using the optional filters.'}</p>
            <dl>
              <div><dt>Last process</dt><dd>{progress.lastProcess ?? '—'}</dd></div>
              <div><dt>Last interface</dt><dd>{progress.lastInterface ?? '—'}</dd></div>
              <div><dt>Process filter</dt><dd>{status.filter.pid ?? filter.pid ?? 'All'}</dd></div>
              <div><dt>Interface filter</dt><dd>{status.filter.interfaceName ?? filter.interfaceName ?? 'All'}</dd></div>
            </dl>
          </div>
        </div>

        <aside className="network-capture-settings card">
          <header><div><small>Capture settings</small><h3>Optional filters</h3></div><span>Advanced</span></header>
          <label>Process PID<input inputMode="numeric" value={pid} disabled={active} onChange={(event) => setPid(event.target.value.replace(/\D/g, ''))} placeholder="All processes" /></label>
          <label>Interface<input value={interfaceName} disabled={active} maxLength={64} onChange={(event) => setInterfaceName(event.target.value)} placeholder="All interfaces, e.g. en0" /></label>
          <p>Filters decide which packets are written to the PCAP. Leave both empty to capture all traffic exposed by the device service.</p>
          <div className="network-capture-privacy"><ShieldAlert size={17} /><span><b>Packet contents may be sensitive.</b> Captures can contain addresses, hostnames, and unencrypted application data. Share the saved file only when necessary.</span></div>
        </aside>
      </div>
    </section>
  )
}
