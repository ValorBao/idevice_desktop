import { useCallback, useEffect, useMemo, useState } from 'react'
import { Check } from 'lucide-react'
import { isDesktopRuntime } from './api'
import type { WorkbenchMode } from './types'
import { useDeviceSession } from './lib/useDeviceSession'
import { TitleBar } from './components/TitleBar'
import { LeftRail } from './components/LeftRail'
import { Onboarding } from './components/Onboarding'
import { PairModal } from './components/PairModal'
import { tabCopy } from './components/WorkbenchTabs'
import { InspectWorkbench, inspectTabs, type InspectSubView } from './pages/InspectWorkbench'
import { FilesWorkbench, filesTabs, type FilesSubView } from './pages/FilesWorkbench'
import { AppsWorkbench, appsTabs, type AppsSubView } from './pages/AppsWorkbench'
import { WatchWorkbench, watchTabs, type WatchInstrument } from './pages/WatchWorkbench'

/** The header copy for the active workbench view. */
function stationCopy(
  mode: WorkbenchMode,
  inspectSubView: InspectSubView,
  filesSubView: FilesSubView,
  appsSubView: AppsSubView,
  watchInstrument: WatchInstrument,
): [string, string] {
  const copy = mode === 'inspect' ? tabCopy(inspectTabs, inspectSubView)
    : mode === 'files' ? tabCopy(filesTabs, filesSubView)
    : mode === 'apps' ? tabCopy(appsTabs, appsSubView)
    : tabCopy(watchTabs, watchInstrument)
  return copy ?? ['Device Lab', '']
}

function App() {
  const desktop = useMemo(isDesktopRuntime, [])
  const [mode, setMode] = useState<WorkbenchMode>('inspect')
  const [inspectSubView, setInspectSubView] = useState<InspectSubView>('overview')
  const [filesSubView, setFilesSubView] = useState<FilesSubView>('afc')
  const [appsSubView, setAppsSubView] = useState<AppsSubView>('manager')
  const [watchInstrument, setWatchInstrument] = useState<WatchInstrument>('monitor')
  const [pairOpen, setPairOpen] = useState(false)
  const [toast, setToast] = useState('')
  const leaveDeviceViews = useCallback(() => setMode('inspect'), [])
  const session = useDeviceSession({ desktop, onError: setToast, onSessionLost: leaveDeviceViews })
  const { catalog: deviceCatalog, device, connection, connected } = session
  const [headerTitle, headerDetail] = stationCopy(mode, inspectSubView, filesSubView, appsSubView, watchInstrument)

  useEffect(() => {
    if (!toast) return
    const timer = window.setTimeout(() => setToast(''), 2500)
    return () => window.clearTimeout(timer)
  }, [toast])

  const finishPairing = async () => {
    if (!await session.pair()) return
    setPairOpen(false)
    setToast(`${device.name} paired`)
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
            onSelectDevice={(id) => void session.select(id)}
            onPairOpen={() => setPairOpen(true)}
            onDisconnect={() => void session.disconnect()}
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
                      onToast={setToast}
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
                      onToast={setToast}
                    />
                  )}
                </>
              )}
            </div>

            {connection !== 'connected' && (
              <Onboarding
                state={connection}
                device={device}
                desktop={desktop}
                onDetect={() => void session.refresh()}
                onPair={() => void finishPairing()}
                onCancel={session.dismiss}
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
