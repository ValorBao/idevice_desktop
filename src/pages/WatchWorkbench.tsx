import { MapPin, ScreenShare, TerminalSquare } from 'lucide-react'
import type { Device } from '../data'
import { WorkbenchTabs } from '../components/WorkbenchTabs'
import { Monitor } from './Monitor'
import { Location } from './Location'
import { LiveScreen } from './LiveScreen'

export type WatchInstrument = 'monitor' | 'location' | 'screen'

export const watchStationCopy: Record<WatchInstrument, [string, string]> = {
  monitor: ['Live Blackbox', 'Processes, performance, network capture, and live device logs'],
  location: ['Location', 'com.apple.dt.simulatelocation'],
  screen: ['Live Screen', 'Live PNG device preview and still-frame capture'],
}

export function WatchWorkbench({
  connected,
  desktop,
  device,
  activeInstrument,
  onInstrumentChange,
  onError,
}: {
  connected: boolean
  desktop: boolean
  device: Device
  activeInstrument: WatchInstrument
  onInstrumentChange: (inst: WatchInstrument) => void
  onError: (message: string) => void
}) {
  return (
    <div className="watch-workbench">
      <WorkbenchTabs
        label="Watch instruments"
        value={activeInstrument}
        onChange={onInstrumentChange}
        tabs={[
          {
            id: 'monitor',
            label: 'Monitor',
            icon: TerminalSquare,
            ariaLabel: 'Monitor',
            extra: <span className="live-chip"><i /> live</span>,
          },
          { id: 'location', label: 'Location', icon: MapPin, ariaLabel: 'Location' },
          { id: 'screen', label: 'Live Screen', icon: ScreenShare, ariaLabel: 'Live Screen' },
        ]}
      />
      <div className="workbench-content">
        {activeInstrument === 'monitor' && (
          <Monitor connected={connected} desktop={desktop} udid={device.udid} onError={onError} />
        )}
        {activeInstrument === 'location' && (
          <Location desktop={desktop} udid={device.udid} onToast={onError} />
        )}
        {activeInstrument === 'screen' && (
          <LiveScreen desktop={desktop} udid={device.udid} onToast={onError} />
        )}
      </div>
    </div>
  )
}
