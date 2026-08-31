import { act, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { NotificationObservationEvent, NotificationObservationStatus } from '../api'

const backend = vi.hoisted(() => ({
  api: {
    notificationObservationStart: vi.fn(),
    notificationObservationStop: vi.fn(),
  },
  events: {
    notificationObservationEvent: vi.fn(),
    notificationObservationStatus: vi.fn(),
  },
}))

vi.mock('../api', () => ({
  ...backend,
  errorMessage: (error: unknown) => String(error),
}))

import { Notifications } from './Notifications'

let eventHandler: ((event: NotificationObservationEvent) => void) | undefined
let statusHandler: ((status: NotificationObservationStatus) => void) | undefined

const selectedName = 'com.apple.mobile.application_installed'

const currentSession = () => {
  const calls = backend.api.notificationObservationStart.mock.calls
  return String(calls[calls.length - 1]?.[1])
}

const observed = (sequence: number, sessionId = currentSession(), name = selectedName): NotificationObservationEvent => ({
  sessionId,
  sequence,
  timestampMs: Date.UTC(2026, 7, 9, 10, 11, 12, sequence % 1_000),
  name,
})

describe('Notification observation workflow', () => {
  beforeEach(() => {
    eventHandler = undefined
    statusHandler = undefined
    backend.api.notificationObservationStart.mockResolvedValue(undefined)
    backend.api.notificationObservationStop.mockResolvedValue(undefined)
    backend.events.notificationObservationEvent.mockImplementation((handler: (event: NotificationObservationEvent) => void) => {
      eventHandler = handler
      return Promise.resolve(vi.fn())
    })
    backend.events.notificationObservationStatus.mockImplementation((handler: (status: NotificationObservationStatus) => void) => {
      statusHandler = handler
      return Promise.resolve(vi.fn())
    })
  })

  it('requires an explicit selection and registers listeners before starting', async () => {
    const user = userEvent.setup()
    render(<Notifications connected desktop udid="device-1" onToast={vi.fn()} />)

    expect(screen.getByRole('button', { name: 'Start Listening' })).toBeDisabled()
    await user.click(screen.getByRole('checkbox', { name: /Application installed/ }))
    const start = screen.getByRole('button', { name: 'Start Listening' })
    await waitFor(() => expect(start).toBeEnabled())
    await user.click(start)

    expect(eventHandler).toBeDefined()
    expect(statusHandler).toBeDefined()
    expect(backend.api.notificationObservationStart).toHaveBeenCalledWith(
      [selectedName],
      expect.any(String),
      'device-1',
    )
  })

  it('adds an explicit custom notification to the next session', async () => {
    const user = userEvent.setup()
    render(<Notifications connected desktop udid="device-1" onToast={vi.fn()} />)
    await user.type(screen.getByLabelText('Custom notification name'), 'com.example.custom')
    await user.click(screen.getByRole('button', { name: 'Add custom notification' }))
    await waitFor(() => expect(screen.getByRole('button', { name: 'Start Listening' })).toBeEnabled())
    await user.click(screen.getByRole('button', { name: 'Start Listening' }))

    expect(backend.api.notificationObservationStart).toHaveBeenCalledWith(
      ['com.example.custom'],
      expect.any(String),
      'device-1',
    )
  })

  it('renders newest-first events and retains at most 500 names', async () => {
    const user = userEvent.setup()
    render(<Notifications connected desktop udid="device-1" onToast={vi.fn()} />)
    await user.click(screen.getByRole('checkbox', { name: /Application installed/ }))
    await waitFor(() => expect(screen.getByRole('button', { name: 'Start Listening' })).toBeEnabled())
    await user.click(screen.getByRole('button', { name: 'Start Listening' }))

    act(() => {
      for (let sequence = 1; sequence <= 502; sequence += 1) eventHandler?.(observed(sequence))
    })

    expect(await screen.findByText('500')).toBeInTheDocument()
    expect(screen.getByText('#502')).toBeInTheDocument()
    expect(screen.queryByText('#1')).not.toBeInTheDocument()
  })

  it('ignores events and status from a superseded session', async () => {
    const user = userEvent.setup()
    render(<Notifications connected desktop udid="device-1" onToast={vi.fn()} />)
    await user.click(screen.getByRole('checkbox', { name: /Application installed/ }))
    await waitFor(() => expect(screen.getByRole('button', { name: 'Start Listening' })).toBeEnabled())
    await user.click(screen.getByRole('button', { name: 'Start Listening' }))

    act(() => {
      eventHandler?.(observed(1, 'stale-session', 'com.apple.stale'))
      statusHandler?.({
        sessionId: 'stale-session',
        state: 'error',
        message: 'stale failure',
        transport: null,
        subscriptions: [selectedName],
      })
    })

    expect(screen.queryByText('com.apple.stale')).not.toBeInTheDocument()
    expect(screen.queryByText('stale failure')).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Pause' })).toBeInTheDocument()
  })

  it('pauses without clearing history and clears only on request', async () => {
    const user = userEvent.setup()
    render(<Notifications connected desktop udid="device-1" onToast={vi.fn()} />)
    await user.click(screen.getByRole('checkbox', { name: /Application installed/ }))
    await waitFor(() => expect(screen.getByRole('button', { name: 'Start Listening' })).toBeEnabled())
    await user.click(screen.getByRole('button', { name: 'Start Listening' }))
    act(() => eventHandler?.(observed(1)))

    await user.click(screen.getByRole('button', { name: 'Pause' }))
    expect(backend.api.notificationObservationStop).toHaveBeenCalledOnce()
    expect(screen.getByText('#1')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Clear History' }))
    expect(screen.queryByText('#1')).not.toBeInTheDocument()
    expect(screen.getByText('No notification events yet')).toBeInTheDocument()
  })

  it('stops an active listener when leaving the page', async () => {
    const user = userEvent.setup()
    const view = render(<Notifications connected desktop udid="device-1" onToast={vi.fn()} />)
    await user.click(screen.getByRole('checkbox', { name: /Application installed/ }))
    await waitFor(() => expect(screen.getByRole('button', { name: 'Start Listening' })).toBeEnabled())
    await user.click(screen.getByRole('button', { name: 'Start Listening' }))
    await waitFor(() => expect(backend.api.notificationObservationStart).toHaveBeenCalledOnce())

    view.unmount()

    await waitFor(() => expect(backend.api.notificationObservationStop).toHaveBeenCalledOnce())
  })
})
