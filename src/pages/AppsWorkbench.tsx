import { AppWindow, BadgeCheck, Beaker, Code2, KeyRound, Package } from 'lucide-react'
import type { Device } from '../data'
import { WorkbenchTabs, type TabItem } from '../components/WorkbenchTabs'
import { Apps } from './Apps'
import { Developer } from './Developer'
import { Profiles } from './Profiles'
import { PersonalSigning } from './PersonalSigning'
import { TestLab } from './TestLab'
import { TrollStore } from './TrollStore'

export type AppsSubView = 'manager' | 'jit' | 'profiles' | 'signing' | 'xctest' | 'trollstore'

export const appsTabs: readonly TabItem<AppsSubView>[] = [
  {
    id: 'manager',
    label: 'Apps',
    ariaLabel: 'Applications & Sideload',
    icon: <AppWindow size={14} />,
    title: 'Applications & JIT',
    detail: 'Installation Proxy · Sideloading · Debugger Tunnel',
  },
  {
    id: 'jit',
    label: 'JIT',
    ariaLabel: 'JIT & Debugger Tunnel',
    icon: <Code2 size={14} />,
    title: 'JIT & Debugger Tunnel',
    detail: 'com.apple.dt.* services',
  },
  {
    id: 'profiles',
    label: 'Profiles',
    ariaLabel: 'Provisioning Profiles',
    icon: <BadgeCheck size={14} />,
    title: 'Provisioning Profiles',
    detail: 'Read-only Misagent signing and expiry inspection',
  },
  {
    id: 'signing',
    label: 'Personal Sign',
    ariaLabel: 'Personal Sign',
    icon: <KeyRound size={14} />,
    title: 'Personal Signing Assistant',
    detail: 'Apple Account or local Keychain identity · signed IPA export',
  },
  {
    id: 'xctest',
    label: 'Test Lab',
    ariaLabel: 'Test Lab',
    icon: <Beaker size={14} />,
    title: 'Test Lab',
    detail: 'Read-only XCTest runner and developer-service preflight',
  },
  {
    id: 'trollstore',
    label: 'TrollStore',
    ariaLabel: 'TrollStore',
    icon: <Package size={14} />,
    title: 'TrollStore',
    detail: 'USB helper restore · local IPA handoff',
  },
]

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
        items={appsTabs}
        active={subView}
        onSelect={onSubViewChange}
        ariaLabel="Apps views"
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
        {subView === 'signing' && (
          <PersonalSigning desktop={desktop} udid={device.udid} deviceName={device.name} onToast={onToast} />
        )}
        {subView === 'xctest' && (
          <TestLab desktop={desktop} udid={device.udid} onToast={onToast} />
        )}
        {subView === 'trollstore' && (
          <TrollStore desktop={desktop} udid={device.udid} onToast={onToast} />
        )}
      </div>
    </div>
  )
}
