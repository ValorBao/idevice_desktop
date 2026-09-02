import { act, renderHook, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { useDesktopListeners, useDeviceTask } from './hooks'

describe('useDeviceTask', () => {
  it('blocks device work in browser mode and reports the supplied explanation', async () => {
    const task = vi.fn(async () => {})
    const onError = vi.fn()
    const { result } = renderHook(() => useDeviceTask(false, onError))

    await act(async () => {
      await result.current(task, 'Connect a device in the desktop app')
    })

    expect(task).not.toHaveBeenCalled()
    expect(onError).toHaveBeenCalledWith('Connect a device in the desktop app')
  })

  it('runs a browser substitute without calling the device task', async () => {
    const task = vi.fn(async () => {})
    const browser = vi.fn()
    const { result } = renderHook(() => useDeviceTask(false, vi.fn()))

    await act(async () => {
      await result.current(task, browser)
    })

    expect(task).not.toHaveBeenCalled()
    expect(browser).toHaveBeenCalledOnce()
  })

  it('runs desktop work and reports a structured command error', async () => {
    const task = vi.fn(async () => {
      throw { kind: 'device', message: 'Device disconnected', retryable: true }
    })
    const onError = vi.fn()
    const { result } = renderHook(() => useDeviceTask(true, onError))

    await act(async () => {
      await result.current(task)
    })

    expect(task).toHaveBeenCalledOnce()
    expect(onError).toHaveBeenCalledWith('Device disconnected')
  })
})

describe('useDesktopListeners', () => {
  it('does not subscribe in browser demo mode', () => {
    const subscribe = vi.fn(async () => [])
    renderHook(() => useDesktopListeners(false, subscribe, vi.fn()))
    expect(subscribe).not.toHaveBeenCalled()
  })

  it('unlistens and stops the backend session when the view unmounts', async () => {
    const stop = vi.fn()
    const onStop = vi.fn()
    const subscribe = vi.fn(async () => [stop])
    const { unmount } = renderHook(() => useDesktopListeners(true, subscribe, onStop))

    await waitFor(() => expect(subscribe).toHaveBeenCalledOnce())
    unmount()

    expect(stop).toHaveBeenCalledOnce()
    expect(onStop).toHaveBeenCalledOnce()
  })

  it('drops a late subscription if the view already unmounted', async () => {
    let resolve!: (value: Array<() => void>) => void
    const pending = new Promise<Array<() => void>>((done) => { resolve = done })
    const stop = vi.fn()
    const { unmount } = renderHook(() => useDesktopListeners(true, () => pending))

    unmount()
    await act(async () => { resolve([stop]) })

    expect(stop).toHaveBeenCalledOnce()
  })
})
