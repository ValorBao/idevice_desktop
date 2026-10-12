import { useCallback, useEffect, useMemo, useRef, useState, type MouseEvent } from 'react'
import { CircleStop, Clapperboard, Download, Pause, Play, RotateCcw, Scissors, Trash2, Undo2, X } from 'lucide-react'
import {
  api,
  dialogs,
  errorMessage,
  events,
  localMediaUrl,
  type ScreenRecording,
  type ScreenRecordingStatus,
} from '../api'
import { on, useDeviceEvents } from '../lib/useDeviceEvents'
import { useInterval } from '../lib/useInterval'
import { MIN_SEGMENT_MS, formatClock, keptSegments, normalizeCuts, skipCut, totalDuration, type TimeRange } from '../lib/clipEdit'

type Phase = 'idle' | 'connecting' | 'recording' | 'stopping' | 'editing'

const megabytes = (value: number) => `${(value / 1024 / 1024).toFixed(1)} MB`

const exportFilename = () => {
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-').replace('T', '-').replace('Z', '')
  return `iphone-recording-${timestamp}.mp4`
}

const demoRecording = (durationMs: number, deviceName: string): ScreenRecording => ({
  id: 'demonstration',
  path: '',
  deviceName,
  durationMs: Math.max(durationMs, 3_000),
  width: 1_170,
  height: 2_532,
  bytes: Math.round(Math.max(durationMs, 3_000) * 820),
})

export function ScreenRecorder({ desktop, deviceName, onToast }: {
  desktop: boolean
  deviceName: string
  onToast: (message: string) => void
}) {
  const [phase, setPhase] = useState<Phase>('idle')
  const [recording, setRecording] = useState<ScreenRecording | null>(null)
  const [recordingDevice, setRecordingDevice] = useState<string | null>(null)
  const [startedAt, setStartedAt] = useState<number | null>(null)
  const [now, setNow] = useState(() => Date.now())
  const [error, setError] = useState<string | null>(null)
  const [cuts, setCuts] = useState<TimeRange[]>([])
  const [markIn, setMarkIn] = useState<number | null>(null)
  const [markOut, setMarkOut] = useState<number | null>(null)
  const [positionMs, setPositionMs] = useState(0)
  const [playing, setPlaying] = useState(false)
  const [exportProgress, setExportProgress] = useState<number | null>(null)
  const sessionRef = useRef<string | null>(null)
  const phaseRef = useRef<Phase>('idle')
  const videoRef = useRef<HTMLVideoElement | null>(null)
  const mountedRef = useRef(true)

  useEffect(() => {
    mountedRef.current = true
    return () => { mountedRef.current = false }
  }, [])

  const enterPhase = useCallback((next: Phase) => {
    phaseRef.current = next
    setPhase(next)
  }, [])

  const openEditor = useCallback((next: ScreenRecording) => {
    setRecording(next)
    setCuts([])
    setMarkIn(null)
    setMarkOut(null)
    setPositionMs(0)
    setPlaying(false)
    enterPhase('editing')
  }, [enterPhase])

  useEffect(() => {
    if (!desktop) return
    let alive = true
    api.screenRecordingCurrent()
      .then((current) => { if (alive && current && phaseRef.current === 'idle') openEditor(current) })
      .catch((reason) => onToast(errorMessage(reason)))
    return () => { alive = false }
  }, [desktop, onToast, openEditor])

  const handleStatus = (status: ScreenRecordingStatus) => {
    if (status.sessionId !== sessionRef.current) return
    if (status.state === 'recording') {
      setRecordingDevice(status.deviceName)
      setStartedAt(Date.now())
      enterPhase('recording')
    } else if (status.state === 'finished' && status.recording) {
      sessionRef.current = null
      openEditor(status.recording)
    } else if (status.state === 'cancelled') {
      sessionRef.current = null
      enterPhase('idle')
    } else if (status.state === 'error') {
      sessionRef.current = null
      setError(status.message)
      if (status.message) onToast(status.message)
      enterPhase('idle')
    }
  }

  const listenersReady = useDeviceEvents(desktop, () => ({
    listeners: [
      on(events.screenRecordingStatus, handleStatus),
      on(events.screenRecordingExportProgress, ({ progress }) => setExportProgress(progress)),
    ],
    stop: () => {
      if (phaseRef.current === 'recording') void api.screenRecordingStop().catch(() => undefined)
      else if (phaseRef.current === 'connecting') void api.screenRecordingCancel()
    },
  }), onToast, [])

  useInterval(phase === 'recording', 200, () => setNow(Date.now()))

  const elapsedMs = startedAt === null ? 0 : Math.max(0, now - startedAt)

  const start = async () => {
    if (recording && !(await dialogs.confirmDestructive(
      'Recording again deletes the current recording and its edits. Export it first if you want to keep it.',
      'Record Again',
    ))) return
    setError(null)
    setRecording(null)
    setRecordingDevice(null)
    setStartedAt(null)
    if (!desktop) {
      setRecordingDevice(deviceName)
      setStartedAt(Date.now())
      setNow(Date.now())
      enterPhase('recording')
      return
    }
    const sessionId = crypto.randomUUID()
    sessionRef.current = sessionId
    enterPhase('connecting')
    try {
      await api.screenRecordingStart(sessionId, deviceName)
    } catch (reason) {
      sessionRef.current = null
      const message = errorMessage(reason)
      setError(message)
      onToast(message)
      enterPhase('idle')
    }
  }

  const stop = async () => {
    if (!desktop) {
      openEditor(demoRecording(elapsedMs, deviceName))
      return
    }
    enterPhase('stopping')
    try {
      const finished = await api.screenRecordingStop()
      sessionRef.current = null
      if (mountedRef.current && phaseRef.current === 'stopping') openEditor(finished)
    } catch (reason) {
      sessionRef.current = null
      const message = errorMessage(reason)
      setError(message)
      onToast(message)
      if (mountedRef.current) enterPhase('idle')
    }
  }

  const cancel = async () => {
    sessionRef.current = null
    enterPhase('idle')
    if (desktop) await api.screenRecordingCancel().catch((reason) => onToast(errorMessage(reason)))
  }

  const discard = async () => {
    if (!(await dialogs.confirmDestructive('Delete this recording and its edits?', 'Delete Recording'))) return
    try {
      if (desktop) await api.screenRecordingDiscard()
      setRecording(null)
      setCuts([])
      enterPhase('idle')
    } catch (reason) {
      onToast(errorMessage(reason))
    }
  }

  const durationMs = recording?.durationMs ?? 0
  const normalizedCuts = useMemo(() => normalizeCuts(cuts, durationMs), [cuts, durationMs])
  const kept = useMemo(() => keptSegments(durationMs, cuts), [cuts, durationMs])
  const resultMs = totalDuration(kept)
  const selection = markIn !== null && markOut !== null && markOut - markIn >= MIN_SEGMENT_MS
    ? { startMs: markIn, endMs: markOut }
    : null

  const seek = (milliseconds: number) => {
    const clamped = Math.max(0, Math.min(milliseconds, durationMs))
    setPositionMs(clamped)
    if (videoRef.current) videoRef.current.currentTime = clamped / 1000
  }

  const seekFromTimeline = (event: MouseEvent<HTMLDivElement>) => {
    const bounds = event.currentTarget.getBoundingClientRect()
    seek(((event.clientX - bounds.left) / bounds.width) * durationMs)
  }

  useEffect(() => {
    if (!playing || !desktop) return
    let frame = 0
    const follow = () => {
      const video = videoRef.current
      if (video) {
        const current = video.currentTime * 1000
        const target = skipCut(current, normalizedCuts)
        if (target !== null) {
          if (target >= durationMs) {
            video.pause()
            setPlaying(false)
          } else {
            video.currentTime = target / 1000
          }
        }
        setPositionMs(target ?? current)
      }
      frame = requestAnimationFrame(follow)
    }
    frame = requestAnimationFrame(follow)
    return () => cancelAnimationFrame(frame)
  }, [playing, desktop, normalizedCuts, durationMs])

  const togglePlayback = () => {
    const video = videoRef.current
    if (!video) return
    if (playing) {
      video.pause()
      setPlaying(false)
      return
    }
    if (skipCut(video.currentTime * 1000, normalizedCuts) !== null || video.ended) {
      video.currentTime = (kept[0]?.startMs ?? 0) / 1000
    }
    void video.play().then(() => setPlaying(true)).catch((reason) => onToast(errorMessage(reason)))
  }

  const cutSelection = () => {
    if (!selection) return
    setCuts((current) => [...current, selection])
    setMarkIn(null)
    setMarkOut(null)
  }

  const exportVideo = async () => {
    if (!recording || kept.length === 0) return
    const filename = exportFilename()
    if (!desktop) {
      onToast(`${filename} ready in demonstration mode`)
      return
    }
    try {
      const destination = await dialogs.mp4Destination(filename)
      if (!destination) return
      setExportProgress(0)
      await api.screenRecordingExport(destination, kept)
      onToast('Video exported')
    } catch (reason) {
      onToast(errorMessage(reason))
    } finally {
      if (mountedRef.current) setExportProgress(null)
    }
  }

  const percent = (milliseconds: number) => `${durationMs > 0 ? (milliseconds / durationMs) * 100 : 0}%`
  const exporting = exportProgress !== null
  const label = phase === 'editing' ? 'ready to edit' : phase

  return (
    <section className="live-screen-page" data-testid="screen-recorder-page">
      <div className="live-screen-toolbar">
        {(phase === 'idle' || phase === 'editing') && (
          <button className="primary-button" disabled={!listenersReady || exporting} onClick={() => void start()}>
            <Clapperboard size={14} />{recording ? 'Record Again' : 'Start Recording'}
          </button>
        )}
        {phase === 'connecting' && (
          <button className="primary-button" onClick={() => void cancel()}><X size={14} />Cancel</button>
        )}
        {(phase === 'recording' || phase === 'stopping') && (
          <button className="primary-button" disabled={phase === 'stopping'} onClick={() => void stop()}>
            <CircleStop size={14} />Stop Recording
          </button>
        )}
        <button disabled={!recording || kept.length === 0 || exporting || phase !== 'editing'} onClick={() => void exportVideo()}>
          <Download size={14} />Export MP4
        </button>
        <button disabled={!recording || exporting || phase !== 'editing'} onClick={() => void discard()}>
          <Trash2 size={14} />Delete
        </button>
        <small className={phase === 'recording' ? 'running' : phase === 'connecting' || phase === 'stopping' ? 'connecting' : error ? 'error' : ''}>
          <i />{label}
        </small>
      </div>

      {error && phase === 'idle' && <div className="process-error" role="status">{error}</div>}

      <div className="live-screen-layout">
        <div className="card live-screen-stage fit recorder-stage">
          {phase === 'editing' && recording ? (
            desktop ? (
              <video
                ref={videoRef}
                src={localMediaUrl(recording.path)}
                preload="auto"
                onEnded={() => setPlaying(false)}
                onPause={() => setPlaying(false)}
                aria-label="Recorded device screen"
              />
            ) : (
              <div className="live-screen-empty">
                <span><Clapperboard size={34} /></span>
                <b>Demonstration recording</b>
                <p>Playback needs the desktop app. The timeline, cuts, and export plan work the same way.</p>
              </div>
            )
          ) : (
            <div className="live-screen-empty">
              <span><Clapperboard size={34} /></span>
              {phase === 'connecting' && <><b>Opening the iPhone screen…</b><p>Keep the iPhone unlocked with its screen on. The first picture takes about two seconds.</p></>}
              {(phase === 'recording' || phase === 'stopping') && <><b className="recorder-clock">{formatClock(elapsedMs)}</b><p>Recording {recordingDevice ?? deviceName}. Use the iPhone normally, then stop here.</p></>}
              {phase === 'idle' && <><b>Record the iPhone screen</b><p>Captures the screen and sound over USB at up to 60 FPS, the way QuickTime does. No developer disk image is needed.</p></>}
            </div>
          )}
        </div>

        <aside className="card live-screen-inspector">
          <header>
            <div><small>Recording</small><h3>{recording ? `${recording.width} × ${recording.height}` : 'Nothing recorded yet'}</h3></div>
            <span className={phase === 'recording' ? 'running' : ''}>{label}</span>
          </header>
          <dl>
            <div><dt>Device</dt><dd>{recording?.deviceName ?? recordingDevice ?? deviceName}</dd></div>
            <div><dt>Original length</dt><dd>{recording ? formatClock(recording.durationMs) : '—'}</dd></div>
            <div><dt>After cuts</dt><dd>{recording ? formatClock(resultMs) : '—'}</dd></div>
            <div><dt>Kept segments</dt><dd>{recording ? kept.length : '—'}</dd></div>
            <div><dt>File size</dt><dd>{recording ? megabytes(recording.bytes) : '—'}</dd></div>
          </dl>
          {exporting && (
            <div className="recorder-export" role="status">
              <div><span style={{ width: `${Math.round((exportProgress ?? 0) * 100)}%` }} /></div>
              <small>Exporting {Math.round((exportProgress ?? 0) * 100)}%</small>
              <button onClick={() => void api.screenRecordingExportCancel()}>Cancel</button>
            </div>
          )}
          <p className="live-screen-hint">Leaving this page finishes a running recording and keeps it. Switching devices discards a recording that is still running.</p>
        </aside>
      </div>

      {phase === 'editing' && recording && (
        <div className="card recorder-editor">
          <div className="recorder-controls">
            <button onClick={togglePlayback} disabled={!desktop} aria-label={playing ? 'Pause' : 'Play'}>
              {playing ? <Pause size={14} /> : <Play size={14} />}
            </button>
            <code>{formatClock(positionMs)} / {formatClock(durationMs)}</code>
            <button onClick={() => setMarkIn(positionMs)}>Set Start</button>
            <button onClick={() => setMarkOut(positionMs)}>Set End</button>
            <button disabled={!selection} onClick={cutSelection}><Scissors size={13} />Cut Selection</button>
            <button disabled={cuts.length === 0} onClick={() => setCuts((current) => current.slice(0, -1))}><Undo2 size={13} />Undo Cut</button>
            <button disabled={cuts.length === 0 && markIn === null && markOut === null} onClick={() => { setCuts([]); setMarkIn(null); setMarkOut(null) }}>
              <RotateCcw size={13} />Reset
            </button>
          </div>
          <div className="recorder-timeline" onClick={seekFromTimeline} role="slider" aria-label="Recording position" aria-valuemin={0} aria-valuemax={durationMs} aria-valuenow={Math.round(positionMs)}>
            {normalizedCuts.map((cut) => (
              <span key={`${cut.startMs}-${cut.endMs}`} className="recorder-cut" style={{ left: percent(cut.startMs), width: percent(cut.endMs - cut.startMs) }} />
            ))}
            {selection && <span className="recorder-selection" style={{ left: percent(selection.startMs), width: percent(selection.endMs - selection.startMs) }} />}
            {markIn !== null && <span className="recorder-mark" style={{ left: percent(markIn) }} />}
            {markOut !== null && <span className="recorder-mark" style={{ left: percent(markOut) }} />}
            <span className="recorder-playhead" style={{ left: percent(positionMs) }} />
          </div>
          <p className="recorder-help">
            Click the timeline to move, set a start and an end, then cut the selection. Playback skips cut footage, so what plays is what exports.
            {kept.length === 0 && ' Everything is cut — undo a cut to export.'}
          </p>
        </div>
      )}
    </section>
  )
}
