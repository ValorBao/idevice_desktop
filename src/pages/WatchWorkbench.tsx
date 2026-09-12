import { MapPin, ScreenShare, TerminalSquare } from 'lucide-react'
import type { Device } from '../data'
import { Monitor } from './Monitor'
import { Location } from './Location'
import { LiveScreen } from './LiveScreen'

export type WatchInstrument = 'monitor' | 'location' | 'screen'

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
      <div className="workbench-subbar">
        <div className="workbench-tabs" role="tablist" aria-label="Watch instruments">
          <button
            role="tab"
            aria-label="Monitor"
            aria-selected={activeInstrument === 'monitor'}
            className={activeInstrument === 'monitor' ? 'active' : ''}
            onClick={() => onInstrumentChange('monitor')}
          >
            <TerminalSquare size={14} />
            <span>Monitor</span>
            <span className="live-chip"><i /> live</span>
          </button>
          <button
            role="tab"
            aria-label="Location"
            aria-selected={activeInstrument === 'location'}
            className={activeInstrument === 'location' ? 'active' : ''}
            onClick={() => onInstrumentChange('location')}
          >
            <MapPin size={14} />
            <span>Location</span>
          </button>
          <button
            role="tab"
            aria-label="Live Screen"
            aria-selected={activeInstrument === 'screen'}
            className={activeInstrument === 'screen' ? 'active' : ''}
            onClick={() => onInstrumentChange('screen')}
          >
            <ScreenShare size={14} />
            <span>Live Screen</span>
          </button>
        </div>
      </div>

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
