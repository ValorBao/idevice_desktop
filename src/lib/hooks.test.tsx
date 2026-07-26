import { act, renderHook } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { useDeviceTask } from './hooks'

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
