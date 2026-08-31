import { useState } from 'react'
import { TerminalSquare, MapPin, Bug, Layers, Radio } from 'lucide-react'
import type { Device } from '../data'
import { Logs } from './Logs'
import { Location } from './Location'
import { CrashReports } from './CrashReports'

export type WatchInstrument = 'logs' | 'location' | 'crashes'

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
            aria-selected={activeInstrument === 'logs'}
            className={activeInstrument === 'logs' ? 'active' : ''}
            onClick={() => onInstrumentChange('logs')}
          >
            <TerminalSquare size={14} />
            <span>OS Logs Console</span>
            <span className="live-chip"><i /> live</span>
          </button>
          <button
            role="tab"
            aria-selected={activeInstrument === 'location'}
            className={activeInstrument === 'location' ? 'active' : ''}
            onClick={() => onInstrumentChange('location')}
          >
            <MapPin size={14} />
            <span>Location Simulator Probe</span>
          </button>
          <button
            role="tab"
            aria-selected={activeInstrument === 'crashes'}
            className={activeInstrument === 'crashes' ? 'active' : ''}
            onClick={() => onInstrumentChange('crashes')}
          >
            <Bug size={14} />
            <span>Crash Monitor</span>
          </button>
        </div>
      </div>

      <div className="workbench-content">
        {activeInstrument === 'logs' && (
          <Logs connected={connected} desktop={desktop} udid={device.udid} onError={onError} />
        )}
        {activeInstrument === 'location' && (
          <Location desktop={desktop} udid={device.udid} onToast={onError} />
        )}
        {activeInstrument === 'crashes' && (
          <CrashReports desktop={desktop} udid={device.udid} onToast={onError} />
        )}
      </div>
    </div>
  )
}
