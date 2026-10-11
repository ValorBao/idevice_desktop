import { act, render, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { on, useDeviceEvents, type DeviceEventSession } from './useDeviceEvents'

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((done) => { resolve = done })
  return { promise, resolve }
}

function Harness({
  enabled = true,
  setup,
  onError = vi.fn(),
}: {
  enabled?: boolean
  setup: () => DeviceEventSession
  onError?: (message: string) => void
}) {
  const ready = useDeviceEvents(enabled, setup, onError, [])
  return <span>{ready ? 'ready' : 'waiting'}</span>
}

describe('useDeviceEvents', () => {
  it('attaches listeners, then starts, and stops on unmount', async () => {
    const stopListener = vi.fn()
    const start = vi.fn().mockResolvedValue(undefined)
    const stop = vi.fn()
    const subscribe = vi.fn().mockResolvedValue(stopListener)

    const view = render(<Harness setup={() => ({
      listeners: [on(subscribe, vi.fn())],
      start,
      stop,
    })} />)

    await waitFor(() => expect(start).toHaveBeenCalledOnce())
    expect(subscribe).toHaveBeenCalledBefore(start)
    expect(view.getByText('ready')).toBeInTheDocument()

    view.unmount()
    expect(stopListener).toHaveBeenCalledOnce()
    expect(stop).toHaveBeenCalledOnce()
  })

  /// A subscription that resolves after unmount still holds a live listener,
  /// so it has to be released; starting the backend then would leak a session.
  it('releases a late subscription without starting', async () => {
    const pending = deferred<() => void>()
    const stopListener = vi.fn()
    const start = vi.fn()

    const view = render(<Harness setup={() => ({
      listeners: [() => pending.promise],
      start,
    })} />)
    view.unmount()

    await act(async () => pending.resolve(stopListener))
    expect(stopListener).toHaveBeenCalledOnce()
    expect(start).not.toHaveBeenCalled()
  })

  /// The first stop may reach the backend before the start it cancels, so the
  /// session is stopped again once start settles.
  it('repeats the stop when unmounted mid-start', async () => {
    const pending = deferred<void>()
    const stop = vi.fn()

    const view = render(<Harness setup={() => ({
      listeners: [on(vi.fn().mockResolvedValue(vi.fn()), vi.fn())],
      start: () => pending.promise,
      stop,
    })} />)

    await waitFor(() => expect(view.getByText('ready')).toBeInTheDocument())
    view.unmount()
    expect(stop).toHaveBeenCalledOnce()

    await act(async () => pending.resolve())
    expect(stop).toHaveBeenCalledTimes(2)
  })

  it('drops payloads that arrive after cleanup', async () => {
    const handler = vi.fn()
    let deliver: ((payload: string) => void) | undefined
    const subscribe = vi.fn().mockImplementation((inner: (payload: string) => void) => {
      deliver = inner
      return Promise.resolve(vi.fn())
    })

    const view = render(<Harness setup={() => ({ listeners: [on(subscribe, handler)] })} />)
    await waitFor(() => expect(deliver).toBeDefined())

    act(() => deliver?.('before'))
    expect(handler).toHaveBeenCalledWith('before')

    view.unmount()
    act(() => deliver?.('after'))
    expect(handler).toHaveBeenCalledOnce()
  })

  it('reports a failed subscription and never starts', async () => {
    const onError = vi.fn()
    const start = vi.fn()

    render(<Harness
      onError={onError}
      setup={() => ({ listeners: [() => Promise.reject(new Error('Cannot listen'))], start })}
    />)

    await waitFor(() => expect(onError).toHaveBeenCalledWith('Cannot listen'))
    expect(start).not.toHaveBeenCalled()
  })

  it('attaches nothing and reports ready when disabled', () => {
    const setup = vi.fn()
    const view = render(<Harness enabled={false} setup={setup} />)
    expect(setup).not.toHaveBeenCalled()
    expect(view.getByText('ready')).toBeInTheDocument()
  })
})
