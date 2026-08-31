import { useState } from 'react'
import { AppWindow, Code2 } from 'lucide-react'
import type { Device } from '../data'
import { Apps } from './Apps'
import { Developer } from './Developer'

export type AppsSubView = 'manager' | 'jit'

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
        </div>
      </div>

      <div className="workbench-content">
        {subView === 'manager' && (
          <Apps desktop={desktop} udid={device.udid} onToast={onToast} />
        )}
        {subView === 'jit' && (
          <Developer desktop={desktop} device={device} onToast={onToast} />
        )}
      </div>
    </div>
  )
}
