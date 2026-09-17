import { MapPin, ScreenShare, TerminalSquare } from 'lucide-react'
import type { Device } from '../data'
import { WorkbenchTabs, type TabItem } from '../components/WorkbenchTabs'
import { Monitor } from './Monitor'
import { Location } from './Location'
import { LiveScreen } from './LiveScreen'

export type WatchInstrument = 'monitor' | 'location' | 'screen'

export const watchTabs: readonly TabItem<WatchInstrument>[] = [
  {
    id: 'monitor',
    label: 'Monitor',
    ariaLabel: 'Monitor',
    icon: <TerminalSquare size={14} />,
    badge: <span className="live-chip"><i /> live</span>,
    title: 'Live Blackbox',
    detail: 'Processes, performance, network capture, and live device logs',
  },
  {
    id: 'location',
    label: 'Location',
    ariaLabel: 'Location',
    icon: <MapPin size={14} />,
    title: 'Location',
    detail: 'com.apple.dt.simulatelocation',
  },
  {
    id: 'screen',
    label: 'Live Screen',
    ariaLabel: 'Live Screen',
    icon: <ScreenShare size={14} />,
    title: 'Live Screen',
    detail: 'Live PNG device preview and still-frame capture',
  },
]

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
        items={watchTabs}
        active={activeInstrument}
        onSelect={onInstrumentChange}
        ariaLabel="Watch instruments"
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
