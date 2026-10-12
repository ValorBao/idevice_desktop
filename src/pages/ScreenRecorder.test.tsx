import { act, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { ScreenRecording, ScreenRecordingStatus } from '../api'

const backend = vi.hoisted(() => ({
  api: {
    screenRecordingStart: vi.fn(),
    screenRecordingStop: vi.fn(),
    screenRecordingCancel: vi.fn(),
    screenRecordingCurrent: vi.fn(),
    screenRecordingDiscard: vi.fn(),
    screenRecordingExport: vi.fn(),
    screenRecordingExportCancel: vi.fn(),
  },
  dialogs: {
    mp4Destination: vi.fn(),
    confirmDestructive: vi.fn(),
  },
  events: {
    screenRecordingStatus: vi.fn(),
    screenRecordingExportProgress: vi.fn(),
  },
}))

vi.mock('../api', () => ({
  ...backend,
  errorMessage: (error: unknown) => String(error),
  localMediaUrl: (path: string) => `asset://localhost${path}`,
}))

import { ScreenRecorder } from './ScreenRecorder'

let statusHandler: ((status: ScreenRecordingStatus) => void) | undefined

const recording: ScreenRecording = {
  id: 'session',
  path: '/cache/screen-recordings/session.mov',
  deviceName: 'Test iPhone',
  durationMs: 10_000,
  width: 828,
  height: 1_792,
  bytes: 7 * 1024 * 1024,
}

const startedSession = () => backend.api.screenRecordingStart.mock.calls[0][0] as string

const status = (state: string, sessionId = startedSession()): ScreenRecordingStatus => ({
  sessionId,
  state,
  message: state === 'error' ? 'the iPhone sent no picture' : null,
  deviceName: state === 'recording' ? 'Test iPhone' : null,
  recording: state === 'finished' ? recording : null,
})

const renderRecorder = (desktop = true) => {
  const onToast = vi.fn()
  const view = render(<ScreenRecorder desktop={desktop} deviceName="Test iPhone" onToast={onToast} />)
  return { ...view, onToast }
}

const recordAndStop = async (user: ReturnType<typeof userEvent.setup>) => {
  await user.click(await screen.findByRole('button', { name: 'Start Recording' }))
  act(() => statusHandler?.(status('recording')))
  await user.click(screen.getByRole('button', { name: 'Stop Recording' }))
  await screen.findByLabelText('Recorded device screen')
}

describe('Screen Recorder workflow', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    statusHandler = undefined
    backend.api.screenRecordingStart.mockResolvedValue(undefined)
    backend.api.screenRecordingStop.mockResolvedValue(recording)
    backend.api.screenRecordingCancel.mockResolvedValue(undefined)
    backend.api.screenRecordingCurrent.mockResolvedValue(null)
    backend.api.screenRecordingDiscard.mockResolvedValue(undefined)
    backend.api.screenRecordingExport.mockResolvedValue(undefined)
    backend.dialogs.mp4Destination.mockResolvedValue('/tmp/clip.mp4')
    backend.dialogs.confirmDestructive.mockResolvedValue(true)
    backend.events.screenRecordingStatus.mockImplementation((handler: (next: ScreenRecordingStatus) => void) => {
      statusHandler = handler
      return Promise.resolve(vi.fn())
    })
    backend.events.screenRecordingExportProgress.mockResolvedValue(vi.fn())
  })

  it('records the selected device and opens the finished recording for editing', async () => {
    const user = userEvent.setup()
    renderRecorder()

    await user.click(await screen.findByRole('button', { name: 'Start Recording' }))
    expect(backend.api.screenRecordingStart).toHaveBeenCalledWith(expect.any(String), 'Test iPhone')
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument()

    act(() => statusHandler?.(status('recording')))
    await user.click(screen.getByRole('button', { name: 'Stop Recording' }))

    const video = await screen.findByLabelText('Recorded device screen')
    expect(video).toHaveAttribute('src', 'asset://localhost/cache/screen-recordings/session.mov')
    expect(screen.getAllByText('0:10.0').length).toBeGreaterThan(0)
  })

  it('ignores status from a recording session it did not start', async () => {
    const user = userEvent.setup()
    renderRecorder()
    await user.click(await screen.findByRole('button', { name: 'Start Recording' }))

    act(() => statusHandler?.(status('cancelled', 'an-older-session')))

    expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument()
  })

  it('reports a failed recording and returns to the start state', async () => {
    const user = userEvent.setup()
    const { onToast } = renderRecorder()
    await user.click(await screen.findByRole('button', { name: 'Start Recording' }))

    act(() => statusHandler?.(status('error')))

    expect(onToast).toHaveBeenCalledWith('the iPhone sent no picture')
    expect(screen.getByRole('status')).toHaveTextContent('the iPhone sent no picture')
    expect(screen.getByRole('button', { name: 'Start Recording' })).toBeInTheDocument()
  })

  it('cancels a recording that is still connecting', async () => {
    const user = userEvent.setup()
    renderRecorder()
    await user.click(await screen.findByRole('button', { name: 'Start Recording' }))
    await user.click(screen.getByRole('button', { name: 'Cancel' }))

    expect(backend.api.screenRecordingCancel).toHaveBeenCalled()
    expect(screen.getByRole('button', { name: 'Start Recording' })).toBeInTheDocument()
  })

  it('exports only the footage left after a cut', async () => {
    const user = userEvent.setup()
    renderRecorder()
    await recordAndStop(user)

    const timeline = screen.getByRole('slider', { name: 'Recording position' })
    vi.spyOn(timeline, 'getBoundingClientRect').mockReturnValue({ left: 0, width: 100 } as DOMRect)
    await user.pointer({ target: timeline, coords: { clientX: 20 }, keys: '[MouseLeft]' })
    await user.click(screen.getByRole('button', { name: 'Set Start' }))
    await user.pointer({ target: timeline, coords: { clientX: 50 }, keys: '[MouseLeft]' })
    await user.click(screen.getByRole('button', { name: 'Set End' }))
    await user.click(screen.getByRole('button', { name: 'Cut Selection' }))
    expect(screen.getByText('0:07.0')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'Export MP4' }))

    await waitFor(() => expect(backend.api.screenRecordingExport).toHaveBeenCalledWith('/tmp/clip.mp4', [
      { startMs: 0, endMs: 2_000 },
      { startMs: 5_000, endMs: 10_000 },
    ]))
  })

  it('asks before a new recording replaces the current one', async () => {
    const user = userEvent.setup()
    renderRecorder()
    await recordAndStop(user)
    backend.dialogs.confirmDestructive.mockResolvedValue(false)

    await user.click(screen.getByRole('button', { name: 'Record Again' }))

    expect(backend.dialogs.confirmDestructive).toHaveBeenCalled()
    expect(backend.api.screenRecordingStart).toHaveBeenCalledTimes(1)
    expect(screen.getByLabelText('Recorded device screen')).toBeInTheDocument()
  })

  it('reopens the working copy kept from an earlier visit', async () => {
    backend.api.screenRecordingCurrent.mockResolvedValue(recording)
    renderRecorder()

    expect(await screen.findByLabelText('Recorded device screen')).toBeInTheDocument()
  })

  it('finishes a running recording when the page closes', async () => {
    const user = userEvent.setup()
    const { unmount } = renderRecorder()
    await user.click(await screen.findByRole('button', { name: 'Start Recording' }))
    act(() => statusHandler?.(status('recording')))

    unmount()

    expect(backend.api.screenRecordingStop).toHaveBeenCalled()
  })

  it('simulates the whole workflow in demonstration mode without calling the backend', async () => {
    const user = userEvent.setup()
    const { onToast } = renderRecorder(false)

    await user.click(screen.getByRole('button', { name: 'Start Recording' }))
    await user.click(screen.getByRole('button', { name: 'Stop Recording' }))
    expect(screen.getByText('Demonstration recording')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Export MP4' }))

    expect(onToast).toHaveBeenCalledWith(expect.stringContaining('ready in demonstration mode'))
    expect(backend.api.screenRecordingStart).not.toHaveBeenCalled()
    expect(backend.api.screenRecordingExport).not.toHaveBeenCalled()
  })
})
