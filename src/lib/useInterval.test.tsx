import { act, render } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useInterval } from './useInterval'

function Harness({ active, delayMs, tick }: { active: boolean; delayMs: number; tick: () => void }) {
  useInterval(active, delayMs, tick)
  return null
}

describe('useInterval', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  it('ticks while active and stops on unmount', () => {
    const tick = vi.fn()
    const view = render(<Harness active delayMs={100} tick={tick} />)

    act(() => { vi.advanceTimersByTime(250) })
    expect(tick).toHaveBeenCalledTimes(2)

    view.unmount()
    act(() => { vi.advanceTimersByTime(500) })
    expect(tick).toHaveBeenCalledTimes(2)
  })

  it('does not tick while inactive', () => {
    const tick = vi.fn()
    render(<Harness active={false} delayMs={100} tick={tick} />)
    act(() => { vi.advanceTimersByTime(500) })
    expect(tick).not.toHaveBeenCalled()
  })

  /// Pages pass an inline closure, so a changed callback must not restart the
  /// timer: that would reset the interval on every render and never fire.
  it('runs the latest callback without restarting the timer', () => {
    const first = vi.fn()
    const second = vi.fn()
    const view = render(<Harness active delayMs={100} tick={first} />)

    act(() => { vi.advanceTimersByTime(90) })
    view.rerender(<Harness active delayMs={100} tick={second} />)
    act(() => { vi.advanceTimersByTime(20) })

    expect(first).not.toHaveBeenCalled()
    expect(second).toHaveBeenCalledOnce()
  })
})
