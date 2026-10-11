import { act, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { DeviceSummary } from '../api'

const backend = vi.hoisted(() => ({
  api: {
    deviceList: vi.fn(),
    deviceSelect: vi.fn(),
    deviceDisconnect: vi.fn(),
    devicePair: vi.fn(),
    deviceMonitorStart: vi.fn(),
    deviceMonitorStop: vi.fn(),
    ddiEnsure: vi.fn(),
  },
  events: { deviceChanged: vi.fn() },
}))
vi.mock('../api', () => ({
  ...backend,
  errorMessage: (error: unknown) => error instanceof Error ? error.message : String(error),
}))
import { useDeviceSession } from './useDeviceSession'

const summary = (id: string, overrides: Partial<DeviceSummary> = {}): DeviceSummary => ({
  id,
  udid: `udid-${id}`,
  deviceId: 1,
  name: `Device ${id}`,
  model: 'iPhone',
  ios: '17.0',
  connection: 'USB',
  transports: ['USB'],
  paired: true,
  connectable: true,
  ...overrides,
})

function Harness({ desktop = true, onError = vi.fn(), onSessionLost = vi.fn() }) {
  const session = useDeviceSession({ desktop, onError, onSessionLost })
  return (
    <div>
      <span data-testid="connection">{session.connection}</span>
      <span data-testid="device">{session.device.id}</span>
      <span data-testid="catalog">{session.catalog.length}</span>
      <button onClick={() => void session.refresh()}>refresh</button>
      <button onClick={() => void session.disconnect()}>disconnect</button>
      <button onClick={session.dismiss}>dismiss</button>
    </div>
  )
}

describe('useDeviceSession', () => {
  beforeEach(() => {
    vi.resetAllMocks()
    backend.api.deviceList.mockResolvedValue([])
    backend.api.deviceSelect.mockResolvedValue(undefined)
    backend.api.deviceDisconnect.mockResolvedValue(undefined)
    backend.api.deviceMonitorStart.mockResolvedValue(undefined)
    backend.api.deviceMonitorStop.mockResolvedValue(undefined)
    backend.api.ddiEnsure.mockResolvedValue(undefined)
    backend.events.deviceChanged.mockResolvedValue(vi.fn())
  })

  it('selects the first usable device and mounts its disk image', async () => {
    backend.api.deviceList.mockResolvedValue([
      summary('a', { paired: false }),
      summary('b'),
    ])

    render(<Harness />)

    await waitFor(() => expect(screen.getByTestId('connection')).toHaveTextContent('connected'))
    expect(screen.getByTestId('device')).toHaveTextContent('b')
    expect(backend.api.deviceSelect).toHaveBeenCalledWith('b')
    await waitFor(() => expect(backend.api.ddiEnsure).toHaveBeenCalledWith('udid-b'))
  })

  /// A device that discovery can see but not reach must not read as a session:
  /// pages would then run against a device with no transport.
  it('leaves the session when the only device is unusable', async () => {
    const onSessionLost = vi.fn()
    backend.api.deviceList.mockResolvedValue([summary('a', { paired: false, connectable: false })])

    render(<Harness onSessionLost={onSessionLost} />)

    await waitFor(() => expect(screen.getByTestId('connection')).toHaveTextContent('detected'))
    expect(onSessionLost).toHaveBeenCalled()
    expect(backend.api.deviceDisconnect).toHaveBeenCalled()
    expect(backend.api.ddiEnsure).not.toHaveBeenCalled()
  })

  it('ends the session when every device disappears', async () => {
    const onSessionLost = vi.fn()
    backend.api.deviceList.mockResolvedValue([summary('a')])
    let notify: (() => void) | undefined
    backend.events.deviceChanged.mockImplementation((handler: () => void) => {
      notify = handler
      return Promise.resolve(vi.fn())
    })

    render(<Harness onSessionLost={onSessionLost} />)
    await waitFor(() => expect(screen.getByTestId('connection')).toHaveTextContent('connected'))

    backend.api.deviceList.mockResolvedValue([])
    await act(async () => { notify?.() })

    await waitFor(() => expect(screen.getByTestId('connection')).toHaveTextContent('none'))
    expect(onSessionLost).toHaveBeenCalled()
  })

  /// Two device events arriving during one listing used to let the older
  /// catalog win. Refreshes are serialized, and the newest snapshot is kept.
  it('serializes overlapping refreshes and keeps the newest catalog', async () => {
    let release!: (value: DeviceSummary[]) => void
    const first = new Promise<DeviceSummary[]>((done) => { release = done })
    backend.api.deviceList.mockReturnValueOnce(first)
    backend.api.deviceList.mockResolvedValue([summary('b'), summary('c')])
    let notify: (() => void) | undefined
    backend.events.deviceChanged.mockImplementation((handler: () => void) => {
      notify = handler
      return Promise.resolve(vi.fn())
    })

    render(<Harness />)
    await waitFor(() => expect(notify).toBeDefined())

    act(() => { notify?.() })
    await act(async () => release([summary('a')]))

    await waitFor(() => expect(screen.getByTestId('catalog')).toHaveTextContent('2'))
    expect(screen.getByTestId('device')).toHaveTextContent('b')
  })

  it('reports a listing failure and holds no session', async () => {
    const onError = vi.fn()
    backend.api.deviceList.mockRejectedValue(new Error('usbmuxd is unavailable'))

    render(<Harness onError={onError} />)

    await waitFor(() => expect(onError).toHaveBeenCalledWith('usbmuxd is unavailable'))
    expect(screen.getByTestId('connection')).toHaveTextContent('none')
  })

  it('dismisses the session view without disconnecting the backend', async () => {
    const onSessionLost = vi.fn()
    backend.api.deviceList.mockResolvedValue([summary('a')])

    render(<Harness onSessionLost={onSessionLost} />)
    await waitFor(() => expect(screen.getByTestId('connection')).toHaveTextContent('connected'))

    backend.api.deviceDisconnect.mockClear()
    onSessionLost.mockClear()
    act(() => screen.getByText('dismiss').click())

    expect(screen.getByTestId('connection')).toHaveTextContent('none')
    expect(backend.api.deviceDisconnect).not.toHaveBeenCalled()
    expect(onSessionLost).not.toHaveBeenCalled()
  })

  it('stops device monitoring on unmount', async () => {
    const stop = vi.fn()
    backend.events.deviceChanged.mockResolvedValue(stop)

    const view = render(<Harness />)
    await waitFor(() => expect(backend.api.deviceMonitorStart).toHaveBeenCalled())

    view.unmount()
    await waitFor(() => expect(stop).toHaveBeenCalled())
    expect(backend.api.deviceMonitorStop).toHaveBeenCalled()
  })

  it('touches no backend in browser demo mode and reveals the demo device', async () => {
    render(<Harness desktop={false} />)
    expect(screen.getByTestId('connection')).toHaveTextContent('connected')
    expect(backend.api.deviceMonitorStart).not.toHaveBeenCalled()

    act(() => screen.getByText('dismiss').click())
    act(() => screen.getByText('refresh').click())

    await waitFor(() => expect(screen.getByTestId('connection')).toHaveTextContent('detected'))
    expect(backend.api.deviceList).not.toHaveBeenCalled()
  })
})
