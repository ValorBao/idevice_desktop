import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const backend = vi.hoisted(() => ({
  api: {
    processesList: vi.fn(),
    appsList: vi.fn(),
    appsDebuggable: vi.fn(),
    processLaunch: vi.fn(),
    processStop: vi.fn(),
  },
  dialogs: {
    confirmDestructive: vi.fn(),
  },
}))

vi.mock('../api', () => ({
  ...backend,
  errorMessage: (error: unknown) => String(error),
}))

import { Processes } from './Processes'

const process = {
  pid: 4242,
  name: 'Example App',
  executablePath: '/private/Example.app/Example',
  isApplication: true,
  canStop: true,
  identity: 'core:4242:/private/Example.app/Example',
}

const snapshot = {
  processes: [
    process,
    {
      pid: 93,
      name: 'locationd',
      executablePath: '/usr/libexec/locationd',
      isApplication: false,
      canStop: false,
      identity: 'core:93:/usr/libexec/locationd',
    },
  ],
  transport: 'CoreDevice AppService · RemotePairing/RSD',
  available: true,
  supportsLaunch: true,
  supportsStop: true,
  limitation: null,
}

describe('Processes workflow', () => {
  beforeEach(() => {
    backend.api.processesList.mockResolvedValue(snapshot)
    backend.api.appsList.mockResolvedValue([{
      bundleId: 'com.example.app',
      name: 'Example App',
      version: '1.0',
      sizeBytes: 1,
      system: false,
      iconDataUrl: null,
      raw: null,
    }])
    backend.api.appsDebuggable.mockResolvedValue([{
      bundleId: 'cn.gblw.AppsDump',
      name: 'AppsDump',
      version: '1.0',
      sizeBytes: 1,
      system: true,
      iconDataUrl: null,
      raw: null,
    }])
    backend.api.processLaunch.mockResolvedValue({
      pid: 5252,
      bundleId: 'com.example.app',
      transport: 'CoreDevice AppService · RemotePairing/RSD',
    })
    backend.api.processStop.mockResolvedValue(undefined)
    backend.dialogs.confirmDestructive.mockResolvedValue(true)
  })

  it('loads processes and protects rows outside installed user applications', async () => {
    render(<Processes desktop udid="device-1" onToast={vi.fn()} />)

    expect(await screen.findByText('Example App')).toBeInTheDocument()
    expect(screen.getByText('locationd')).toBeInTheDocument()
    expect(screen.getByText('Protected')).toBeInTheDocument()
    expect(backend.api.processesList).toHaveBeenCalledWith('device-1')
    expect(backend.api.appsList).toHaveBeenCalledWith('device-1')
    expect(backend.api.appsDebuggable).toHaveBeenCalledWith('device-1')
    expect(screen.getByRole('option', { name: /AppsDump/ })).toBeInTheDocument()
  })

  it('returns the opaque process identity when stopping a confirmed target', async () => {
    const user = userEvent.setup()
    render(<Processes desktop udid="device-1" onToast={vi.fn()} />)

    await user.click(await screen.findByRole('button', { name: 'Stop' }))

    expect(backend.dialogs.confirmDestructive).toHaveBeenCalledWith(
      'Stop “Example App” (pid 4242)? Unsaved work in this application may be lost.',
      'Stop Process',
    )
    await waitFor(() => {
      expect(backend.api.processStop).toHaveBeenCalledWith(4242, process.identity, 'device-1')
    })
  })

  it('keeps DVT fallback rows read-only and explains why stop is unavailable', async () => {
    backend.api.processesList.mockResolvedValueOnce({
      ...snapshot,
      transport: 'DVT DeviceInfo · RemotePairing/RSD',
      supportsStop: false,
      limitation: 'Stopping applications is disabled on the DVT fallback.',
    })
    render(<Processes desktop udid="dvt-device" onToast={vi.fn()} />)

    expect(await screen.findByText(/disabled on the DVT fallback/)).toBeInTheDocument()
    expect(screen.getByText('Read only')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Stop' })).not.toBeInTheDocument()
  })

  it('launches the selected installed application and refreshes', async () => {
    const user = userEvent.setup()
    const onToast = vi.fn()
    render(<Processes desktop udid="device-1" onToast={onToast} />)

    await screen.findByRole('option', { name: /Example App/ })
    await user.click(screen.getByRole('button', { name: 'Launch' }))

    await waitFor(() => {
      expect(backend.api.processLaunch).toHaveBeenCalledWith('com.example.app', 'device-1')
    })
    expect(onToast).toHaveBeenCalledWith('com.example.app launched as pid 5252')
    expect(backend.api.processesList.mock.calls.length).toBeGreaterThanOrEqual(2)
  })

  it('shows an explicit Legacy support boundary without dead controls', async () => {
    backend.api.processesList.mockResolvedValueOnce({
      processes: [],
      transport: 'Legacy',
      available: false,
      supportsLaunch: false,
      supportsStop: false,
      limitation: 'Process monitoring is unavailable on iOS 16 and earlier.',
    })
    render(<Processes desktop udid="legacy-device" onToast={vi.fn()} />)

    expect(await screen.findByText('Processes unavailable on this device')).toBeInTheDocument()
    expect(screen.getByText(/iOS 16 and earlier/)).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Launch' })).not.toBeInTheDocument()
  })
})
