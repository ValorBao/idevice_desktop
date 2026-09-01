import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Device } from '../data'
import { CrashReports } from './CrashReports'
import { Developer } from './Developer'
import { Location } from './Location'

const backend = vi.hoisted(() => ({
  api: {
    locationStart: vi.fn(),
    locationStop: vi.fn(),
    developerStatus: vi.fn(),
    appsDebuggable: vi.fn(),
    jitStart: vi.fn(),
    jitStop: vi.fn(),
    crashReportsList: vi.fn(),
    crashReportRead: vi.fn(),
    crashReportExport: vi.fn(),
  },
  dialogs: {
    saveFile: vi.fn(),
  },
  events: {
    ddiProgress: vi.fn(),
  },
}))

const map = vi.hoisted(() => ({
  remove: vi.fn(),
  on: vi.fn(),
  setView: vi.fn(),
  invalidateSize: vi.fn(),
  flyTo: vi.fn(),
}))

const marker = vi.hoisted(() => ({
  addTo: vi.fn(),
  setLatLng: vi.fn(),
}))

vi.mock('../api', () => ({
  ...backend,
  errorMessage: (error: unknown) => String(error),
}))

vi.mock('leaflet', () => ({
  default: {
    map: vi.fn(() => ({ ...map, setView: vi.fn(() => map) })),
    tileLayer: vi.fn(() => ({ addTo: vi.fn() })),
    divIcon: vi.fn(() => ({})),
    marker: vi.fn(() => ({ ...marker, addTo: vi.fn(() => marker) })),
  },
}))

const device: Device = {
  id: 'device-1',
  udid: 'device-1',
  name: 'Test iPhone',
  model: 'iPhone11,8',
  modelId: 'iPhone11,8',
  ios: '17.0',
  build: '21A329',
  serial: 'serial',
  ecid: 'ecid',
  chip: 'A12',
  wifi: '192.0.2.1',
  conn: 'USB',
  battery: 80,
  batteryHealth: 90,
  cycles: 100,
  storageUsed: 10,
  storageTotal: 64,
  transports: ['USB'],
  connectable: true,
}

describe('current surface acceptance', () => {
  beforeEach(() => {
    backend.api.locationStart.mockResolvedValue({
      latitude: 37.7749,
      longitude: -122.4194,
      transport: 'DVT/RSD',
    })
    backend.api.locationStop.mockResolvedValue(undefined)
    backend.api.developerStatus.mockResolvedValue({
      developerMode: true,
      ddiMounted: true,
      ddiImages: null,
      rsdAvailable: true,
    })
    backend.api.appsDebuggable.mockResolvedValue([{
      bundleId: 'com.example.debug',
      name: 'Debug App',
      version: '1.0',
      sizeBytes: 1024,
      system: true,
      iconDataUrl: null,
      raw: null,
    }])
    backend.api.jitStart.mockResolvedValue({
      bundleId: 'com.example.debug',
      pid: 4242,
    })
    backend.api.jitStop.mockResolvedValue(undefined)
    backend.api.crashReportsList.mockResolvedValue([{
      name: 'LargeReport.ips',
      path: '/LargeReport.ips',
      kind: 'IPS',
      process: 'LargeReport',
      sizeBytes: 6 * 1024 * 1024,
      modified: '2026-07-26 23:00:00',
    }])
    backend.api.crashReportRead.mockResolvedValue({
      path: '/LargeReport.ips',
      content: 'preview content',
      truncated: true,
      sizeBytes: 6 * 1024 * 1024,
    })
    backend.api.crashReportExport.mockResolvedValue(undefined)
    backend.dialogs.saveFile.mockResolvedValue('/tmp/LargeReport.ips')
    backend.events.ddiProgress.mockResolvedValue(vi.fn())
  })

  it('starts Location with the selected device and clears it when leaving the page', async () => {
    const user = userEvent.setup()
    const view = render(<Location desktop udid="device-1" onToast={vi.fn()} />)

    await user.click(screen.getByRole('button', { name: 'Start simulation' }))
    await waitFor(() => {
      expect(backend.api.locationStart).toHaveBeenCalledWith(37.7749, -122.4194, 'device-1')
    })
    expect(screen.getByText(/Location override active · DVT\/RSD/)).toBeInTheDocument()

    view.unmount()
    await waitFor(() => expect(backend.api.locationStop).toHaveBeenCalledOnce())
  })

  it('starts JIT for a debuggable app and detaches when leaving the page', async () => {
    const user = userEvent.setup()
    const view = render(<Developer desktop device={device} onToast={vi.fn()} />)

    await screen.findByRole('option', { name: /Debug App/ })
    await user.click(screen.getByRole('button', { name: 'Start JIT session' }))
    await waitFor(() => {
      expect(backend.api.jitStart).toHaveBeenCalledWith('com.example.debug', 'device-1')
    })
    expect(screen.getByText(/debugserver attached · pid 4242/)).toBeInTheDocument()

    view.unmount()
    await waitFor(() => expect(backend.api.jitStop).toHaveBeenCalledOnce())
  })

  it('labels a large crash preview as truncated while exporting the original report', async () => {
    const user = userEvent.setup()
    render(<CrashReports desktop udid="device-1" onToast={vi.fn()} />)

    expect(await screen.findByText('Preview limited to 4 MB. Export saves the complete report.')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Export' }))

    await waitFor(() => {
      expect(backend.api.crashReportExport).toHaveBeenCalledWith(
        '/LargeReport.ips',
        '/tmp/LargeReport.ips',
        'device-1',
      )
    })
  })
})
