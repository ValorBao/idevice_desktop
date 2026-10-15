import { AppWindow, Code2, KeyRound, Package } from 'lucide-react'
import type { Device } from '../data'
import { WorkbenchTabs, type TabItem } from '../components/WorkbenchTabs'
import { Apps } from './Apps'
import { Developer } from './Developer'
import { PersonalSigning } from './PersonalSigning'
import { TrollStore } from './TrollStore'

export type AppsSubView = 'manager' | 'signing' | 'jit' | 'trollstore'

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
    id: 'signing',
    label: 'Personal Sign',
    ariaLabel: 'Personal Sign',
    icon: <KeyRound size={14} />,
    title: 'Personal Signing Assistant',
    detail: 'Apple Account or local Keychain identity · signed IPA export',
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
    id: 'trollstore',
    label: 'TrollStore',
    ariaLabel: 'TrollStore',
    icon: <Package size={14} />,
    badge: <span className="tab-pill-subtle">15.0–17.0</span>,
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
        {subView === 'signing' && (
          <PersonalSigning desktop={desktop} udid={device.udid} deviceName={device.name} onToast={onToast} />
        )}
        {subView === 'trollstore' && (
          <TrollStore desktop={desktop} udid={device.udid} onToast={onToast} />
        )}
      </div>
    </div>
  )
}
