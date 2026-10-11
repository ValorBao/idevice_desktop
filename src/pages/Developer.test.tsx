import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Device } from '../data'
import { Developer } from './Developer'

const backend = vi.hoisted(() => ({
  api: {
    developerStatus: vi.fn(),
    appsDebuggable: vi.fn(),
    ddiDownload: vi.fn(),
    jitStart: vi.fn(),
    jitStop: vi.fn(),
    developerReveal: vi.fn(),
    developerEnable: vi.fn(),
    developerAccept: vi.fn(),
  },
  dialogs: {
    confirmDestructive: vi.fn(),
  },
  events: {
    ddiProgress: vi.fn(),
  },
}))

vi.mock('../api', () => ({
  ...backend,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}))

const legacyDevice: Device = {
  id: 'device-legacy',
  udid: 'udid-legacy',
  name: 'Legacy iPhone',
  model: 'iPhone10,1',
  modelId: 'iPhone10,1',
  ios: '14.2',
  build: '18B92',
  serial: 'serial',
  ecid: 'ecid',
  chip: 'A11',
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

const modernDevice: Device = { ...legacyDevice, id: 'device-modern', udid: 'udid-modern', ios: '17.0' }

const status = (ddiMounted: boolean, developerMode: boolean | null = true) => ({
  developerMode,
  ddiMounted,
  ddiImages: null,
  rsdAvailable: ddiMounted,
})

describe('developer disk image download', () => {
  beforeEach(() => {
    backend.api.developerStatus.mockResolvedValue(status(false))
    backend.api.appsDebuggable.mockResolvedValue([])
    backend.api.ddiDownload.mockResolvedValue(undefined)
    backend.api.jitStop.mockResolvedValue(undefined)
    backend.api.developerEnable.mockResolvedValue(undefined)
    backend.api.developerReveal.mockResolvedValue(undefined)
    backend.api.developerAccept.mockResolvedValue(undefined)
    backend.dialogs.confirmDestructive.mockResolvedValue(true)
    backend.events.ddiProgress.mockResolvedValue(vi.fn())
  })

  it('offers the one-time download only when a legacy device has no image', async () => {
    render(<Developer desktop device={legacyDevice} onToast={vi.fn()} />)

    const download = await screen.findByRole('button', { name: 'Download and mount' })
    expect(download).toBeEnabled()
    expect(screen.getByText(/19 MB Developer Disk Image/)).toBeInTheDocument()
  })

  it('does not offer a download for iOS 17, which builds its own image', async () => {
    render(<Developer desktop device={modernDevice} onToast={vi.fn()} />)

    await waitFor(() => expect(backend.api.developerStatus).toHaveBeenCalled())
    expect(screen.queryByRole('button', { name: 'Download and mount' })).not.toBeInTheDocument()
    expect(screen.getByText(/Mounting is automatic on connection/)).toBeInTheDocument()
  })

  it('does not offer a download once the image is mounted', async () => {
    backend.api.developerStatus.mockResolvedValue(status(true))
    render(<Developer desktop device={legacyDevice} onToast={vi.fn()} />)

    await screen.findByText(/Mounted automatically when this device was selected/)
    expect(screen.queryByRole('button', { name: 'Download and mount' })).not.toBeInTheDocument()
  })

  it('sends the selected device and reports the installed image', async () => {
    const user = userEvent.setup()
    const onToast = vi.fn()
    backend.api.developerStatus
      .mockResolvedValueOnce(status(false))
      .mockResolvedValue(status(true))
    render(<Developer desktop device={legacyDevice} onToast={onToast} />)

    await user.click(await screen.findByRole('button', { name: 'Download and mount' }))

    expect(backend.api.ddiDownload).toHaveBeenCalledWith('udid-legacy')
    await waitFor(() => expect(onToast).toHaveBeenCalledWith('Developer Disk Image installed and mounted'))
    // The refreshed status has an image, so the action retires itself.
    await waitFor(() => expect(screen.queryByRole('button', { name: 'Download and mount' })).not.toBeInTheDocument())
  })

  it('reports a failed download and leaves the action usable', async () => {
    const user = userEvent.setup()
    const onToast = vi.fn()
    backend.api.ddiDownload.mockRejectedValue(new Error('does not match its pinned checksum'))
    render(<Developer desktop device={legacyDevice} onToast={onToast} />)

    await user.click(await screen.findByRole('button', { name: 'Download and mount' }))

    await waitFor(() => expect(onToast).toHaveBeenCalledWith('does not match its pinned checksum'))
    expect(screen.getByRole('button', { name: 'Download and mount' })).toBeEnabled()
  })

  it('asks for Developer Mode instead of a download when it is off', async () => {
    backend.api.developerStatus.mockResolvedValue(status(false, false))
    render(<Developer desktop device={legacyDevice} onToast={vi.fn()} />)

    expect(await screen.findByText('Turn on Developer Mode on Legacy iPhone')).toBeInTheDocument()
    expect(screen.getByText(/Settings › Privacy & Security › Developer Mode/)).toBeInTheDocument()
    // Downloading 19 MB for a device that cannot mount it yet is not offered.
    expect(screen.queryByRole('button', { name: 'Download and mount' })).not.toBeInTheDocument()
  })

  it('does not ask for Developer Mode on releases that have none', async () => {
    // iOS 14 and 15 have no AMFI service, so the status is null rather than off.
    backend.api.developerStatus.mockResolvedValue(status(false, null))
    render(<Developer desktop device={legacyDevice} onToast={vi.fn()} />)

    expect(await screen.findByRole('button', { name: 'Download and mount' })).toBeInTheDocument()
    expect(screen.queryByText(/Turn on Developer Mode/)).not.toBeInTheDocument()
  })

  it('warns that the device restarts before enabling Developer Mode', async () => {
    const user = userEvent.setup()
    const onToast = vi.fn()
    render(<Developer desktop device={legacyDevice} onToast={onToast} />)

    await user.click(screen.getByRole('button', { name: 'Enable Developer Mode' }))

    const [message, label] = backend.dialogs.confirmDestructive.mock.calls[0]
    expect(message).toContain('restarts Legacy iPhone')
    expect(message).toContain('Accept after reboot')
    expect(label).toBe('Restart and enable')
    expect(backend.api.developerEnable).toHaveBeenCalledWith('udid-legacy')
    await waitFor(() => expect(onToast).toHaveBeenCalledWith('Legacy iPhone is restarting · unlock it, then choose Accept after reboot'))
  })

  it('points at Settings when the device refuses because it has a passcode', async () => {
    const user = userEvent.setup()
    const onToast = vi.fn()
    backend.api.developerEnable.mockRejectedValue(new Error('unknown error `Device has a passcode set` returned from device'))
    render(<Developer desktop device={legacyDevice} onToast={onToast} />)

    await user.click(screen.getByRole('button', { name: 'Enable Developer Mode' }))

    // The switch is revealed so the instruction can actually be followed.
    await waitFor(() => expect(backend.api.developerReveal).toHaveBeenCalledWith('udid-legacy'))
    expect(onToast).toHaveBeenCalledWith(expect.stringContaining('Settings › Privacy & Security › Developer Mode'))
    expect(onToast).not.toHaveBeenCalledWith(expect.stringContaining('is restarting'))
  })

  it('sends nothing when the restart is declined', async () => {
    const user = userEvent.setup()
    const onToast = vi.fn()
    backend.dialogs.confirmDestructive.mockResolvedValue(false)
    render(<Developer desktop device={legacyDevice} onToast={onToast} />)

    await user.click(screen.getByRole('button', { name: 'Enable Developer Mode' }))

    await waitFor(() => expect(backend.dialogs.confirmDestructive).toHaveBeenCalled())
    expect(backend.api.developerEnable).not.toHaveBeenCalled()
    expect(onToast).not.toHaveBeenCalled()
  })

  it('does not warn about a restart for the actions that do not cause one', async () => {
    const user = userEvent.setup()
    render(<Developer desktop device={legacyDevice} onToast={vi.fn()} />)

    await user.click(screen.getByRole('button', { name: 'Show Developer Mode setting' }))
    await user.click(screen.getByRole('button', { name: 'Accept after reboot' }))

    expect(backend.dialogs.confirmDestructive).not.toHaveBeenCalled()
    expect(backend.api.developerReveal).toHaveBeenCalledWith('udid-legacy')
    expect(backend.api.developerAccept).toHaveBeenCalledWith('udid-legacy')
  })

  it('does not reach the backend in browser demo mode', async () => {
    const user = userEvent.setup()
    const onToast = vi.fn()
    render(<Developer desktop={false} device={legacyDevice} onToast={onToast} />)

    // Demo mode reports a mounted image, so the download is not offered at all.
    expect(screen.queryByRole('button', { name: 'Download and mount' })).not.toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Enable Developer Mode' }))
    expect(backend.api.ddiDownload).not.toHaveBeenCalled()
    expect(backend.api.developerEnable).not.toHaveBeenCalled()
  })
})
