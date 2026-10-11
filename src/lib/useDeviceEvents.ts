import { useEffect, useRef, useState, type DependencyList } from 'react'
import { errorMessage, type UnlistenFn } from '../api'

/**
 * A Tauri event subscription whose handler is only invoked while the owning
 * effect is alive. Build one with {@link on}.
 */
export type Listener = (alive: () => boolean) => Promise<UnlistenFn>

/**
 * Pairs an `events.*` subscriber with its handler. Payloads that arrive after
 * the effect has been cleaned up are dropped, so handlers never have to check
 * a `disposed` flag themselves.
 */
export const on = <T,>(
  subscribe: (handler: (payload: T) => void) => Promise<UnlistenFn>,
  handler: (payload: T) => void,
): Listener =>
  (alive) => subscribe((payload) => { if (alive()) handler(payload) })

export type DeviceEventSession = {
  listeners: Listener[]
  /** Runs once every listener is attached, typically to start the backend stream. */
  start?: () => void | Promise<unknown>
  /**
   * Runs when the effect is cleaned up. If `start` is still in flight at that
   * point it runs again once `start` settles: the first call lets the backend
   * refuse a late registration, the second reaches a session that did register.
   */
  stop?: () => void
}

/**
 * Subscribes to a set of device events for as long as `enabled` holds and the
 * `deps` are stable, and reports subscription or start failures to `onError`.
 *
 * Every streaming page repeated the same choreography: subscribe to a status
 * and a payload event, unsubscribe again if the component unmounted while the
 * subscriptions were still resolving, then stop the backend session on
 * cleanup. This hook owns that choreography; the page keeps only its handlers.
 *
 * Handlers are captured when the session is built, so a page that closes over
 * changing values lists them in `deps`. `onError` is exempt: it is always read
 * at its latest value, because a changed reporter must not tear down a live
 * device stream.
 *
 * Returns whether the listeners are attached. In browser demo mode there is
 * nothing to attach, so it is immediately true.
 */
export function useDeviceEvents(
  enabled: boolean,
  setup: () => DeviceEventSession,
  onError: (message: string) => void,
  deps: DependencyList,
): boolean {
  const [ready, setReady] = useState(false)
  // The latest reporter is used without restarting the session when a page
  // passes an inline callback.
  const onErrorRef = useRef(onError)
  useEffect(() => { onErrorRef.current = onError }, [onError])

  useEffect(() => {
    if (!enabled) return
    let alive = true
    const isAlive = () => alive
    const session = setup()
    const stops: UnlistenFn[] = []
    setReady(false)

    const finish = () => {
      stops.splice(0).forEach((stop) => stop())
      session.stop?.()
    }

    void (async () => {
      try {
        const attached = await Promise.all(session.listeners.map((listener) => listener(isAlive)))
        if (!alive) {
          attached.forEach((stop) => stop())
          return
        }
        stops.push(...attached)
        setReady(true)
        if (session.start) {
          try {
            await session.start()
          } finally {
            if (!alive) session.stop?.()
          }
        }
      } catch (error) {
        if (alive) onErrorRef.current(errorMessage(error))
      }
    })()

    return () => {
      alive = false
      finish()
    }
    // The caller's deps describe when the session must be rebuilt.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabled, ...deps])

  return !enabled || ready
}
