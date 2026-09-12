import { AppWindow, BadgeCheck, Beaker, Code2, KeyRound } from 'lucide-react'
import type { Device } from '../data'
import { Apps } from './Apps'
import { Developer } from './Developer'
import { Profiles } from './Profiles'
import { PersonalSigning } from './PersonalSigning'
import { TestLab } from './TestLab'

export type AppsSubView = 'manager' | 'jit' | 'profiles' | 'signing' | 'xctest'

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
      <div className="workbench-subbar">
        <div className="workbench-tabs" role="tablist" aria-label="Apps views">
          <button
            role="tab"
            aria-selected={subView === 'manager'}
            className={subView === 'manager' ? 'active' : ''}
            onClick={() => onSubViewChange('manager')}
          >
            <AppWindow size={14} />
            <span>Applications & Sideload</span>
          </button>
          <button
            role="tab"
            aria-selected={subView === 'jit'}
            className={subView === 'jit' ? 'active' : ''}
            onClick={() => onSubViewChange('jit')}
          >
            <Code2 size={14} />
            <span>JIT & Debugger Tunnel</span>
          </button>
          <button
            role="tab"
            aria-label="Provisioning Profiles"
            aria-selected={subView === 'profiles'}
            className={subView === 'profiles' ? 'active' : ''}
            onClick={() => onSubViewChange('profiles')}
          >
            <BadgeCheck size={14} />
            <span>Profiles</span>
          </button>
          <button
            role="tab"
            aria-label="Personal Sign"
            aria-selected={subView === 'signing'}
            className={subView === 'signing' ? 'active' : ''}
            onClick={() => onSubViewChange('signing')}
          >
            <KeyRound size={14} />
            <span>Personal Sign</span>
          </button>
          <button
            role="tab"
            aria-label="Test Lab"
            aria-selected={subView === 'xctest'}
            className={subView === 'xctest' ? 'active' : ''}
            onClick={() => onSubViewChange('xctest')}
          >
            <Beaker size={14} />
            <span>Test Lab</span>
          </button>
        </div>
      </div>

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
      </div>
    </div>
  )
}
