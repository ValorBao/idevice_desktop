import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const backend = vi.hoisted(() => ({
  api: {
    deviceList: vi.fn(),
    deviceSelect: vi.fn(),
    deviceMonitorStart: vi.fn(),
    deviceMonitorStop: vi.fn(),
  },
  events: {
    deviceChanged: vi.fn(),
  },
}))

vi.mock('./api', () => ({
  ...backend,
  errorMessage: (error: unknown) => String(error),
  isDesktopRuntime: () => true,
}))

vi.mock('./pages/Overview', () => ({ Overview: () => <div>Overview</div> }))
vi.mock('./pages/Diagnostics', () => ({ Diagnostics: () => <div>Diagnostics</div> }))
vi.mock('./pages/Files', () => ({ Files: () => <div>Files</div> }))
vi.mock('./pages/Apps', () => ({ Apps: () => <div>Apps</div> }))
vi.mock('./pages/CrashReports', () => ({ CrashReports: () => <div>Crash Reports</div> }))
vi.mock('./pages/Logs', () => ({ Logs: () => <div>Logs</div> }))
vi.mock('./pages/Developer', () => ({ Developer: () => <div>Developer</div> }))
vi.mock('./pages/Location', () => ({
  Location: ({ udid }: { udid: string }) => <div data-testid="location-page">{udid}</div>,
}))

import App from './App'

const devices = [
  {
    id: 'device-a',
    udid: 'udid-a',
    name: 'Alpha iPhone',
    model: 'iPhone A',
    ios: '17.0',
    connection: 'USB',
    transports: ['USB'],
    paired: true,
    connectable: true,
  },
  {
    id: 'device-b',
    udid: 'udid-b',
    name: 'Beta iPhone',
    model: 'iPhone B',
    ios: '14.2',
    connection: 'USB',
    transports: ['USB'],
    paired: true,
    connectable: true,
  },
]

describe('device page lifecycle', () => {
  beforeEach(() => {
    backend.api.deviceList.mockResolvedValue(devices)
    backend.api.deviceSelect.mockResolvedValue(undefined)
    backend.api.deviceMonitorStart.mockResolvedValue(undefined)
    backend.api.deviceMonitorStop.mockResolvedValue(undefined)
    backend.events.deviceChanged.mockResolvedValue(vi.fn())
  })

  it('remounts the active page when the selected device changes', async () => {
    const user = userEvent.setup()
    render(<App />)

    await screen.findByRole('button', { name: 'Select device' })
    await user.click(screen.getByRole('button', { name: 'Location' }))
    const firstPage = screen.getByTestId('location-page')
    expect(firstPage).toHaveTextContent('udid-a')

    await user.click(screen.getByRole('button', { name: 'Select device' }))
    await user.click(screen.getByRole('button', { name: /Beta iPhone/ }))

    await waitFor(() => expect(screen.getByTestId('location-page')).toHaveTextContent('udid-b'))
    expect(screen.getByTestId('location-page')).not.toBe(firstPage)
  })
})
