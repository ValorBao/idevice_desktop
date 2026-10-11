import { ClipboardPaste, FolderOpen } from 'lucide-react'
import type { Device } from '../data'
import { WorkbenchTabs, type TabItem } from '../components/WorkbenchTabs'
import { Files } from './Files'
import { Pasteboard } from './Pasteboard'

export type FilesSubView = 'afc' | 'pasteboard'

export const filesTabs: readonly TabItem<FilesSubView>[] = [
  {
    id: 'afc',
    label: 'AFC & Sandboxes',
    ariaLabel: 'Files',
    icon: <FolderOpen size={14} />,
    title: 'Payload Files',
    detail: 'Apple File Conduit (AFC) · Application Sandboxes',
  },
  {
    id: 'pasteboard',
    label: 'Pasteboard',
    ariaLabel: 'Pasteboard',
    icon: <ClipboardPaste size={14} />,
    title: 'Pasteboard',
    detail: 'Explicit bounded CoreDevice text and image transfer',
  },
]

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
        items={filesTabs}
        active={subView}
        onSelect={onSubViewChange}
        ariaLabel="Files views"
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
