import { useEffect, useRef } from 'react'

/**
 * Runs `tick` every `delayMs` while `active` holds.
 *
 * The latest `tick` is always the one invoked, so callers can pass an inline
 * closure without restarting the timer on every render.
 */
export function useInterval(active: boolean, delayMs: number, tick: () => void) {
  const tickRef = useRef(tick)
  useEffect(() => { tickRef.current = tick }, [tick])
  useEffect(() => {
    if (!active) return
    const timer = window.setInterval(() => tickRef.current(), delayMs)
    return () => window.clearInterval(timer)
  }, [active, delayMs])
}
