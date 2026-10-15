import { useState } from 'react'
import { Activity, Clapperboard, Cpu, MapPin, ScreenShare, TerminalSquare } from 'lucide-react'
import type { Device } from '../data'
import { WorkbenchTabs, type TabItem } from '../components/WorkbenchTabs'
import { Logs } from './Logs'
import { Processes } from './Processes'
import { Performance } from './Performance'
import { Location } from './Location'
import { LiveScreen } from './LiveScreen'
import { ScreenRecorder } from './ScreenRecorder'

export type WatchInstrument =
  | 'monitor'
  | 'processes'
  | 'screen'
  | 'location'

type ProcessLens = 'performance' | 'processes'
type ScreenMode = 'preview' | 'recorder'

export const watchTabs: readonly TabItem<WatchInstrument>[] = [
  {
    id: 'monitor',
    label: 'Logs',
    ariaLabel: 'Monitor',
    icon: <TerminalSquare size={14} />,
    badge: <span className="live-chip"><i /> live</span>,
    title: 'Live Blackbox',
    detail: 'Real-time OS Trace and structured device syslog stream',
  },
  {
    id: 'processes',
    label: 'Processes',
    ariaLabel: 'Processes & Performance',
    icon: <Activity size={14} />,
    title: 'Processes & Performance',
    detail: 'Live CPU and memory telemetry, process table, and application control',
  },
  {
    id: 'screen',
    label: 'Screen',
    ariaLabel: 'Screen Studio',
    icon: <ScreenShare size={14} />,
    title: 'Screen Studio',
    detail: 'Live PNG preview, still frame capture, and 60 FPS USB screen recording',
  },
  {
    id: 'location',
    label: 'Location',
    ariaLabel: 'Location',
    icon: <MapPin size={14} />,
    title: 'Location',
    detail: 'com.apple.dt.simulatelocation',
  },
]

export function WatchWorkbench({
  connected,
  desktop,
  device,
  activeInstrument,
  onInstrumentChange,
  onToast,
}: {
  connected: boolean
  desktop: boolean
  device: Device
  activeInstrument: WatchInstrument
  onInstrumentChange: (inst: WatchInstrument) => void
  onToast: (message: string) => void
}) {
  const [processLens, setProcessLens] = useState<ProcessLens>('performance')
  const [screenMode, setScreenMode] = useState<ScreenMode>('preview')

  return (
    <div className="watch-workbench">
      <WorkbenchTabs
        items={watchTabs}
        active={activeInstrument}
        onSelect={onInstrumentChange}
        ariaLabel="Watch instruments"
      />

      <div className="workbench-content">
        {activeInstrument === 'monitor' && (
          <Logs connected={connected} desktop={desktop} udid={device.udid} onToast={onToast} />
        )}
        {activeInstrument === 'processes' && (
          <div className="watch-merged-view">
            <div className="watch-mode-switch" role="tablist" aria-label="Process view mode">
              <button
                type="button"
                role="tab"
                aria-selected={processLens === 'performance'}
                className={processLens === 'performance' ? 'active' : ''}
                onClick={() => setProcessLens('performance')}
              >
                <Activity size={13} />
                <span>Live Telemetry (CPU & Memory)</span>
              </button>
              <button
                type="button"
                role="tab"
                aria-selected={processLens === 'processes'}
                className={processLens === 'processes' ? 'active' : ''}
                onClick={() => setProcessLens('processes')}
              >
                <Cpu size={13} />
                <span>Process Control (Launch & Stop)</span>
              </button>
            </div>
            {processLens === 'performance' ? (
              <Performance desktop={desktop} udid={device.udid} onToast={onToast} />
            ) : (
              <Processes desktop={desktop} udid={device.udid} onToast={onToast} />
            )}
          </div>
        )}
        {activeInstrument === 'screen' && (
          <div className="watch-merged-view">
            <div className="watch-mode-switch" role="tablist" aria-label="Screen capture mode">
              <button
                type="button"
                role="tab"
                aria-selected={screenMode === 'preview'}
                className={screenMode === 'preview' ? 'active' : ''}
                onClick={() => setScreenMode('preview')}
              >
                <ScreenShare size={13} />
                <span>Live Preview & Still Frame</span>
              </button>
              <button
                type="button"
                role="tab"
                aria-selected={screenMode === 'recorder'}
                className={screenMode === 'recorder' ? 'active' : ''}
                onClick={() => setScreenMode('recorder')}
              >
                <Clapperboard size={13} />
                <span>USB Video Recorder (MP4)</span>
              </button>
            </div>
            {screenMode === 'preview' ? (
              <LiveScreen desktop={desktop} udid={device.udid} onToast={onToast} />
            ) : (
              <ScreenRecorder key={device.udid} desktop={desktop} deviceName={device.name} onToast={onToast} />
            )}
          </div>
        )}
        {activeInstrument === 'location' && (
          <Location desktop={desktop} udid={device.udid} onToast={onToast} />
        )}
      </div>
    </div>
  )
}
