import { useEffect, useMemo } from 'react'
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

/**
 * Checks whether a device's iOS version is within the TrollStore 2.x window
 * (iOS 15.0 through 17.0) so devices on iOS 17.1+ or below 15.0 don't clutter
 * the primary Apps navigation bar with an inapplicable tool.
 */
export function isTrollStoreCandidateIos(ios: string): boolean {
  const parts = ios.trim().split('.').map((part) => Number.parseInt(part, 10))
  const major = parts[0] ?? 0
  const minor = parts[1] ?? 0
  const patch = parts[2] ?? 0
  if (Number.isNaN(major) || major < 15 || major > 17) return false
  if (major === 17 && (minor > 0 || patch > 0)) return false
  if (major === 16 && minor > 7) return false
  return true
}

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
    id: 'profiles',
    label: 'Profiles',
    ariaLabel: 'Provisioning Profiles',
    icon: <BadgeCheck size={14} />,
    title: 'Provisioning Profiles',
    detail: 'Read-only Misagent signing and expiry inspection',
  },
  {
    id: 'xctest',
    label: 'Test Lab',
    ariaLabel: 'Test Lab',
    icon: <Beaker size={14} />,
    badge: <span className="tab-pill-subtle">Preflight</span>,
    title: 'Test Lab',
    detail: 'Read-only XCTest runner and developer-service preflight',
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
  const showTrollStore = isTrollStoreCandidateIos(device.ios)
  const visibleTabs = useMemo(
    () => showTrollStore ? appsTabs : appsTabs.filter((tab) => tab.id !== 'trollstore'),
    [showTrollStore],
  )

  useEffect(() => {
    if (!showTrollStore && subView === 'trollstore') {
      onSubViewChange('manager')
    }
  }, [showTrollStore, subView, onSubViewChange])

  return (
    <div className="apps-workbench">
      <WorkbenchTabs
        items={visibleTabs}
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
        {subView === 'trollstore' && showTrollStore && (
          <TrollStore desktop={desktop} udid={device.udid} onToast={onToast} />
        )}
      </div>
    </div>
  )
}
