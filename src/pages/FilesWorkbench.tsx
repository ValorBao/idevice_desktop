import { ClipboardPaste, FolderOpen } from 'lucide-react'
import type { Device } from '../data'
import { Files } from './Files'
import { Pasteboard } from './Pasteboard'

export type FilesSubView = 'afc' | 'pasteboard'

export function FilesWorkbench({
  desktop,
  device,
  subView,
  onSubViewChange,
  onToast,
}: {
  desktop: boolean
  device: Device
  subView: FilesSubView
  onSubViewChange: (sub: FilesSubView) => void
  onToast: (message: string) => void
}) {
  return (
    <div className="files-workbench">
      <div className="workbench-subbar">
        <div className="workbench-tabs" role="tablist" aria-label="Files views">
          <button
            role="tab"
            aria-label="Files"
            aria-selected={subView === 'afc'}
            className={subView === 'afc' ? 'active' : ''}
            onClick={() => onSubViewChange('afc')}
          >
            <FolderOpen size={14} />
            <span>AFC & Sandboxes</span>
          </button>
          <button
            role="tab"
            aria-label="Pasteboard"
            aria-selected={subView === 'pasteboard'}
            className={subView === 'pasteboard' ? 'active' : ''}
            onClick={() => onSubViewChange('pasteboard')}
          >
            <ClipboardPaste size={14} />
            <span>Pasteboard</span>
          </button>
        </div>
      </div>

      <div className="workbench-content">
        {subView === 'afc' && (
          <Files desktop={desktop} udid={device.udid} onToast={onToast} />
        )}
        {subView === 'pasteboard' && (
          <Pasteboard desktop={desktop} udid={device.udid} deviceName={device.name} onToast={onToast} />
        )}
      </div>
    </div>
  )
}
