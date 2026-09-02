import { useCallback, useEffect, useRef } from 'react'
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

/** True while the calling component is mounted. Start/stop handlers use this to skip setState after unmount. */
export function useMountedRef() {
  const mountedRef = useRef(true)
  useEffect(() => {
    mountedRef.current = true
    return () => { mountedRef.current = false }
  }, [])
  return mountedRef
}

type Unlisten = () => void

/**
 * Subscribe to desktop event listeners for a long-running device session.
 *
 * Unlistens when the view unmounts or `deps` change, then runs `onStop` so the
 * matching backend task does not outlive the page. `alive()` is false after
 * cleanup; event handlers should ignore payloads once that happens.
 */
export function useDesktopListeners(
  enabled: boolean,
  subscribe: (alive: () => boolean) => Promise<Unlisten[]>,
  onStop?: () => void,
  deps: readonly unknown[] = [],
) {
  const subscribeRef = useRef(subscribe)
  subscribeRef.current = subscribe
  const onStopRef = useRef(onStop)
  onStopRef.current = onStop

  useEffect(() => {
    if (!enabled) return
    let disposed = false
    const alive = () => !disposed
    let stops: Unlisten[] = []
    void subscribeRef.current(alive).then((listeners) => {
      if (disposed) {
        listeners.forEach((stop) => stop())
        return
      }
      stops = listeners
    })
    return () => {
      disposed = true
      stops.forEach((stop) => stop())
      onStopRef.current?.()
    }
    // Callers pass the values that should restart the session (udid, callbacks).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabled, ...deps])
}
