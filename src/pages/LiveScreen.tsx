import { useEffect, useRef, useState } from 'react'
import { CircleStop, Download, Expand, Minimize2, Play, ScanLine } from 'lucide-react'
import {
  api,
  dialogs,
  errorMessage,
  events,
  type LiveScreenFrame,
  type LiveScreenStatus,
} from '../api'
import { bytes } from '../lib/format'
import { useDesktopListeners, useMountedRef } from '../lib/hooks'

const TARGET_FPS = 2

const idleStatus = (): LiveScreenStatus => ({
  state: 'idle',
  message: null,
  transport: null,
  targetFps: TARGET_FPS,
})

const frameFilename = () => {
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-').replace('T', '-').replace('Z', '')
  return `live-screen-${timestamp}.png`
}

const demoDataUrl = (sequence: number) => {
  const minutes = String(Math.floor(sequence / 120) % 60).padStart(2, '0')
  const seconds = String(Math.floor(sequence / 2) % 60).padStart(2, '0')
  const pulse = 40 + Math.round(Math.sin(sequence / 3) * 18)
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="390" height="844" viewBox="0 0 390 844">
    <defs><linearGradient id="bg" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#171d35"/><stop offset=".55" stop-color="#30235a"/><stop offset="1" stop-color="#102d43"/></linearGradient></defs>
    <rect width="390" height="844" rx="42" fill="url(#bg)"/>
    <rect x="142" y="12" width="106" height="30" rx="15" fill="#05070d"/>
    <text x="24" y="35" fill="#fff" font-family="-apple-system,sans-serif" font-size="13" font-weight="600">9:41</text>
    <text x="195" y="168" text-anchor="middle" fill="#fff" font-family="-apple-system,sans-serif" font-size="64" font-weight="200">${minutes}:${seconds}</text>
    <text x="195" y="199" text-anchor="middle" fill="#b9c4e8" font-family="-apple-system,sans-serif" font-size="15">Live Screen demonstration</text>
    <circle cx="195" cy="390" r="92" fill="#7c6cff" opacity=".${pulse}"/>
    <circle cx="195" cy="390" r="58" fill="#11162c" stroke="#b0a7ff" stroke-width="2"/>
    <path d="M174 390h42M195 369v42" stroke="#d7d3ff" stroke-width="5" stroke-linecap="round"/>
    <rect x="24" y="650" width="342" height="142" rx="28" fill="#fff" opacity=".09"/>
    <text x="48" y="692" fill="#fff" font-family="-apple-system,sans-serif" font-size="18" font-weight="600">Device preview</text>
    <text x="48" y="722" fill="#c5cbe0" font-family="-apple-system,sans-serif" font-size="13">PNG refresh · target 2 FPS</text>
    <text x="48" y="754" fill="#8e98bc" font-family="ui-monospace,monospace" font-size="11">FRAME ${sequence.toString().padStart(4, '0')}</text>
    <rect x="130" y="825" width="130" height="5" rx="3" fill="#fff" opacity=".8"/>
  </svg>`
  return `data:image/svg+xml;charset=UTF-8,${encodeURIComponent(svg)}`
}

const demoFrame = (sequence: number): LiveScreenFrame => ({
  sequence,
  timestampMs: Date.now(),
  width: 390,
  height: 844,
  bytes: 38_400 + sequence * 17,
  fps: sequence > 1 ? 2 : 0,
  dataUrl: demoDataUrl(sequence),
})

const isActive = (state: string) => state === 'connecting' || state === 'running'

export function LiveScreen({ desktop, udid, onToast }: { desktop: boolean; udid: string; onToast: (message: string) => void }) {
  const [status, setStatus] = useState<LiveScreenStatus>(idleStatus)
  const [frame, setFrame] = useState<LiveScreenFrame | null>(null)
  const [fit, setFit] = useState(true)
  const [listenersReady, setListenersReady] = useState(!desktop)
  const mountedRef = useMountedRef()
  const ownsSessionRef = useRef(false)
  const demoSequenceRef = useRef(0)

  useDesktopListeners(
    desktop,
    async (alive) => {
      try {
        const [stopFrame, stopStatus] = await Promise.all([
          events.liveScreenFrame((next) => {
            if (alive()) setFrame(next)
          }),
          events.liveScreenStatus((next) => {
            if (!alive()) return
            if (next.state === 'error') {
              ownsSessionRef.current = false
              if (next.message) onToast(next.message)
            }
            if (next.state === 'stopped') ownsSessionRef.current = false
            setStatus(next.state === 'stopped' ? { ...next, state: 'paused' } : next)
          }),
        ])
        if (!alive()) {
          stopFrame()
          stopStatus()
          return []
        }
        setListenersReady(true)
        return [stopFrame, stopStatus]
      } catch (error) {
        if (alive()) onToast(errorMessage(error))
        return []
      }
    },
    () => {
      if (ownsSessionRef.current) {
        ownsSessionRef.current = false
        void api.liveScreenStop()
      }
    },
    [onToast],
  )

  useEffect(() => {
    if (desktop) return
    return () => { ownsSessionRef.current = false }
  }, [desktop])

  useEffect(() => {
    if (desktop || status.state !== 'running') return
    const timer = window.setInterval(() => {
      demoSequenceRef.current += 1
      setFrame(demoFrame(demoSequenceRef.current))
    }, 500)
    return () => window.clearInterval(timer)
  }, [desktop, status.state])

  useEffect(() => {
    const pauseWhenHidden = () => {
      if (!document.hidden || !ownsSessionRef.current) return
      ownsSessionRef.current = false
      setStatus((current) => ({
        ...current,
        state: 'paused',
        message: 'Preview paused while the window was hidden.',
      }))
      if (desktop) void api.liveScreenStop().catch((error) => onToast(errorMessage(error)))
    }
    document.addEventListener('visibilitychange', pauseWhenHidden)
    return () => document.removeEventListener('visibilitychange', pauseWhenHidden)
  }, [desktop, onToast])

  const start = async () => {
    ownsSessionRef.current = true
    setFrame(null)
    setStatus({ state: desktop ? 'connecting' : 'running', message: null, transport: desktop ? null : 'Screenshot refresh · demonstration', targetFps: TARGET_FPS })
    try {
      if (desktop) await api.liveScreenStart(udid)
      else {
        demoSequenceRef.current = 1
        setFrame(demoFrame(1))
      }
    } catch (error) {
      ownsSessionRef.current = false
      const message = errorMessage(error)
      if (mountedRef.current) setStatus((current) => ({ ...current, state: 'error', message }))
      onToast(message)
    }
  }

  const stop = async () => {
    if (!isActive(status.state)) return
    ownsSessionRef.current = false
    try {
      if (desktop) await api.liveScreenStop()
      if (mountedRef.current) setStatus((current) => ({ ...current, state: 'paused', message: null }))
    } catch (error) {
      ownsSessionRef.current = true
      const message = errorMessage(error)
      if (mountedRef.current) setStatus((current) => ({ ...current, state: 'error', message }))
      onToast(message)
    }
  }

  const saveFrame = async () => {
    if (!frame) return
    const filename = frameFilename()
    try {
      if (desktop) {
        const destination = await dialogs.pngDestination(filename)
        if (!destination) return
        await api.liveScreenExportFrame(destination)
        onToast('Current frame saved')
      } else {
        onToast(`${filename} ready in demonstration mode`)
      }
    } catch (error) {
      onToast(errorMessage(error))
    }
  }

  const label = status.state === 'idle' ? 'ready' : status.state

  return (
    <section className="live-screen-page">
      <div className="live-screen-toolbar">
        {!isActive(status.state) && (
          <button className="primary-button" disabled={!listenersReady} onClick={() => void start()}>
            <Play size={14} />Start Preview
          </button>
        )}
        {isActive(status.state) && (
          <button className="primary-button" onClick={() => void stop()}>
            <CircleStop size={14} />Stop Preview
          </button>
        )}
        <button disabled={!frame} onClick={() => void saveFrame()}><Download size={14} />Save Frame</button>
        <div className="live-screen-scale" aria-label="Preview scale">
          <button className={fit ? 'active' : ''} onClick={() => setFit(true)} aria-label="Fit to window"><Minimize2 size={13} />Fit</button>
          <button className={!fit ? 'active' : ''} onClick={() => setFit(false)} aria-label="Actual size"><Expand size={13} />100%</button>
        </div>
        <small className={status.state}><i />{label}</small>
      </div>

      {status.state === 'error' && <div className="process-error" role="status">{status.message ?? 'Live Screen failed.'}</div>}
      {status.state === 'paused' && status.message && <div className="process-notice" role="status">{status.message}</div>}

      <div className="live-screen-layout">
        <div className={`card live-screen-stage ${fit ? 'fit' : 'actual'}`}>
          {frame ? (
            <img src={frame.dataUrl} width={frame.width} height={frame.height} alt="Live device screen" />
          ) : (
            <div className="live-screen-empty">
              <span><ScanLine size={34} /></span>
              <b>{status.state === 'connecting' ? 'Opening device preview…' : 'Live Screen is ready'}</b>
              <p>Start the preview to refresh the device display at up to {status.targetFps} PNG frames per second.</p>
            </div>
          )}
        </div>

        <aside className="card live-screen-inspector">
          <header><div><small>Preview session</small><h3>{frame ? `${frame.width} × ${frame.height}` : 'Waiting for a frame'}</h3></div><span className={status.state}>{label}</span></header>
          <dl>
            <div><dt>Measured FPS</dt><dd>{frame ? frame.fps.toFixed(1) : '—'}</dd></div>
            <div><dt>Target FPS</dt><dd>{status.targetFps}</dd></div>
            <div><dt>Frame size</dt><dd>{frame ? bytes(frame.bytes) : '—'}</dd></div>
            <div><dt>Frame number</dt><dd>{frame ? frame.sequence.toLocaleString() : '—'}</dd></div>
          </dl>
          <div className="live-screen-route"><small>Transport</small><code>{status.transport ?? 'Not connected'}</code></div>
          <div className="live-screen-note">
            <b>PNG refresh preview</b>
            <p>This first release favors compatibility and still-frame accuracy. It is intended for inspection, not smooth video or remote control.</p>
          </div>
          <p className="live-screen-hint">The session stops automatically when you leave this page, switch devices, or hide the window.</p>
        </aside>
      </div>
    </section>
  )
}
