import { getCurrentWindow } from '@tauri-apps/api/window'
import { type MouseEvent } from 'react'
import type { Device } from '../data'
import { isDesktopRuntime } from '../api'
import type { Connection } from '../types'

export function TitleBar({ device, connection }: { device: Device; connection: Connection }) {
  const status = connection === 'connected' ? `${device.conn} connected · ready` : connection === 'detected' ? 'device found · awaiting trust' : 'scanning for devices'
  const startWindowDrag = (event: MouseEvent<HTMLDivElement>) => {
    if (!isDesktopRuntime() || event.button !== 0) return
    const target = event.target
    if (target instanceof Element && target.closest('button')) return
    event.preventDefault()
    void getCurrentWindow().startDragging().catch((error) => console.error('Unable to drag window', error))
  }
  const controlWindow = (action: 'close' | 'minimize' | 'fullscreen') => {
    if (!isDesktopRuntime()) return
    const appWindow = getCurrentWindow()
    const operation = action === 'close'
      ? appWindow.close()
      : action === 'minimize'
        ? appWindow.minimize()
        : appWindow.isFullscreen().then((fullscreen) => appWindow.setFullscreen(!fullscreen))
    void operation.catch((error) => console.error(`Unable to ${action} window`, error))
  }
  return (
    <div className="titlebar" data-tauri-drag-region onMouseDown={startWindowDrag}>
      <div className="traffic-lights">
        <button type="button" aria-label="Close window" title="Close" onClick={() => controlWindow('close')}><i /></button>
        <button type="button" aria-label="Minimize window" title="Minimize" onClick={() => controlWindow('minimize')}><i /></button>
        <button type="button" aria-label="Enter or exit fullscreen" title="Fullscreen" onClick={() => controlWindow('fullscreen')}><i /></button>
      </div>
      <div className="app-title" data-tauri-drag-region>
        <span className="brand-mark"><i /><i /><i /></span>
        <b>DEVICE LAB</b>
        <small>{connection === 'connected' ? device.name : connection === 'detected' ? `${device.model} · untrusted` : 'No device'}</small>
      </div>
      <div className="titlebar-spacer" data-tauri-drag-region />
      <div className={`connection-status status-${connection}`} data-tauri-drag-region><i />{status}</div>
    </div>
  )
}
