import { act, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { PerformanceSample, PerformanceStatus } from '../api'

const backend = vi.hoisted(() => ({
  api: {
    performanceStart: vi.fn(),
    performanceStop: vi.fn(),
    performanceExportCsv: vi.fn(),
  },
  dialogs: {
    saveFile: vi.fn(),
  },
  events: {
    performanceSample: vi.fn(),
    performanceStatus: vi.fn(),
  },
}))

vi.mock('../api', () => ({
  ...backend,
  errorMessage: (error: unknown) => String(error),
}))

import { Performance } from './Performance'

let sampleHandler: ((sample: PerformanceSample) => void) | undefined
let statusHandler: ((status: PerformanceStatus) => void) | undefined

const sample = (overrides: Partial<PerformanceSample> = {}): PerformanceSample => ({
  sequence: 1,
  timestampMs: 1_786_176_000_000,
  intervalMs: 1_000,
  transport: 'DVT Sysmontap · RemotePairing/RSD',
  systemCpuPercent: 31.25,
  processes: [{
    pid: 4242,
    name: 'Example App',
    identity: 'unique:example-1',
    cpuPercent: 12.5,
    memoryBytes: 64 * 1024 * 1024,
  }],
  ...overrides,
})

const emitRunning = () => act(() => statusHandler?.({
  state: 'running',
  message: null,
  transport: 'DVT Sysmontap · RemotePairing/RSD',
  intervalMs: 1_000,
}))

describe('Performance workflow', () => {
  beforeEach(() => {
    sampleHandler = undefined
    statusHandler = undefined
    backend.api.performanceStart.mockResolvedValue(undefined)
    backend.api.performanceStop.mockResolvedValue(undefined)
    backend.api.performanceExportCsv.mockResolvedValue(undefined)
    backend.dialogs.saveFile.mockResolvedValue('/tmp/performance-example.csv')
    backend.events.performanceSample.mockImplementation((handler: (sample: PerformanceSample) => void) => {
      sampleHandler = handler
      return Promise.resolve(vi.fn())
    })
    backend.events.performanceStatus.mockImplementation((handler: (status: PerformanceStatus) => void) => {
      statusHandler = handler
      return Promise.resolve(vi.fn())
    })
  })

  it('registers listeners before starting and renders live process metrics', async () => {
    render(<Performance desktop udid="device-1" onToast={vi.fn()} />)

    await waitFor(() => expect(backend.api.performanceStart).toHaveBeenCalledWith('device-1', 1_000))
    expect(sampleHandler).toBeDefined()
    expect(statusHandler).toBeDefined()

    emitRunning()
    act(() => sampleHandler?.(sample()))

    expect(await screen.findAllByText('Example App')).not.toHaveLength(0)
    expect(screen.getAllByText('12.5%')).not.toHaveLength(0)
    expect(screen.getAllByText('64 MB')).not.toHaveLength(0)
    expect(screen.getByText('31.3%')).toBeInTheDocument()
    expect(screen.getByText('live')).toBeInTheDocument()
  })

  it('keeps missing device metrics unavailable instead of displaying zero', async () => {
    render(<Performance desktop udid="device-1" onToast={vi.fn()} />)
    await waitFor(() => expect(backend.api.performanceStart).toHaveBeenCalledOnce())

    act(() => sampleHandler?.(sample({
      systemCpuPercent: null,
      processes: [{
        pid: 4242,
        name: 'Example App',
        identity: 'unique:example-1',
        cpuPercent: null,
        memoryBytes: null,
      }],
    })))

    expect(await screen.findAllByText('—')).toHaveLength(5)
    expect(screen.queryByText('0.0%')).not.toBeInTheDocument()
    expect(screen.queryByText('0 B')).not.toBeInTheDocument()
  })

  it('stops while paused and resumes with the newly selected interval', async () => {
    const user = userEvent.setup()
    render(<Performance desktop udid="device-1" onToast={vi.fn()} />)
    await waitFor(() => expect(backend.api.performanceStart).toHaveBeenCalledOnce())
    emitRunning()

    await user.click(screen.getByRole('button', { name: 'Pause' }))
    await waitFor(() => expect(backend.api.performanceStop).toHaveBeenCalledOnce())
    await user.selectOptions(screen.getByRole('combobox', { name: 'Sample interval' }), '2000')
    await user.click(screen.getByRole('button', { name: 'Resume' }))

    await waitFor(() => {
      expect(backend.api.performanceStart).toHaveBeenLastCalledWith('device-1', 2_000)
    })
  })

  it('does not start a late stream after pausing during listener registration', async () => {
    const user = userEvent.setup()
    let releaseSampleListener: (() => void) | undefined
    let releaseStatusListener: (() => void) | undefined
    backend.events.performanceSample.mockImplementation((handler: (sample: PerformanceSample) => void) => {
      sampleHandler = handler
      return new Promise<() => void>((resolve) => {
        releaseSampleListener = () => resolve(vi.fn())
      })
    })
    backend.events.performanceStatus.mockImplementation((handler: (status: PerformanceStatus) => void) => {
      statusHandler = handler
      return new Promise<() => void>((resolve) => {
        releaseStatusListener = () => resolve(vi.fn())
      })
    })
    render(<Performance desktop udid="device-1" onToast={vi.fn()} />)

    await user.click(screen.getByRole('button', { name: 'Pause' }))
    await waitFor(() => expect(backend.api.performanceStop).toHaveBeenCalledOnce())
    await act(async () => {
      releaseSampleListener?.()
      releaseStatusListener?.()
    })

    expect(backend.api.performanceStart).not.toHaveBeenCalled()
    expect(screen.getByRole('button', { name: 'Resume' })).toBeInTheDocument()
  })

  it('exports the selected process history through the desktop backend', async () => {
    const user = userEvent.setup()
    const onToast = vi.fn()
    render(<Performance desktop udid="device-1" onToast={onToast} />)
    await waitFor(() => expect(backend.api.performanceStart).toHaveBeenCalledOnce())
    act(() => sampleHandler?.(sample()))

    await user.click(await screen.findByRole('button', { name: 'Export CSV' }))

    expect(backend.dialogs.saveFile).toHaveBeenCalledWith('performance-Example-App-4242.csv')
    await waitFor(() => {
      expect(backend.api.performanceExportCsv).toHaveBeenCalledWith(
        '/tmp/performance-example.csv',
        [{
          timestampMs: 1_786_176_000_000,
          pid: 4242,
          name: 'Example App',
          identity: 'unique:example-1',
          cpuPercent: 12.5,
          memoryBytes: 64 * 1024 * 1024,
        }],
      )
    })
    expect(onToast).toHaveBeenCalledWith('Exported 1 performance samples')
  })

  it('stops sampling when leaving the page', async () => {
    const view = render(<Performance desktop udid="device-1" onToast={vi.fn()} />)
    await waitFor(() => expect(backend.api.performanceStart).toHaveBeenCalledOnce())

    view.unmount()

    await waitFor(() => expect(backend.api.performanceStop).toHaveBeenCalledOnce())
  })
})
