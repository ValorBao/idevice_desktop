import { Clapperboard, MapPin, ScreenShare, TerminalSquare } from 'lucide-react'
import type { Device } from '../data'
import { WorkbenchTabs, type TabItem } from '../components/WorkbenchTabs'
import { Monitor } from './Monitor'
import { Location } from './Location'
import { LiveScreen } from './LiveScreen'
import { ScreenRecorder } from './ScreenRecorder'

export type WatchInstrument = 'monitor' | 'location' | 'screen' | 'recorder'

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
  {
    id: 'recorder',
    label: 'Recorder',
    ariaLabel: 'Screen Recorder',
    icon: <Clapperboard size={14} />,
    title: 'Screen Recorder',
    detail: 'Record the iPhone screen over USB, cut it, and export MP4',
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
          <Monitor connected={connected} desktop={desktop} udid={device.udid} onToast={onToast} />
        )}
        {activeInstrument === 'location' && (
          <Location desktop={desktop} udid={device.udid} onToast={onToast} />
        )}
        {activeInstrument === 'screen' && (
          <LiveScreen desktop={desktop} udid={device.udid} onToast={onToast} />
        )}
        {activeInstrument === 'recorder' && (
          <ScreenRecorder key={device.udid} desktop={desktop} deviceName={device.name} onToast={onToast} />
        )}
      </div>
    </div>
  )
}
