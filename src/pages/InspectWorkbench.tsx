import { useState } from 'react'
import { Activity, AppWindow, Bug } from 'lucide-react'
import type { Device } from '../data'
import { Overview } from './Overview'
import { Diagnostics } from './Diagnostics'
import { CrashReports } from './CrashReports'

export type InspectSubView = 'overview' | 'diagnostics' | 'crashes'

export function InspectWorkbench({
  device,
  desktop,
  subView,
  onSubViewChange,
  onError,
}: {
  device: Device
  desktop: boolean
  subView: InspectSubView
  onSubViewChange: (sub: InspectSubView) => void
  onError: (message: string) => void
}) {
  return (
    <div className="inspect-workbench">
      <div className="workbench-subbar">
        <div className="workbench-tabs" role="tablist" aria-label="Inspect views">
          <button
            role="tab"
            aria-selected={subView === 'overview'}
            className={subView === 'overview' ? 'active' : ''}
            onClick={() => onSubViewChange('overview')}
          >
            <AppWindow size={14} />
            <span>Overview & Hardware</span>
          </button>
          <button
            role="tab"
            aria-selected={subView === 'diagnostics'}
            className={subView === 'diagnostics' ? 'active' : ''}
            onClick={() => onSubViewChange('diagnostics')}
          >
            <Activity size={14} />
            <span>Diagnostics Relay</span>
          </button>
          <button
            role="tab"
            aria-selected={subView === 'crashes'}
            className={subView === 'crashes' ? 'active' : ''}
            onClick={() => onSubViewChange('crashes')}
          >
            <Bug size={14} />
            <span>Crash Reports</span>
          </button>
        </div>
      </div>

      <div className="workbench-content">
        {subView === 'overview' && <Overview device={device} desktop={desktop} onError={onError} />}
        {subView === 'diagnostics' && <Diagnostics device={device} desktop={desktop} onError={onError} />}
        {subView === 'crashes' && <CrashReports desktop={desktop} udid={device.udid} onToast={onError} />}
      </div>
    </div>
  )
}
