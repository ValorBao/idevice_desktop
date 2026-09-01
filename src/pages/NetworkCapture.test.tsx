import { act, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { NetworkCaptureProgress, NetworkCaptureStatus } from '../api'

const backend = vi.hoisted(() => ({
  api: {
    networkCaptureStart: vi.fn(),
    networkCaptureStop: vi.fn(),
    networkCaptureCancel: vi.fn(),
  },
  dialogs: {
    pcapDestination: vi.fn(),
  },
  events: {
    networkCaptureProgress: vi.fn(),
    networkCaptureStatus: vi.fn(),
  },
}))

vi.mock('../api', () => ({
  ...backend,
  errorMessage: (error: unknown) => String(error),
}))

import { NetworkCapture } from './NetworkCapture'

let progressHandler: ((progress: NetworkCaptureProgress) => void) | undefined
let statusHandler: ((status: NetworkCaptureStatus) => void) | undefined

const status = (state: string, message: string | null = null): NetworkCaptureStatus => ({
  state,
  message,
  destination: '/tmp/device-traffic.pcap',
  transport: state === 'running' ? 'Pcapd · USB' : null,
  filter: { pid: 4242, interfaceName: 'en0' },
})

describe('Network Capture workflow', () => {
  beforeEach(() => {
    progressHandler = undefined
    statusHandler = undefined
    backend.api.networkCaptureStart.mockResolvedValue(undefined)
    backend.api.networkCaptureStop.mockResolvedValue(undefined)
    backend.api.networkCaptureCancel.mockResolvedValue(undefined)
    backend.dialogs.pcapDestination.mockResolvedValue('/tmp/device-traffic.pcap')
    backend.events.networkCaptureProgress.mockImplementation((handler: (progress: NetworkCaptureProgress) => void) => {
      progressHandler = handler
      return Promise.resolve(vi.fn())
    })
    backend.events.networkCaptureStatus.mockImplementation((handler: (status: NetworkCaptureStatus) => void) => {
      statusHandler = handler
      return Promise.resolve(vi.fn())
    })
  })

  it('registers listeners before starting with the selected filters and destination', async () => {
    const user = userEvent.setup()
    render(<NetworkCapture desktop udid="device-1" onToast={vi.fn()} />)

    const start = screen.getByRole('button', { name: 'Start Capture' })
    await waitFor(() => expect(start).toBeEnabled())
    await user.type(screen.getByLabelText('Process PID'), '4242')
    await user.type(screen.getByLabelText('Interface'), 'en0')
    await user.click(start)

    expect(progressHandler).toBeDefined()
    expect(statusHandler).toBeDefined()
    expect(backend.dialogs.pcapDestination).toHaveBeenCalledOnce()
    await waitFor(() => {
      expect(backend.api.networkCaptureStart).toHaveBeenCalledWith(
        '/tmp/device-traffic.pcap',
        'device-1',
        4242,
        'en0',
      )
    })
  })

  it('renders throttled capture progress without retaining packet payloads', async () => {
    const user = userEvent.setup()
    render(<NetworkCapture desktop udid="device-1" onToast={vi.fn()} />)
    await waitFor(() => expect(screen.getByRole('button', { name: 'Start Capture' })).toBeEnabled())
    await user.click(screen.getByRole('button', { name: 'Start Capture' }))
    act(() => statusHandler?.({ ...status('running'), filter: { pid: null, interfaceName: null } }))
    act(() => progressHandler?.({
      packets: 1_234,
      bytes: 2 * 1024 * 1024,
      outputBytes: 2 * 1024 * 1024 + 24 + 1_234 * 16,
      elapsedMs: 65_000,
      lastProcess: 'MobileSafari',
      lastInterface: 'en0',
    }))

    expect(await screen.findByText('1,234')).toBeInTheDocument()
    expect(screen.getByText('01:05')).toBeInTheDocument()
    expect(screen.getByText('MobileSafari')).toBeInTheDocument()
    expect(screen.getByText('Pcapd · USB')).toBeInTheDocument()
  })

  it('stops and reports the final saved destination', async () => {
    const user = userEvent.setup()
    const onToast = vi.fn()
    render(<NetworkCapture desktop udid="device-1" onToast={onToast} />)
    await waitFor(() => expect(screen.getByRole('button', { name: 'Start Capture' })).toBeEnabled())
    await user.click(screen.getByRole('button', { name: 'Start Capture' }))
    act(() => statusHandler?.(status('running')))

    await user.click(screen.getByRole('button', { name: 'Stop & Save' }))
    expect(backend.api.networkCaptureStop).toHaveBeenCalledOnce()
    act(() => statusHandler?.(status('completed')))

    expect(await screen.findByText(/Capture saved to/)).toHaveTextContent('/tmp/device-traffic.pcap')
    expect(onToast).toHaveBeenCalledWith('device-traffic.pcap saved')
  })

  it('cancels and deletes the partial capture', async () => {
    const user = userEvent.setup()
    render(<NetworkCapture desktop udid="device-1" onToast={vi.fn()} />)
    await waitFor(() => expect(screen.getByRole('button', { name: 'Start Capture' })).toBeEnabled())
    await user.click(screen.getByRole('button', { name: 'Start Capture' }))
    act(() => statusHandler?.(status('running')))

    await user.click(screen.getByRole('button', { name: 'Cancel & Delete' }))
    expect(backend.api.networkCaptureCancel).toHaveBeenCalledOnce()
    act(() => statusHandler?.(status('cancelled')))

    expect(await screen.findByText('The partial capture was deleted.')).toBeInTheDocument()
  })

  it('rejects an invalid PID before opening the save dialog', async () => {
    const user = userEvent.setup()
    const onToast = vi.fn()
    render(<NetworkCapture desktop udid="device-1" onToast={onToast} />)
    await waitFor(() => expect(screen.getByRole('button', { name: 'Start Capture' })).toBeEnabled())
    await user.type(screen.getByLabelText('Process PID'), '0')
    await user.click(screen.getByRole('button', { name: 'Start Capture' }))

    expect(onToast).toHaveBeenCalledWith('PID must be a whole number greater than zero')
    expect(backend.dialogs.pcapDestination).not.toHaveBeenCalled()
    expect(backend.api.networkCaptureStart).not.toHaveBeenCalled()
  })

  it('cancels an active capture when leaving the page', async () => {
    const user = userEvent.setup()
    const view = render(<NetworkCapture desktop udid="device-1" onToast={vi.fn()} />)
    await waitFor(() => expect(screen.getByRole('button', { name: 'Start Capture' })).toBeEnabled())
    await user.click(screen.getByRole('button', { name: 'Start Capture' }))
    await waitFor(() => expect(backend.api.networkCaptureStart).toHaveBeenCalledOnce())

    view.unmount()

    await waitFor(() => expect(backend.api.networkCaptureCancel).toHaveBeenCalledOnce())
  })
})
