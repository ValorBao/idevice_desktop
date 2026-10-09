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
  /** The entry exists but its metadata could not be read; only `name` is real. */
  unreadable: boolean
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
export type LogStatus = { sessionId: string; udid: string; state: string; message: string | null }
export type DeviceLog = {
  sessionId: string
  udid: string
  timestamp: string
  level: string
  process: string
  pid: number
  message: string
  subsystem: string | null
  category: string | null
}
export type ProcessSummary = {
  pid: number
  name: string
  executablePath: string | null
  isApplication: boolean
  canStop: boolean
  /** Opaque stale-row guard; return it unchanged when stopping a process. */
  identity: string
}
export type ProcessSnapshot = {
  processes: ProcessSummary[]
  transport: string
  available: boolean
  supportsLaunch: boolean
  supportsStop: boolean
  limitation: string | null
}
export type ProcessLaunch = { pid: number; bundleId: string; transport: string }
export type PerformanceProcessSample = {
  pid: number
  name: string
  identity: string
  cpuPercent: number | null
  memoryBytes: number | null
}
export type PerformanceSample = {
  sequence: number
  timestampMs: number
  intervalMs: number
  transport: string
  systemCpuPercent: number | null
  processes: PerformanceProcessSample[]
}
export type PerformanceStatus = {
  state: string
  message: string | null
  transport: string | null
  intervalMs: number
}
export type PerformanceExportRow = {
  timestampMs: number
  pid: number
  name: string
  identity: string
  cpuPercent: number | null
  memoryBytes: number | null
}
export type NetworkCaptureFilter = {
  pid: number | null
  interfaceName: string | null
}
export type NetworkCaptureProgress = {
  packets: number
  bytes: number
  outputBytes: number
  elapsedMs: number
  lastProcess: string | null
  lastInterface: string | null
}
export type NetworkCaptureStatus = {
  state: string
  message: string | null
  destination: string
  transport: string | null
  filter: NetworkCaptureFilter
}
export type NotificationObservationEvent = {
  sessionId: string
  sequence: number
  timestampMs: number
  name: string
}
export type NotificationObservationStatus = {
  sessionId: string
  state: string
  message: string | null
  transport: string | null
  subscriptions: string[]
}
export type LiveScreenFrame = {
  sequence: number
  timestampMs: number
  width: number
  height: number
  bytes: number
  fps: number
  dataUrl: string
}
export type LiveScreenStatus = {
  state: string
  message: string | null
  transport: string | null
  targetFps: number
}
export type ProvisioningProfileSummary = {
  id: string
  uuid: string | null
  name: string
  teamName: string | null
  teamIdentifier: string | null
  applicationIdentifier: string | null
  createdAt: string | null
  expiresAt: string | null
  daysRemaining: number | null
  expirationState: string
  profileType: string
  platforms: string[]
  deviceCount: number
  provisionsAllDevices: boolean
  getTaskAllow: boolean | null
  sizeBytes: number
  parseError: string | null
}
export type ProvisioningProfileSnapshot = {
  profiles: ProvisioningProfileSummary[]
  transport: string
  totalCount: number
  truncated: boolean
}
export type PersonalSigningIdentity = {
  hash: string
  name: string
  teamIdentifier: string | null
  matchesProfile: boolean
}
export type PersonalSigningPreflight = {
  ipaName: string
  appName: string
  bundleId: string
  version: string
  profileName: string
  profileUuid: string | null
  teamIdentifier: string | null
  applicationIdentifier: string | null
  expiresAt: string | null
  deviceCount: number
  deviceIncluded: boolean
  identities: PersonalSigningIdentity[]
  selectedIdentityHash: string | null
  ready: boolean
  blockers: string[]
  warnings: string[]
}
export type PersonalSigningRequest = {
  ipaPath: string
  profilePath: string
  identityHash: string
  outputPath: string
}
export type PersonalSigningResult = {
  outputPath: string
  appName: string
  bundleId: string
  profileName: string
  identityName: string
  sizeBytes: number
}

export type PersonalAccountStatus = {
  loggedIn: boolean
  email: string | null
  teamId: string | null
  teamName: string | null
  anisetteServer: string
  credentialStorage: string
  passwordStored: boolean
}

export type PersonalAccountTwoFactor = {
  lastError: string | null
  unknown: boolean
  sms: boolean
  numbers: Array<{
    numberWithDialCode: string
    lastTwoDigits: string
    pushMode: string
    id: number
  }>
  selectedNumberId: number | null
}

export type PersonalAccountSigningRequest = {
  operationId: string
  ipaPath: string
  outputPath: string
  udid: string
  deviceName: string
}

export type PersonalAccountSigningResult = {
  outputPath: string
  appName: string
  bundleId: string
  version: string
  teamId: string
  teamName: string
  sizeBytes: number
}
export type PasteboardTextSnapshot = {
  text: string | null
  byteLength: number
  characterCount: number
  changeCount: number | null
  itemCount: number
  state: string
  message: string | null
  transport: string
}
export type PasteboardWriteResult = {
  byteLength: number
  characterCount: number
  transport: string
}
export type PasteboardImageSnapshot = {
  dataUrl: string | null
  mimeType: string | null
  width: number | null
  height: number | null
  byteLength: number
  changeCount: number | null
  itemCount: number
  state: string
  message: string | null
  transport: string
}
export type PasteboardImagePreparation = {
  preparationId: string
  dataUrl: string
  mimeType: string
  width: number
  height: number
  byteLength: number
  fileName: string
}
export type PasteboardImageWriteResult = {
  mimeType: string
  width: number
  height: number
  byteLength: number
  transport: string
}
export type XCTestRunnerCandidate = {
  bundleId: string
  name: string
  version: string
  executable: string | null
  debuggable: boolean
  isWebdriverAgent: boolean
  configurationReady: boolean
  issues: string[]
}
export type XCTestTargetApp = {
  bundleId: string
  name: string
  version: string
  debuggable: boolean
}
export type XCTestPreflightSnapshot = {
  iosVersion: string
  transport: string
  executionSupported: boolean
  limitation: string | null
  runnerTotal: number
  targetTotal: number
  truncated: boolean
  runners: XCTestRunnerCandidate[]
  targets: XCTestTargetApp[]
}
export type XCTestPlanRequest = {
  runnerBundleId: string
  targetBundleId: string | null
  mode: 'test' | 'wda'
  testsToRun: string[]
  testsToSkip: string[]
  timeoutSeconds: number
}
export type XCTestRunPlan = {
  runnerBundleId: string
  runnerName: string
  targetBundleId: string | null
  targetName: string | null
  mode: 'test' | 'wda'
  testsToRun: string[]
  testsToSkip: string[]
  timeoutSeconds: number
  wdaBridge: boolean
  transport: string
}
export type DeveloperStatus = {
  developerMode: boolean | null
  ddiMounted: boolean
  ddiImages: unknown
  rsdAvailable: boolean
}
export type JitSession = { bundleId: string; pid: number; response: string | null }
export type LocationSession = { latitude: number; longitude: number; transport: string }

export type TrollStoreRemovableApp = {
  bundleId: string
  name: string
  bundleName: string
}

export type TrollStoreStatus = {
  productVersion: string
  buildVersion: string
  helperSupported: boolean
  helperDetail: string
  helperCaution: string | null
  trollstoreInstalled: boolean
  removableApps: TrollStoreRemovableApp[]
  ipaInstallAvailable: boolean
  ipaInstallDetail: string
}

export type TrollStoreHelperResult = {
  appName: string
  message: string
}

export type TrollStoreIpaResult = {
  fileName: string
  message: string
}

declare global {
  interface Window { __TAURI_INTERNALS__?: unknown }
}

export const isDesktopRuntime = () => typeof window !== 'undefined' && Boolean(window.__TAURI_INTERNALS__)

const call = <T>(command: string, args: Record<string, unknown> = {}) => invoke<T>(command, args)

export const api = {
  deviceList: () => call<DeviceSummary[]>('device_list'),
  deviceSelect: (id: string) => call<void>('device_select', { udid: id }),
  deviceDisconnect: () => call<void>('device_disconnect'),
  devicePair: (udid: string) => call<DeviceSummary>('device_pair', { udid, hostName: 'idevice_desktop' }),
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
  afcCreateFile: (path: string, udid?: string, bundleId?: string) => call<void>('afc_create_file', { udid, path, bundleId: bundleId ?? null }),
  afcRename: (from: string, to: string, udid?: string, bundleId?: string) => call<void>('afc_rename', { udid, from, to, bundleId: bundleId ?? null }),
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
  logsStart: (sessionId: string, udid: string, pid?: number) => call<void>('logs_start', { sessionId, udid, pid }),
  logsStop: (sessionId: string) => call<void>('logs_stop', { sessionId }),
  processesList: (udid?: string) => call<ProcessSnapshot>('processes_list', { udid }),
  processLaunch: (bundleId: string, udid?: string) => call<ProcessLaunch>('process_launch', { udid, bundleId }),
  processStop: (pid: number, identity: string, udid?: string) => call<void>('process_stop', { udid, pid, identity }),
  performanceStart: (udid?: string, intervalMs = 1000) => call<void>('performance_start', { udid, intervalMs }),
  performanceStop: () => call<void>('performance_stop'),
  performanceExportCsv: (localPath: string, rows: PerformanceExportRow[]) => call<void>('performance_export_csv', { localPath, rows }),
  networkCaptureStart: (localPath: string, udid?: string, pid?: number, interfaceName?: string) => call<void>('network_capture_start', {
    localPath,
    udid,
    pid: pid ?? null,
    interfaceName: interfaceName?.trim() || null,
  }),
  networkCaptureStop: () => call<void>('network_capture_stop'),
  networkCaptureCancel: () => call<void>('network_capture_cancel'),
  notificationObservationStart: (subscriptions: string[], sessionId: string, udid?: string) => call<void>('notification_observation_start', {
    udid,
    sessionId,
    subscriptions,
  }),
  notificationObservationStop: () => call<void>('notification_observation_stop'),
  liveScreenStart: (udid?: string) => call<void>('live_screen_start', { udid }),
  liveScreenStop: () => call<void>('live_screen_stop'),
  liveScreenExportFrame: (localPath: string) => call<void>('live_screen_export_frame', { localPath }),
  provisioningProfiles: (udid?: string) => call<ProvisioningProfileSnapshot>('provisioning_profiles_list', { udid }),
  personalSigningPreflight: (ipaPath: string, profilePath: string, udid: string) => call<PersonalSigningPreflight>('personal_signing_preflight', { ipaPath, profilePath, udid }),
  personalSigningExport: (request: PersonalSigningRequest, udid: string) => call<PersonalSigningResult>('personal_signing_export', { request, udid }),
  personalAccountStatus: () => call<PersonalAccountStatus>('personal_account_status'),
  personalAccountLogin: (email: string, password: string) => call<PersonalAccountStatus>('personal_account_login', { email, password }),
  personalAccountSubmitTwoFactor: (code: string) => call<void>('personal_account_submit_two_factor', { code }),
  personalAccountCancelTwoFactor: () => call<void>('personal_account_cancel_two_factor'),
  personalAccountSignCancel: (operationId: string) => call<void>('personal_account_sign_cancel', { operationId }),
  personalAccountSignOut: () => call<void>('personal_account_sign_out'),
  personalAccountSignExport: (request: PersonalAccountSigningRequest) => call<PersonalAccountSigningResult>('personal_account_sign_export', { request }),
  pasteboardTextRead: (udid?: string) => call<PasteboardTextSnapshot>('pasteboard_text_read', { udid }),
  pasteboardTextWrite: (text: string, udid?: string) => call<PasteboardWriteResult>('pasteboard_text_write', { udid, text }),
  pasteboardImageRead: (udid?: string) => call<PasteboardImageSnapshot>('pasteboard_image_read', { udid }),
  pasteboardImagePrepare: (localPath: string, udid?: string) => call<PasteboardImagePreparation>('pasteboard_image_prepare', { udid, localPath }),
  pasteboardImageWrite: (preparationId: string, udid?: string) => call<PasteboardImageWriteResult>('pasteboard_image_write', { udid, preparationId }),
  pasteboardImageDiscard: (preparationId: string) => call<boolean>('pasteboard_image_discard', { preparationId }),
  xctestPreflight: (udid?: string) => call<XCTestPreflightSnapshot>('xctest_preflight', { udid }),
  xctestPlanPrepare: (request: XCTestPlanRequest, udid?: string) => call<XCTestRunPlan>('xctest_plan_prepare', { udid, request }),
  trollstoreStatus: (udid?: string) => call<TrollStoreStatus>('trollstore_status', { udid }),
  trollstoreHelperInstall: (bundleId: string, udid?: string) => call<TrollStoreHelperResult>('trollstore_helper_install', { udid, bundleId }),
  trollstoreIpaInstall: (localPath: string, udid?: string) => call<TrollStoreIpaResult>('trollstore_ipa_install', { udid, localPath }),
  developerStatus: (udid?: string) => call<DeveloperStatus>('developer_status', { udid }),
  developerReveal: (udid?: string) => call<void>('developer_mode_reveal', { udid }),
  developerEnable: (udid?: string) => call<void>('developer_mode_enable', { udid }),
  developerAccept: (udid?: string) => call<void>('developer_mode_accept', { udid }),
  ddiMount: (paths: { imagePath: string; signaturePath?: string; manifestPath?: string; trustCachePath?: string }, udid?: string) => call<void>('ddi_mount', { udid, ...paths }),
  ddiMountAuto: (udid?: string) => call<void>('ddi_mount_auto', { udid }),
  // Mounts only when the device has no image yet. The app calls this once per
  // device selection, so no page has to ask for a mount. It fails with kind
  // `ddi-missing` when iOS 16 or earlier needs its one-time download.
  ddiEnsure: (udid?: string) => call<void>('ddi_ensure', { udid }),
  // The only path that reaches the network, and only when the user asks.
  ddiDownload: (udid?: string) => call<void>('ddi_download', { udid }),
  ddiUnmount: (udid?: string) => call<void>('ddi_unmount', { udid }),
  jitStart: (bundleId: string, udid?: string) => call<JitSession>('jit_start', { udid, bundleId }),
  jitStop: () => call<void>('jit_stop'),
  locationStart: (latitude: number, longitude: number, udid?: string) => call<LocationSession>('location_start', { udid, latitude, longitude }),
  locationStop: () => call<void>('location_stop'),
}

export const events = {
  deviceChanged: (handler: (payload: DeviceChangeEvent) => void) => listen<DeviceChangeEvent>('device://changed', (event) => handler(event.payload)),
  logStatus: (handler: (payload: LogStatus) => void) => listen<LogStatus>('logs://status', (event) => handler(event.payload)),
  logLine: (handler: (payload: DeviceLog) => void) => listen<DeviceLog>('logs://line', (event) => handler(event.payload)),
  performanceSample: (handler: (payload: PerformanceSample) => void) => listen<PerformanceSample>('performance://sample', (event) => handler(event.payload)),
  performanceStatus: (handler: (payload: PerformanceStatus) => void) => listen<PerformanceStatus>('performance://status', (event) => handler(event.payload)),
  networkCaptureProgress: (handler: (payload: NetworkCaptureProgress) => void) => listen<NetworkCaptureProgress>('network-capture://progress', (event) => handler(event.payload)),
  networkCaptureStatus: (handler: (payload: NetworkCaptureStatus) => void) => listen<NetworkCaptureStatus>('network-capture://status', (event) => handler(event.payload)),
  notificationObservationEvent: (handler: (payload: NotificationObservationEvent) => void) => listen<NotificationObservationEvent>('notifications://event', (event) => handler(event.payload)),
  notificationObservationStatus: (handler: (payload: NotificationObservationStatus) => void) => listen<NotificationObservationStatus>('notifications://status', (event) => handler(event.payload)),
  liveScreenFrame: (handler: (payload: LiveScreenFrame) => void) => listen<LiveScreenFrame>('live-screen://frame', (event) => handler(event.payload)),
  liveScreenStatus: (handler: (payload: LiveScreenStatus) => void) => listen<LiveScreenStatus>('live-screen://status', (event) => handler(event.payload)),
  appProgress: (handler: (payload: OperationProgress) => void) => listen<OperationProgress>('apps://install-progress', (event) => handler(event.payload)),
  personalSigningProgress: (handler: (payload: OperationProgress) => void) => listen<OperationProgress>('personal-signing://progress', (event) => handler(event.payload)),
  personalAccountTwoFactor: (handler: (payload: PersonalAccountTwoFactor) => void) => listen<PersonalAccountTwoFactor>('personal-account://two-factor-required', (event) => handler(event.payload)),
  personalAccountProgress: (handler: (payload: OperationProgress) => void) => listen<OperationProgress>('personal-account://progress', (event) => handler(event.payload)),
  ddiProgress: (handler: (payload: OperationProgress) => void) => listen<OperationProgress>('developer://ddi-progress', (event) => handler(event.payload)),
  trollstoreProgress: (handler: (payload: OperationProgress) => void) => listen<OperationProgress>('trollstore://progress', (event) => handler(event.payload)),
  transferProgress: (handler: (payload: OperationProgress) => void) => listen<OperationProgress>('files://transfer-progress', (event) => handler(event.payload)),
  raw: <T>(name: string, handler: (event: Event<T>) => void) => listen<T>(name, handler),
}

export const dialogs = {
  ipa: () => open({ multiple: false, filters: [{ name: 'iOS application', extensions: ['ipa'] }] }),
  provisioningProfile: () => open({ multiple: false, filters: [{ name: 'Provisioning profile', extensions: ['mobileprovision', 'provisionprofile'] }] }),
  file: (name: string, extensions: string[]) => open({ multiple: false, filters: [{ name, extensions }] }),
  anyFile: () => open({ multiple: false }),
  saveFile: (defaultPath: string) => save({ defaultPath }),
  pcapDestination: (defaultPath: string) => save({ defaultPath, filters: [{ name: 'Packet capture', extensions: ['pcap'] }] }),
  pngDestination: (defaultPath: string) => save({ defaultPath, filters: [{ name: 'PNG image', extensions: ['png'] }] }),
  signedIpaDestination: (defaultPath: string) => save({ defaultPath, filters: [{ name: 'Signed iOS application', extensions: ['ipa'] }] }),
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
