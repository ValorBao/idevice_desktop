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
import { Files } from './pages/Files'
import { AppsWorkbench, type AppsSubView } from './pages/AppsWorkbench'
import { WatchWorkbench, type WatchInstrument } from './pages/WatchWorkbench'

const workbenchMeta: Record<WorkbenchMode, [string, string]> = {
  inspect: ['Inspect Bench', 'Hardware telemetry · Diagnostics relay · Crash logs'],
  files: ['Files Explorer', 'Apple File Conduit (AFC) · Application sandboxes'],
  apps: ['Applications & JIT', 'Installation proxy · Sideloading · Debugger tunnel'],
  watch: ['Live Watch Station', 'OS Trace stream · Location spoofer probe · Signal monitors'],
}

function App() {
  const desktop = useMemo(isDesktopRuntime, [])
  const [mode, setMode] = useState<WorkbenchMode>('inspect')
  const [inspectSubView, setInspectSubView] = useState<InspectSubView>('overview')
  const [appsSubView, setAppsSubView] = useState<AppsSubView>('manager')
  const [watchInstrument, setWatchInstrument] = useState<WatchInstrument>('logs')
  const [deviceCatalog, setDeviceCatalog] = useState<Device[]>(desktop ? [] : devices)
  const [deviceId, setDeviceId] = useState(desktop ? '' : 'd1')
  const deviceIdRef = useRef(deviceId)
  const [connection, setConnection] = useState<Connection>(desktop ? 'none' : 'connected')
  const [pairOpen, setPairOpen] = useState(false)
  const [toast, setToast] = useState('')
  const device = deviceCatalog.find((item) => item.id === deviceId) ?? deviceCatalog[0] ?? devices[0]
  const connected = connection === 'connected'
  useEffect(() => { deviceIdRef.current = deviceId }, [deviceId])

  const refreshDevices = useCallback(async () => {
    if (!desktop) return
    try {
      const found = await api.deviceList()
      const catalog = found.map(summaryToDevice)
      setDeviceCatalog(catalog)
      if (!found.length) {
        setDeviceId('')
        setConnection('none')
        return
      }
      const current = found.find((item) => item.id === deviceIdRef.current)
      const target = current ?? found.find((item) => item.paired && item.connectable) ?? found[0]
      setDeviceId(target.id)
      if (target.paired && target.connectable) {
        if (target.id !== deviceIdRef.current) await api.deviceSelect(target.id)
        setConnection('connected')
      } else {
        setConnection('detected')
      }
    } catch (error) {
      setConnection('none')
      setToast(errorMessage(error))
    }
  }, [desktop])

  useEffect(() => {
    if (!desktop) return
    let disposed = false
    let unlisten: (() => void) | undefined
    events.deviceChanged(() => { if (!disposed) void refreshDevices() }).then((stop) => { unlisten = stop })
    void api.deviceMonitorStart().then(refreshDevices).catch((error) => setToast(errorMessage(error)))
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
                <h1>{workbenchMeta[mode][0]}</h1>
                <p>{workbenchMeta[mode][1]}</p>
              </div>
              <div className="header-spacer" />
              <div className="header-device-state">
                <span className="header-state-dot" />
                <span>{connected ? device.conn : 'Offline'}</span>
                <i />
                <span>{connected ? `iOS ${device.ios}` : 'No session'}</span>
              </div>
            </header>

            <div className="page-scroll" key={mode}>
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
                <Files desktop={desktop} udid={device.udid} onToast={setToast} />
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
