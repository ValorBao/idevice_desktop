import { Activity, AppWindow, Bug } from 'lucide-react'
import type { Device } from '../data'
import { WorkbenchTabs } from '../components/WorkbenchTabs'
import { Overview } from './Overview'
import { Diagnostics } from './Diagnostics'
import { CrashReports } from './CrashReports'

export type InspectSubView = 'overview' | 'diagnostics' | 'crashes'

export const inspectStationCopy: Record<InspectSubView, [string, string]> = {
  overview: ['Inspect Station', 'Vehicle Telemetry · Diagnostics Relay · Crash Analytics'],
  diagnostics: ['Diagnostics Relay', 'com.apple.mobile.diagnostics_relay'],
  crashes: ['Crash Reports', 'com.apple.crashreportcopymobile'],
}

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
      <WorkbenchTabs
        label="Inspect views"
        value={subView}
        onChange={onSubViewChange}
        tabs={[
          { id: 'overview', label: 'Overview & Hardware', icon: AppWindow },
          { id: 'diagnostics', label: 'Diagnostics Relay', icon: Activity },
          { id: 'crashes', label: 'Crash Reports', icon: Bug },
        ]}
      />
      <div className="workbench-content">
        {subView === 'overview' && <Overview device={device} desktop={desktop} onError={onError} />}
        {subView === 'diagnostics' && <Diagnostics device={device} desktop={desktop} onError={onError} />}
        {subView === 'crashes' && <CrashReports desktop={desktop} udid={device.udid} onToast={onError} />}
      </div>
    </div>
  )
}
