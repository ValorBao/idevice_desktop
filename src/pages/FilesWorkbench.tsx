import { ClipboardPaste, FolderOpen } from 'lucide-react'
import type { Device } from '../data'
import { WorkbenchTabs } from '../components/WorkbenchTabs'
import { Files } from './Files'
import { Pasteboard } from './Pasteboard'

export type FilesSubView = 'afc' | 'pasteboard'

export const filesStationCopy: Record<FilesSubView, [string, string]> = {
  afc: ['Payload Files', 'Apple File Conduit (AFC) · Application Sandboxes'],
  pasteboard: ['Pasteboard', 'Explicit bounded CoreDevice text and image transfer'],
}

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
      <WorkbenchTabs
        label="Files views"
        value={subView}
        onChange={onSubViewChange}
        tabs={[
          { id: 'afc', label: 'AFC & Sandboxes', icon: FolderOpen, ariaLabel: 'Files' },
          { id: 'pasteboard', label: 'Pasteboard', icon: ClipboardPaste, ariaLabel: 'Pasteboard' },
        ]}
      />
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
