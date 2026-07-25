import { useCallback } from 'react'
import { errorMessage } from '../api'

/**
 * What a task should do when the app is running in the browser demo rather than
 * on the desktop: show a message explaining the operation needs a device, or run
 * a demo-only substitute.
 */
type BrowserFallback = string | (() => void)

/**
 * Wraps a device operation with the guard and error handling every page repeats:
 * skip the call entirely in browser demo mode, and report a thrown error through
 * the page's notification callback.
 *
 * Guards that depend on page state stay inside the task, where they keep their
 * original order relative to the desktop check.
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
        onError(errorMessage(error))
      }
    },
    [desktop, onError],
  )
}
