import { Activity, AppWindow, Bug } from 'lucide-react'
import type { Device } from '../data'
import { WorkbenchTabs, type TabItem } from '../components/WorkbenchTabs'
import { Overview } from './Overview'
import { Diagnostics } from './Diagnostics'
import { CrashReports } from './CrashReports'

export type InspectSubView = 'overview' | 'diagnostics' | 'crashes'

export const inspectTabs: readonly TabItem<InspectSubView>[] = [
  {
    id: 'overview',
    label: 'Overview & Hardware',
    icon: <AppWindow size={14} />,
    title: 'Inspect Station',
    detail: 'Vehicle Telemetry · Diagnostics Relay · Crash Analytics',
  },
  {
    id: 'diagnostics',
    label: 'Diagnostics Relay',
    icon: <Activity size={14} />,
    title: 'Diagnostics Relay',
    detail: 'com.apple.mobile.diagnostics_relay',
  },
  {
    id: 'crashes',
    label: 'Crash Reports',
    icon: <Bug size={14} />,
    title: 'Crash Reports',
    detail: 'com.apple.crashreportcopymobile',
  },
]

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
        items={inspectTabs}
        active={subView}
        onSelect={onSubViewChange}
        ariaLabel="Inspect views"
      />

      <div className="workbench-content">
        {subView === 'overview' && <Overview device={device} desktop={desktop} onError={onError} />}
        {subView === 'diagnostics' && <Diagnostics device={device} desktop={desktop} onError={onError} />}
        {subView === 'crashes' && <CrashReports desktop={desktop} udid={device.udid} onToast={onError} />}
      </div>
    </div>
  )
}
