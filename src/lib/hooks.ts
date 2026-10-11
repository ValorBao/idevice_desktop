import { useCallback } from 'react'
import { reportCommand } from './commandError'

/**
 * What a task should do when the app is running in the browser demo rather than
 * on the desktop: show a message explaining the operation needs a device, or run
 * a demo-only substitute.
 */
type BrowserFallback = string | (() => void)

/**
 * Wraps a device operation with the demo-mode guard and reports thrown errors
 * through `reportCommand` so cancelled commands do not toast as failures.
 *
 * Guards that depend on page state stay inside the task, where they keep their
 * original order relative to the desktop check. A catch that also resets page
 * state should call `reportCommand` itself rather than moving into this hook.
 */
export function useDeviceTask(desktop: boolean, onError: (message: string) => void) {
  return useCallback(
    async (task: () => Promise<void>, browser?: BrowserFallback) => {
      if (!desktop) {
        if (typeof browser === 'string') onError(browser)
        else browser?.()
        return
      }
      try {
        await task()
      } catch (error) {
        reportCommand(error, onError)
      }
    },
    [desktop, onError],
  )
}
