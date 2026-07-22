export type Device = {
  id: string
  name: string
  model: string
  modelId: string
  ios: string
  build: string
  udid: string
  serial: string
  ecid: string
  chip: string
  wifi: string
  conn: string
  battery: number
  batteryHealth: number
  cycles: number
  storageUsed: number
  storageTotal: number
}

export const devices: Device[] = [
  { id: 'd1', name: "Jackson's iPhone", model: 'iPhone 15 Pro', modelId: 'iPhone16,1', ios: '17.5.1', build: '21F90', udid: '00008130-001A2D3C1E88001C', serial: 'F2LX9K3PQ1NM', ecid: '0x1A2D3C1E88001C', chip: 'A17 Pro', wifi: 'F0:18:98:2A:6B:C4', conn: 'USB', battery: 87, batteryHealth: 94, cycles: 212, storageUsed: 142, storageTotal: 256 },
  { id: 'd2', name: 'Test iPad', model: 'iPad Pro 11\"', modelId: 'iPad14,3', ios: '17.4', build: '21E219', udid: '00008027-000C5D2E1488002E', serial: 'DMPX5K9LQ2', ecid: '0x0C5D2E1488002E', chip: 'M2', wifi: 'A4:83:E7:11:09:2D', conn: 'Network', battery: 63, batteryHealth: 100, cycles: 48, storageUsed: 88, storageTotal: 512 },
]

export const gestalt = [
  ['ProductName', 'iPhone OS'], ['ProductVersion', '17.5.1'], ['BuildVersion', '21F90'], ['DeviceClass', 'iPhone'],
  ['HardwareModel', 'D83AP'], ['ChipID', '0x8130'], ['BoardId', '0x08'], ['ProductType', 'iPhone16,1'],
  ['RegionInfo', 'LL/A'], ['DeviceColor', '1 (Natural Titanium)'], ['CPUArchitecture', 'arm64e'], ['HasSEP', 'true'],
  ['DeviceName', "Jackson's iPhone"], ['TotalSystemAvailable', '8192 MB'], ['SupportsForceTouch', 'false'], ['DeviceSupportsHDRDisplay', 'true'],
]

export const ioTree = [
  ['', 'AppleARMPE', ''], ['├─ ', 'arm-io', '@200000000'], ['│  ├─ ', 'AppleT8130CLPC', ''], ['│  └─ ', 'AppleH13CamIn', ''],
  ['├─ ', 'AppleSmartBatteryManager', ''], ['│  └─ ', 'AppleSmartBattery', 'Capacity 94%'], ['├─ ', 'IOThunderboltFamily', ''],
  ['├─ ', 'AppleBCMWLANCore', 'en0'], ['└─ ', 'AppleEmbeddedNVMeController', '1TB'],
]

export type FileEntry = { name: string; folder?: boolean; kind?: string; size?: string; date: string }
export const fileSystem: Record<string, FileEntry[]> = {
  '/': [
    { name: 'DCIM', folder: true, date: 'May 2, 2026' }, { name: 'Downloads', folder: true, date: 'Apr 28, 2026' },
    { name: 'Books', folder: true, date: 'Mar 14, 2026' }, { name: 'PhotoData', folder: true, date: 'May 1, 2026' },
    { name: 'iTunes_Control', folder: true, date: 'Jan 9, 2026' }, { name: 'Purchases', folder: true, date: 'Feb 2, 2026' },
    { name: 'info.plist', kind: 'Property List', size: '1.2 KB', date: 'Mar 11, 2026' },
  ],
  '/DCIM': [
    { name: '100APPLE', folder: true, date: 'May 2, 2026' }, { name: '101APPLE', folder: true, date: 'Apr 19, 2026' },
    { name: '.MISC', folder: true, date: 'Jan 1, 2026' },
  ],
  '/DCIM/100APPLE': [
    { name: 'IMG_0001.HEIC', kind: 'HEIF Image', size: '2.4 MB', date: 'May 2, 2026' },
    { name: 'IMG_0002.HEIC', kind: 'HEIF Image', size: '3.1 MB', date: 'May 2, 2026' },
    { name: 'IMG_0003.MOV', kind: 'QuickTime', size: '48.2 MB', date: 'May 1, 2026' },
    { name: 'IMG_0004.HEIC', kind: 'HEIF Image', size: '2.0 MB', date: 'Apr 30, 2026' },
  ],
}

export type AppInfo = { id: string; name: string; bundle: string; version: string; size: string; color: string; icon?: string; system?: boolean; fresh?: boolean }
export const installedApps: AppInfo[] = [
  { id: 'a1', name: 'StikDebug', bundle: 'com.stik.debug', version: '1.4.2', size: '24.1 MB', color: '#7c6cff' },
  { id: 'a2', name: 'CrossCode', bundle: 'app.crosscode.ios', version: '0.9.0', size: '88.7 MB', color: '#ff7a59' },
  { id: 'a3', name: 'Protokolle', bundle: 'de.protokolle.app', version: '2.1.0', size: '12.3 MB', color: '#34d399' },
  { id: 'a4', name: 'SideStore', bundle: 'com.SideStore.SideStore', version: '0.6.1', size: '31.5 MB', color: '#0a84ff' },
  { id: 'a5', name: 'Safari', bundle: 'com.apple.mobilesafari', version: '17.5', size: '104 MB', color: '#1d9bf0', system: true },
  { id: 'a6', name: 'Settings', bundle: 'com.apple.Preferences', version: '17.5', size: '48 MB', color: '#8e8e93', system: true },
  { id: 'a7', name: 'Files', bundle: 'com.apple.DocumentsApp', version: '17.5', size: '22 MB', color: '#2a87ff', system: true },
]

export type LogLine = { time: string; level: 'INFO' | 'DEBUG' | 'NOTICE' | 'WARN' | 'ERROR'; process: string; message: string }
export const initialLogs: LogLine[] = [
  { time: '18:42:01.221', level: 'INFO', process: 'lockdownd', message: 'StartSession with host 6F3A… succeeded' },
  { time: '18:42:01.398', level: 'DEBUG', process: 'usbmuxd', message: 'Connecting device 0x130 to port 62078' },
  { time: '18:42:02.014', level: 'NOTICE', process: 'CoreDevice', message: 'RSD handshake complete, 38 services advertised' },
  { time: '18:42:02.551', level: 'INFO', process: 'installd', message: 'Verifying code signature for app.crosscode.ios' },
  { time: '18:42:03.102', level: 'WARN', process: 'thermalmonitord', message: 'Thermal pressure level moved to Nominal→Fair' },
  { time: '18:42:03.770', level: 'INFO', process: 'mobile_assertion', message: 'heartbeat ack (seq 4821)' },
  { time: '18:42:04.233', level: 'DEBUG', process: 'afcd', message: 'OPEN /DCIM/100APPLE/IMG_0003.MOV mode=r' },
  { time: '18:42:04.901', level: 'ERROR', process: 'symptomsd', message: 'Failed to read pref domain com.apple.xpc.activity' },
  { time: '18:42:05.412', level: 'INFO', process: 'debugserver', message: 'Attached to pid 1294, granting JIT entitlement' },
  { time: '18:42:06.008', level: 'NOTICE', process: 'locationd', message: 'Simulated location override engaged' },
]

export const liveLogPool = [
  ['INFO', 'lockdownd', 'GetValue BatteryCurrentCapacity → 87'], ['DEBUG', 'usbmuxd', 'rx 64 bytes on session 0x4 (plist)'],
  ['NOTICE', 'CoreDevice', 'service com.apple.instruments.server advertised'], ['INFO', 'mobile_assertion', 'heartbeat ack ok'],
  ['DEBUG', 'afcd', 'READDIR /DCIM/100APPLE entries=4'], ['WARN', 'thermalmonitord', 'pressure level Nominal'],
] as const

export const presets = [
  { id: 'sf', name: 'San Francisco, CA', lat: 37.7749, lng: -122.4194, x: 42, y: 48 },
  { id: 'cupertino', name: 'Apple Park', lat: 37.3349, lng: -122.009, x: 40, y: 56 },
  { id: 'nyc', name: 'New York, NY', lat: 40.7128, lng: -74.006, x: 68, y: 42 },
  { id: 'london', name: 'London, UK', lat: 51.5074, lng: -0.1278, x: 56, y: 30 },
  { id: 'tokyo', name: 'Tokyo, JP', lat: 35.6762, lng: 139.6503, x: 80, y: 52 },
]
