import { act, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { LiveScreenFrame, LiveScreenStatus } from '../api'

const backend = vi.hoisted(() => ({
  api: {
    liveScreenStart: vi.fn(),
    liveScreenStop: vi.fn(),
    liveScreenExportFrame: vi.fn(),
  },
  dialogs: {
    pngDestination: vi.fn(),
  },
  events: {
    liveScreenFrame: vi.fn(),
    liveScreenStatus: vi.fn(),
  },
}))

vi.mock('../api', () => ({
  ...backend,
  errorMessage: (error: unknown) => String(error),
}))

import { LiveScreen } from './LiveScreen'

let frameHandler: ((frame: LiveScreenFrame) => void) | undefined
let statusHandler: ((status: LiveScreenStatus) => void) | undefined

const frame: LiveScreenFrame = {
  sequence: 12,
  timestampMs: 1_700_000_000_000,
  width: 1_290,
  height: 2_796,
  bytes: 256 * 1024,
  fps: 1.92,
  dataUrl: 'data:image/png;base64,cG5n',
}

const status = (state: string): LiveScreenStatus => ({
  state,
  message: null,
  transport: state === 'running' ? 'DVT Screenshot · CoreDeviceProxy/RSD' : null,
  targetFps: 2,
})

describe('Live Screen workflow', () => {
  beforeEach(() => {
    frameHandler = undefined
    statusHandler = undefined
    Object.defineProperty(document, 'hidden', { configurable: true, value: false })
    backend.api.liveScreenStart.mockResolvedValue(undefined)
    backend.api.liveScreenStop.mockResolvedValue(undefined)
    backend.api.liveScreenExportFrame.mockResolvedValue(undefined)
    backend.dialogs.pngDestination.mockResolvedValue('/tmp/current-frame.png')
    backend.events.liveScreenFrame.mockImplementation((handler: (next: LiveScreenFrame) => void) => {
      frameHandler = handler
      return Promise.resolve(vi.fn())
    })
    backend.events.liveScreenStatus.mockImplementation((handler: (next: LiveScreenStatus) => void) => {
      statusHandler = handler
      return Promise.resolve(vi.fn())
    })
  })

  it('registers frame and status listeners before starting the selected device', async () => {
    const user = userEvent.setup()
    render(<LiveScreen desktop udid="device-1" onToast={vi.fn()} />)

    const start = screen.getByRole('button', { name: 'Start Preview' })
    await waitFor(() => expect(start).toBeEnabled())
    await user.click(start)

    expect(frameHandler).toBeDefined()
    expect(statusHandler).toBeDefined()
    expect(backend.api.liveScreenStart).toHaveBeenCalledWith('device-1')
  })

  it('shows the latest frame and measured device details', async () => {
    render(<LiveScreen desktop udid="device-1" onToast={vi.fn()} />)
    await waitFor(() => expect(screen.getByRole('button', { name: 'Start Preview' })).toBeEnabled())

    act(() => statusHandler?.(status('running')))
    act(() => frameHandler?.(frame))

    expect(await screen.findByRole('img', { name: 'Live device screen' })).toHaveAttribute('src', frame.dataUrl)
    expect(screen.getByText('1290 × 2796')).toBeInTheDocument()
    expect(screen.getByText('1.9')).toBeInTheDocument()
    expect(screen.getByText('256 KB')).toBeInTheDocument()
    expect(screen.getByText('DVT Screenshot · CoreDeviceProxy/RSD')).toBeInTheDocument()
  })

  it('stops an active preview and keeps the current still visible', async () => {
    const user = userEvent.setup()
    render(<LiveScreen desktop udid="device-1" onToast={vi.fn()} />)
    await waitFor(() => expect(screen.getByRole('button', { name: 'Start Preview' })).toBeEnabled())
    await user.click(screen.getByRole('button', { name: 'Start Preview' }))
    act(() => statusHandler?.(status('running')))
    act(() => frameHandler?.(frame))

    await user.click(screen.getByRole('button', { name: 'Stop Preview' }))

    expect(backend.api.liveScreenStop).toHaveBeenCalledOnce()
    expect(await screen.findAllByText('paused')).toHaveLength(2)
    expect(screen.getByRole('img', { name: 'Live device screen' })).toBeInTheDocument()
  })

  it('exports the exact backend frame to the chosen PNG destination', async () => {
    const user = userEvent.setup()
    const onToast = vi.fn()
    render(<LiveScreen desktop udid="device-1" onToast={onToast} />)
    await waitFor(() => expect(screen.getByRole('button', { name: 'Start Preview' })).toBeEnabled())
    act(() => frameHandler?.(frame))

    await user.click(screen.getByRole('button', { name: 'Save Frame' }))

    expect(backend.dialogs.pngDestination).toHaveBeenCalledOnce()
    expect(backend.api.liveScreenExportFrame).toHaveBeenCalledWith('/tmp/current-frame.png')
    expect(onToast).toHaveBeenCalledWith('Current frame saved')
  })

  it('pauses the owned session when the window becomes hidden', async () => {
    const user = userEvent.setup()
    render(<LiveScreen desktop udid="device-1" onToast={vi.fn()} />)
    await waitFor(() => expect(screen.getByRole('button', { name: 'Start Preview' })).toBeEnabled())
    await user.click(screen.getByRole('button', { name: 'Start Preview' }))
    act(() => statusHandler?.(status('running')))

    Object.defineProperty(document, 'hidden', { configurable: true, value: true })
    act(() => document.dispatchEvent(new Event('visibilitychange')))

    await waitFor(() => expect(backend.api.liveScreenStop).toHaveBeenCalledOnce())
    expect(screen.getByText('Preview paused while the window was hidden.')).toBeInTheDocument()
  })

  it('stops an owned session when leaving the page', async () => {
    const user = userEvent.setup()
    const view = render(<LiveScreen desktop udid="device-1" onToast={vi.fn()} />)
    await waitFor(() => expect(screen.getByRole('button', { name: 'Start Preview' })).toBeEnabled())
    await user.click(screen.getByRole('button', { name: 'Start Preview' }))
    await waitFor(() => expect(backend.api.liveScreenStart).toHaveBeenCalledOnce())

    view.unmount()

    await waitFor(() => expect(backend.api.liveScreenStop).toHaveBeenCalledOnce())
  })
})
