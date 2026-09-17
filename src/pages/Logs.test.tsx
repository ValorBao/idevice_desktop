import { StrictMode } from 'react'
import { act, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { DeviceLog, LogStatus } from '../api'

const backend = vi.hoisted(() => ({
  api: { logsStart: vi.fn(), logsStop: vi.fn() },
  events: { logLine: vi.fn(), logStatus: vi.fn() },
}))
vi.mock('../api', () => ({ ...backend, errorMessage: (error: unknown) => error instanceof Error ? error.message : String(error) }))
import { Logs } from './Logs'

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((done) => { resolve = done })
  return { promise, resolve }
}

const line = (sessionId: string, udid: string, message: string): DeviceLog => ({
  sessionId, udid, message, timestamp: '12:00:00', level: 'INFO', process: 'App', pid: 1, subsystem: null, category: null,
})

describe('log session lifecycle', () => {
  let lines: ((payload: DeviceLog) => void)[]
  let statuses: ((payload: LogStatus) => void)[]
  beforeEach(() => {
    vi.resetAllMocks()
    lines = []
    statuses = []
    backend.api.logsStart.mockResolvedValue(undefined)
    backend.api.logsStop.mockResolvedValue(undefined)
    backend.events.logLine.mockImplementation((handler) => { lines.push(handler); return Promise.resolve(vi.fn()) })
    backend.events.logStatus.mockImplementation((handler) => { statuses.push(handler); return Promise.resolve(vi.fn()) })
  })

  it('releases subscriptions that resolve after unmount without starting a stream', async () => {
    const pending = deferred<() => void>()
    const stop = vi.fn()
    backend.events.logLine.mockReturnValue(pending.promise)
    const view = render(<Logs connected desktop udid="one" onToast={vi.fn()} />)
    view.unmount()
    await act(async () => pending.resolve(stop))
    expect(stop).toHaveBeenCalledOnce()
    expect(backend.api.logsStart).not.toHaveBeenCalled()
  })

  it('cleans the obsolete StrictMode subscription and only starts the live effect', async () => {
    const pending = deferred<() => void>()
    const stop = vi.fn()
    backend.events.logLine.mockReturnValueOnce(pending.promise)
    render(<StrictMode><Logs connected desktop udid="one" onToast={vi.fn()} /></StrictMode>)
    await waitFor(() => expect(backend.api.logsStart).toHaveBeenCalledTimes(1))
    await act(async () => pending.resolve(stop))
    expect(stop).toHaveBeenCalledOnce()
    expect(backend.api.logsStart).toHaveBeenCalledTimes(1)
  })

  it('stops a late start by its own ID without stopping the replacement', async () => {
    const pending = deferred<void>()
    backend.api.logsStart.mockReturnValueOnce(pending.promise)
    const onToast = vi.fn()
    const view = render(<Logs connected desktop udid="one" onToast={onToast} />)
    await waitFor(() => expect(backend.api.logsStart).toHaveBeenCalledTimes(1))
    const oldId = backend.api.logsStart.mock.calls[0][0]
    view.rerender(<Logs connected desktop udid="two" onToast={onToast} />)
    await waitFor(() => expect(backend.api.logsStart).toHaveBeenCalledTimes(2))
    const newId = backend.api.logsStart.mock.calls[1][0]
    await act(async () => pending.resolve())
    expect(backend.api.logsStop.mock.calls).toEqual([[oldId], [oldId]])
    expect(newId).not.toBe(oldId)
    act(() => {
      lines[lines.length - 1]?.(line(oldId, 'one', 'stale log'))
      lines[lines.length - 1]?.(line(newId, 'one', 'wrong device'))
      lines[lines.length - 1]?.(line(newId, 'two', 'current log'))
    })
    expect(screen.queryByText('stale log')).not.toBeInTheDocument()
    expect(screen.queryByText('wrong device')).not.toBeInTheDocument()
    expect(screen.getByText('current log')).toBeInTheDocument()
  })

  it('shows backend errors and ignores an obsolete stopped event', async () => {
    const onToast = vi.fn()
    render(<Logs connected desktop udid="one" onToast={onToast} />)
    await waitFor(() => expect(backend.api.logsStart).toHaveBeenCalled())
    const sessionId = backend.api.logsStart.mock.calls[0][0]
    act(() => statuses[0]({ sessionId, udid: 'one', state: 'running', message: null }))
    expect(screen.getByText('live')).toBeInTheDocument()
    act(() => statuses[0]({ sessionId: 'old', udid: 'one', state: 'stopped', message: null }))
    expect(screen.getByText('live')).toBeInTheDocument()
    act(() => statuses[0]({ sessionId, udid: 'one', state: 'error', message: 'Device disconnected' }))
    expect(screen.getByText('error', { selector: 'small' })).toBeInTheDocument()
    expect(onToast).toHaveBeenCalledWith('Device disconnected')
  })

  it('reports subscription failures without starting the backend', async () => {
    const onToast = vi.fn()
    backend.events.logStatus.mockRejectedValue(new Error('Cannot listen'))
    render(<Logs connected desktop udid="one" onToast={onToast} />)
    await waitFor(() => expect(onToast).toHaveBeenCalledWith('Cannot listen'))
    expect(backend.api.logsStart).not.toHaveBeenCalled()
  })
  it('starts in WebViews without crypto.randomUUID', async () => {
    const getRandomValues = crypto.getRandomValues.bind(crypto)
    vi.stubGlobal('crypto', { getRandomValues })
    try {
      render(<Logs connected desktop udid="one" onToast={vi.fn()} />)
      await waitFor(() => expect(backend.api.logsStart).toHaveBeenCalled())
      expect(backend.api.logsStart.mock.calls[0][0]).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/)
    } finally {
      vi.unstubAllGlobals()
    }
  })

})
