import { invoke } from '@tauri-apps/api/core'
import { listen, type Event, type UnlistenFn } from '@tauri-apps/api/event'
import { confirm, open, save } from '@tauri-apps/plugin-dialog'

export type CommandError = { kind: string; message: string; retryable: boolean }

export type DeviceSummary = {
  id: string
  udid: string
  deviceId: number
  connection: string
  transports: string[]
  connectable: boolean
  paired: boolean
  name: string | null
  model: string | null
  ios: string | null
}

export type DeviceChangeEvent = {
  kind: 'connected' | 'disconnected'
  device: DeviceSummary | null
  deviceId: number | null
}

export type DeviceOverview = {
  udid: string
  name: string | null
  productType: string | null
  productVersion: string | null
  buildVersion: string | null
  serialNumber: string | null
  uniqueChipId: string | null
  hardwareModel: string | null
  hardwarePlatform: string | null
  wifiAddress: string | null
  connection: string
  paired: boolean
  battery: {
    level: number | null
    healthPercent: number | null
    cycleCount: number | null
    temperatureCelsius: number | null
    voltageVolts: number | null
    raw: unknown
  }
  storage: null | {
    totalBytes: number
    freeBytes: number
    usedBytes: number
    blockSize: number
  }
}

export type RemoteFileEntry = {
  name: string
  path: string
  kind: string
  isDirectory: boolean
  size: number
  modified: string
}

export type FileSharingApp = { bundleId: string; name: string }

export type InstalledApp = {
  bundleId: string
  name: string
  version: string
  sizeBytes: number
  system: boolean
  iconDataUrl: string | null
  raw: unknown
}

export type CrashReportSummary = {
  name: string
  path: string
  kind: string
  process: string
  sizeBytes: number | null
  modified: string
}

export type CrashReportContent = {
  path: string
  content: string
  truncated: boolean
  sizeBytes: number
}

export type OperationProgress = { operation: string; item: string; percent: number }
export type DeviceLog = {
  timestamp: string
  level: string
  process: string
  pid: number
  message: string
  subsystem: string | null
  category: string | null
}
export type DeveloperStatus = {
  developerMode: boolean | null
  ddiMounted: boolean
  ddiImages: unknown
  rsdAvailable: boolean
}
export type JitSession = { bundleId: string; pid: number; response: string | null }
export type LocationSession = { latitude: number; longitude: number; transport: string }

declare global {
  interface Window { __TAURI_INTERNALS__?: unknown }
}

export const isDesktopRuntime = () => typeof window !== 'undefined' && Boolean(window.__TAURI_INTERNALS__)

const call = <T>(command: string, args: Record<string, unknown> = {}) => invoke<T>(command, args)

export const api = {
  deviceList: () => call<DeviceSummary[]>('device_list'),
  deviceSelect: (id: string) => call<void>('device_select', { udid: id }),
  deviceDisconnect: () => call<void>('device_disconnect'),
  devicePair: (udid: string) => call<DeviceSummary>('device_pair', { udid, hostName: 'idevice desktop' }),
  deviceForget: (udid: string) => call<void>('device_forget', { udid }),
  deviceMonitorStart: () => call<void>('device_monitor_start'),
  deviceMonitorStop: () => call<void>('device_monitor_stop'),
  overview: (udid?: string) => call<DeviceOverview>('overview_get', { udid }),
  screenshot: (udid?: string) => call<string>('device_screenshot', { udid }),
  diagnostic: (kind: 'battery' | 'gestalt' | 'io' | 'nand' | 'wifi', udid?: string) => {
    if (kind === 'battery') return call<unknown>('diagnostics_battery', { udid })
    if (kind === 'gestalt') return call<unknown>('diagnostics_gestalt', { udid, keys: null })
    if (kind === 'io') return call<unknown>('diagnostics_ioregistry', { udid, plane: null, name: null, class: null })
    if (kind === 'nand') return call<unknown>('diagnostics_nand', { udid })
    return call<unknown>('diagnostics_wifi', { udid })
  },
  afcList: (path: string, udid?: string, bundleId?: string) => call<RemoteFileEntry[]>('afc_list', { udid, path, bundleId: bundleId ?? null }),
  afcMkdir: (path: string, udid?: string, bundleId?: string) => call<void>('afc_mkdir', { udid, path, bundleId: bundleId ?? null }),
  afcRemove: (path: string, recursive: boolean, udid?: string, bundleId?: string) => call<void>('afc_remove', { udid, path, recursive, bundleId: bundleId ?? null }),
  afcUpload: (localPath: string, remotePath: string, udid?: string, bundleId?: string) => call<void>('afc_upload', { udid, localPath, remotePath, bundleId: bundleId ?? null }),
  afcDownload: (remotePath: string, localPath: string, udid?: string, bundleId?: string) => call<void>('afc_download', { udid, remotePath, localPath, bundleId: bundleId ?? null }),
  afcTransferCancel: () => call<void>('afc_transfer_cancel', {}),
  fileSharingApps: (udid?: string) => call<FileSharingApp[]>('file_sharing_apps', { udid }),
  appsList: (udid?: string) => call<InstalledApp[]>('apps_list', { udid }),
  appsDebuggable: (udid?: string) => call<InstalledApp[]>('apps_debuggable', { udid }),
  appInstall: (localPath: string, udid?: string) => call<void>('app_install', { udid, localPath }),
  appUninstall: (bundleId: string, udid?: string) => call<void>('app_uninstall', { udid, bundleId }),
  crashReportsList: (udid?: string) => call<CrashReportSummary[]>('crash_reports_list', { udid }),
  crashReportRead: (path: string, udid?: string) => call<CrashReportContent>('crash_report_read', { udid, path }),
  crashReportExport: (path: string, localPath: string, udid?: string) => call<void>('crash_report_export', { udid, path, localPath }),
  logsStart: (udid?: string, pid?: number) => call<void>('logs_start', { udid, pid }),
  logsStop: () => call<void>('logs_stop'),
  developerStatus: (udid?: string) => call<DeveloperStatus>('developer_status', { udid }),
  developerReveal: (udid?: string) => call<void>('developer_mode_reveal', { udid }),
  developerEnable: (udid?: string) => call<void>('developer_mode_enable', { udid }),
  developerAccept: (udid?: string) => call<void>('developer_mode_accept', { udid }),
  ddiMount: (paths: { imagePath: string; signaturePath?: string; manifestPath?: string; trustCachePath?: string }, udid?: string) => call<void>('ddi_mount', { udid, ...paths }),
  ddiMountAuto: (udid?: string) => call<void>('ddi_mount_auto', { udid }),
  ddiUnmount: (udid?: string) => call<void>('ddi_unmount', { udid }),
  jitStart: (bundleId: string, udid?: string) => call<JitSession>('jit_start', { udid, bundleId }),
  jitStop: () => call<void>('jit_stop'),
  locationStart: (latitude: number, longitude: number, udid?: string) => call<LocationSession>('location_start', { udid, latitude, longitude }),
  locationStop: () => call<void>('location_stop'),
}

export const events = {
  deviceChanged: (handler: (payload: DeviceChangeEvent) => void) => listen<DeviceChangeEvent>('device://changed', (event) => handler(event.payload)),
  logLine: (handler: (payload: DeviceLog) => void) => listen<DeviceLog>('logs://line', (event) => handler(event.payload)),
  appProgress: (handler: (payload: OperationProgress) => void) => listen<OperationProgress>('apps://install-progress', (event) => handler(event.payload)),
  ddiProgress: (handler: (payload: OperationProgress) => void) => listen<OperationProgress>('developer://ddi-progress', (event) => handler(event.payload)),
  transferProgress: (handler: (payload: OperationProgress) => void) => listen<OperationProgress>('files://transfer-progress', (event) => handler(event.payload)),
  raw: <T>(name: string, handler: (event: Event<T>) => void) => listen<T>(name, handler),
}

export const dialogs = {
  ipa: () => open({ multiple: false, filters: [{ name: 'iOS application', extensions: ['ipa'] }] }),
  file: (name: string, extensions: string[]) => open({ multiple: false, filters: [{ name, extensions }] }),
  anyFile: () => open({ multiple: false }),
  saveFile: (defaultPath: string) => save({ defaultPath }),
  /**
   * Confirms a destructive action.
   *
   * This must go through the dialog plugin rather than `window.confirm`: wry
   * does not implement the WKWebView JavaScript panel delegates, so the native
   * `confirm` never opens a dialog and resolves to false, silently cancelling
   * whatever it was guarding. In browser demo mode there is no plugin, so fall
   * back to the real `window.confirm`, which works there.
   */
  confirmDestructive: (message: string, actionLabel: string) =>
    isDesktopRuntime()
      ? confirm(message, { title: actionLabel, kind: 'warning', okLabel: actionLabel, cancelLabel: 'Cancel' })
      : Promise.resolve(window.confirm(message)),
}

export const errorMessage = (error: unknown) => {
  if (typeof error === 'object' && error && 'message' in error) return String((error as CommandError).message)
  return String(error)
}

export type { UnlistenFn }
