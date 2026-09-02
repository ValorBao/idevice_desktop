import { AppWindow, BadgeCheck, Beaker, Code2 } from 'lucide-react'
import type { Device } from '../data'
import { WorkbenchTabs } from '../components/WorkbenchTabs'
import { Apps } from './Apps'
import { Developer } from './Developer'
import { Profiles } from './Profiles'
import { TestLab } from './TestLab'

export type AppsSubView = 'manager' | 'jit' | 'profiles' | 'xctest'

export const appsStationCopy: Record<AppsSubView, [string, string]> = {
  manager: ['Applications & JIT', 'Installation Proxy · Sideloading · Debugger Tunnel'],
  jit: ['JIT & Debugger Tunnel', 'com.apple.dt.* services'],
  profiles: ['Provisioning Profiles', 'Read-only Misagent signing and expiry inspection'],
  xctest: ['Test Lab', 'Read-only XCTest runner and developer-service preflight'],
}

export function AppsWorkbench({
  desktop,
  device,
  subView,
  onSubViewChange,
  onToast,
}: {
  desktop: boolean
  device: Device
  subView: AppsSubView
  onSubViewChange: (sub: AppsSubView) => void
  onToast: (message: string) => void
}) {
  return (
    <div className="apps-workbench">
      <WorkbenchTabs
        label="Apps views"
        value={subView}
        onChange={onSubViewChange}
        tabs={[
          { id: 'manager', label: 'Applications & Sideload', icon: AppWindow },
          { id: 'jit', label: 'JIT & Debugger Tunnel', icon: Code2 },
          { id: 'profiles', label: 'Profiles', icon: BadgeCheck, ariaLabel: 'Provisioning Profiles' },
          { id: 'xctest', label: 'Test Lab', icon: Beaker, ariaLabel: 'Test Lab' },
        ]}
      />
      <div className="workbench-content">
        {subView === 'manager' && (
          <Apps desktop={desktop} udid={device.udid} onToast={onToast} />
        )}
        {subView === 'jit' && (
          <Developer desktop={desktop} device={device} onToast={onToast} />
        )}
        {subView === 'profiles' && (
          <Profiles desktop={desktop} udid={device.udid} onToast={onToast} />
        )}
        {subView === 'xctest' && (
          <TestLab desktop={desktop} udid={device.udid} onToast={onToast} />
        )}
      </div>
    </div>
  )
}
