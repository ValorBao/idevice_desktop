import type { AppInfo, Device } from '../data'
import { api, type DeviceSummary, type InstalledApp } from '../api'
import { appColor, bytes } from './format'

export const summaryToDevice = (summary: DeviceSummary): Device => ({
  id: summary.id,
  name: summary.name ?? (summary.connectable ? 'Unpaired device' : 'Network iPhone'),
  model: summary.model ?? (summary.connectable ? 'iPhone / iPad' : 'Bonjour device'),
  modelId: summary.model ?? 'Unknown',
  ios: summary.ios ?? '—',
  build: '—',
  udid: summary.udid,
  serial: '—',
  ecid: '—',
  chip: 'Apple Silicon',
  wifi: '—',
  conn: summary.connection,
  battery: 0,
  batteryHealth: 0,
  cycles: 0,
  storageUsed: 0,
  storageTotal: 0,
  transports: summary.transports,
  connectable: summary.connectable,
})

export const deviceScreenCache = new Map<string, string>()

/** Screenshot the device, optionally mounting a DDI after the first failure. */
export async function captureDeviceScreen(udid: string, mountIfNeeded = false) {
  try {
    const image = await api.screenshot(udid)
    deviceScreenCache.set(udid, image)
    return image
  } catch (error) {
    if (!mountIfNeeded) throw error
    await api.ddiMountAuto(udid)
    const image = await api.screenshot(udid)
    deviceScreenCache.set(udid, image)
    return image
  }
}

export const installedToApp = (app: InstalledApp): AppInfo => ({
  id: app.bundleId,
  name: app.name,
  bundle: app.bundleId,
  version: app.version || '—',
  size: bytes(app.sizeBytes),
  color: appColor(app.bundleId),
  icon: app.iconDataUrl ?? undefined,
  system: app.system,
})
