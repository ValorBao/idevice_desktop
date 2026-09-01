import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Check } from 'lucide-react'
import { devices, type Device } from './data'
import { api, errorMessage, events, isDesktopRuntime } from './api'
import type { Connection, WorkbenchMode } from './types'
import { summaryToDevice } from './lib/device'
import { TitleBar } from './components/TitleBar'
import { LeftRail } from './components/LeftRail'
import { Onboarding } from './components/Onboarding'
import { PairModal } from './components/PairModal'
import { InspectWorkbench, type InspectSubView } from './pages/InspectWorkbench'
import { FilesWorkbench, type FilesSubView } from './pages/FilesWorkbench'
import { AppsWorkbench, type AppsSubView } from './pages/AppsWorkbench'
import { WatchWorkbench, type WatchInstrument } from './pages/WatchWorkbench'

function stationCopy(
  mode: WorkbenchMode,
  inspectSubView: InspectSubView,
  filesSubView: FilesSubView,
  appsSubView: AppsSubView,
  watchInstrument: WatchInstrument,
): [string, string] {
  if (mode === 'inspect') {
    if (inspectSubView === 'diagnostics') return ['Diagnostics Relay', 'com.apple.mobile.diagnostics_relay']
    if (inspectSubView === 'crashes') return ['Crash Reports', 'com.apple.crashreportcopymobile']
    return ['Inspect Station', 'Vehicle Telemetry · Diagnostics Relay · Crash Analytics']
  }
  if (mode === 'files') {
    if (filesSubView === 'pasteboard') return ['Pasteboard', 'Explicit bounded CoreDevice text and image transfer']
    return ['Payload Files', 'Apple File Conduit (AFC) · Application Sandboxes']
  }
  if (mode === 'apps') {
    if (appsSubView === 'jit') return ['JIT & Debugger Tunnel', 'com.apple.dt.* services']
    if (appsSubView === 'profiles') return ['Provisioning Profiles', 'Read-only Misagent signing and expiry inspection']
    if (appsSubView === 'xctest') return ['Test Lab', 'Read-only XCTest runner and developer-service preflight']
    return ['Applications & JIT', 'Installation Proxy · Sideloading · Debugger Tunnel']
  }
  if (watchInstrument === 'location') return ['Location', 'com.apple.dt.simulatelocation']
  if (watchInstrument === 'screen') return ['Live Screen', 'Live PNG device preview and still-frame capture']
  return ['Live Blackbox', 'Processes, performance, network capture, and live device logs']
}

function App() {
  const desktop = useMemo(isDesktopRuntime, [])
  const [mode, setMode] = useState<WorkbenchMode>('inspect')
  const [inspectSubView, setInspectSubView] = useState<InspectSubView>('overview')
  const [filesSubView, setFilesSubView] = useState<FilesSubView>('afc')
  const [appsSubView, setAppsSubView] = useState<AppsSubView>('manager')
  const [watchInstrument, setWatchInstrument] = useState<WatchInstrument>('monitor')
  const [deviceCatalog, setDeviceCatalog] = useState<Device[]>(desktop ? [] : devices)
  const [deviceId, setDeviceId] = useState(desktop ? '' : 'd1')
  const deviceIdRef = useRef(deviceId)
  const [connection, setConnection] = useState<Connection>(desktop ? 'none' : 'connected')
  const [pairOpen, setPairOpen] = useState(false)
  const [toast, setToast] = useState('')
  const mountedRef = useRef(true)
  const lifecycleRef = useRef(0)
  const refreshRunningRef = useRef(false)
  const refreshPendingRef = useRef(false)
  const device = deviceCatalog.find((item) => item.id === deviceId) ?? deviceCatalog[0] ?? devices[0]
  const connected = connection === 'connected'
  const [headerTitle, headerDetail] = stationCopy(mode, inspectSubView, filesSubView, appsSubView, watchInstrument)
  useEffect(() => { deviceIdRef.current = deviceId }, [deviceId])
  useEffect(() => {
    mountedRef.current = true
    lifecycleRef.current += 1
    return () => {
      mountedRef.current = false
      lifecycleRef.current += 1
    }
  }, [])

  const refreshDevices = useCallback(async () => {
    if (!desktop || !mountedRef.current) return
    refreshPendingRef.current = true
    if (refreshRunningRef.current) return

    refreshRunningRef.current = true
    const lifecycle = lifecycleRef.current
    const lifecycleIsCurrent = () => mountedRef.current && lifecycleRef.current === lifecycle
    try {
      while (refreshPendingRef.current && lifecycleIsCurrent()) {
        refreshPendingRef.current = false
        try {
          const found = await api.deviceList()
          if (!lifecycleIsCurrent()) return
          // A device event arrived while this snapshot was loading. Do not let
          // the older catalog take over the session; fetch the latest one.
          if (refreshPendingRef.current) continue

          const catalog = found.map(summaryToDevice)
          setDeviceCatalog(catalog)
          if (!found.length) {
            setDeviceId('')
            setConnection('none')
            setMode('inspect')
            await api.deviceDisconnect().catch((error) => {
              if (lifecycleIsCurrent()) setToast(errorMessage(error))
            })
            continue
          }

          const current = found.find((item) => item.id === deviceIdRef.current)
          const target = current ?? found.find((item) => item.paired && item.connectable) ?? found[0]
          if (target.paired && target.connectable) {
            if (target.id !== deviceIdRef.current) await api.deviceSelect(target.id)
            if (!lifecycleIsCurrent()) return
            if (refreshPendingRef.current) continue
            setDeviceId(target.id)
            setConnection('connected')
          } else {
            setDeviceId(target.id)
            setConnection('detected')
            setMode('inspect')
            await api.deviceDisconnect().catch((error) => {
              if (lifecycleIsCurrent()) setToast(errorMessage(error))
            })
          }
        } catch (error) {
          if (!lifecycleIsCurrent()) return
          if (refreshPendingRef.current) continue
          setConnection('none')
          setToast(errorMessage(error))
        }
      }
    } finally {
      refreshRunningRef.current = false
      // StrictMode can remount while the first mount still has a listing in
      // flight. The remounted effect marks a refresh pending; start it after
      // the obsolete runner releases the serialization lock.
      if (refreshPendingRef.current && mountedRef.current) void refreshDevices()
    }
  }, [desktop])

  useEffect(() => {
    if (!desktop) return
    let disposed = false
    let unlisten: (() => void) | undefined
    void events.deviceChanged(() => { if (!disposed) void refreshDevices() })
      .then((stop) => {
        if (disposed) stop()
        else unlisten = stop
      })
      .catch((error) => { if (!disposed) setToast(errorMessage(error)) })
    void api.deviceMonitorStart()
      .then(() => { if (!disposed) return refreshDevices() })
      .catch((error) => { if (!disposed) setToast(errorMessage(error)) })
    return () => {
      disposed = true
      unlisten?.()
      void api.deviceMonitorStop()
    }
  }, [desktop, refreshDevices])

  useEffect(() => {
    if (!toast) return
    const timer = window.setTimeout(() => setToast(''), 2500)
    return () => window.clearTimeout(timer)
  }, [toast])

  const selectDevice = async (id: string) => {
    setDeviceId(id)
    if (!desktop) {
      setConnection('connected')
      return
    }
    try {
      await api.deviceSelect(id)
      setConnection('connected')
    } catch (error) {
      setConnection('detected')
      setToast(errorMessage(error))
    }
  }

  const disconnect = async () => {
    if (desktop) await api.deviceDisconnect().catch((error) => setToast(errorMessage(error)))
    setConnection('none')
    setMode('inspect')
  }

  const finishPairing = async () => {
    try {
      if (desktop) {
        const paired = await api.devicePair(device.udid)
        await api.deviceSelect(paired.udid)
        await refreshDevices()
      }
      setConnection('connected')
      setPairOpen(false)
      setToast(`${device.name} paired`)
    } catch (error) {
      setToast(errorMessage(error))
    }
  }

  return (
    <div className="desktop theme-clean mode-dark device-lab">
      <div className="window-shell">
        <TitleBar device={device} connection={connection} />
        <div className="window-body">
          <LeftRail
            device={device}
            deviceCatalog={deviceCatalog}
            connection={connection}
            desktop={desktop}
            mode={mode}
            onSelectMode={setMode}
            onSelectDevice={(id) => void selectDevice(id)}
            onPairOpen={() => setPairOpen(true)}
            onDisconnect={() => void disconnect()}
            onToast={setToast}
          />

          <main className="main-panel">
            <header className="page-header">
              <div>
                <h1>{headerTitle}</h1>
                <p>{headerDetail}</p>
              </div>
              <div className="header-spacer" />
              <div className="header-device-state">
                <span className="header-state-dot" />
                <span>{connected ? device.conn : 'Offline'}</span>
                <i />
                <span>{connected ? `iOS ${device.ios}` : 'No session'}</span>
              </div>
            </header>

            <div className="page-scroll" key={`${mode}:${device.udid}`}>
              {connected && (
                <>
                  {mode === 'inspect' && (
                    <InspectWorkbench
                      device={device}
                      desktop={desktop}
                      subView={inspectSubView}
                      onSubViewChange={setInspectSubView}
                      onError={setToast}
                    />
                  )}
                  {mode === 'files' && (
                    <FilesWorkbench
                      desktop={desktop}
                      device={device}
                      subView={filesSubView}
                      onSubViewChange={setFilesSubView}
                      onToast={setToast}
                    />
                  )}
                  {mode === 'apps' && (
                    <AppsWorkbench
                      desktop={desktop}
                      device={device}
                      subView={appsSubView}
                      onSubViewChange={setAppsSubView}
                      onToast={setToast}
                    />
                  )}
                  {mode === 'watch' && (
                    <WatchWorkbench
                      connected={connected}
                      desktop={desktop}
                      device={device}
                      activeInstrument={watchInstrument}
                      onInstrumentChange={setWatchInstrument}
                      onError={setToast}
                    />
                  )}
                </>
              )}
            </div>

            {!connected && (
              <Onboarding
                state={connection}
                device={device}
                desktop={desktop}
                onDetect={() => desktop ? void refreshDevices() : setConnection('detected')}
                onPair={() => void finishPairing()}
                onCancel={() => setConnection('none')}
              />
            )}
          </main>
        </div>
      </div>
      {pairOpen && <PairModal onClose={() => setPairOpen(false)} onPair={() => void finishPairing()} />}
      {toast && <div className="toast"><span><Check size={12} /></span>{toast}</div>}
    </div>
  )
}

export default App
