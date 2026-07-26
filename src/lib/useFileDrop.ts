import { useEffect, useState, type RefObject } from 'react'
import { getCurrentWebview } from '@tauri-apps/api/webview'

/**
 * Files dragged in from the operating system, scoped to one element.
 *
 * The webview's own drop events are not an option on the desktop: Tauri
 * intercepts OS drags before they reach the page, so `ondrop` never fires and
 * `dataTransfer.files` stays empty. Its own event is also the only source of a
 * real path — a browser `File` has no filesystem location to give, and the
 * `File.path` property some code reaches for is an Electron extension that
 * WKWebView does not implement.
 *
 * Tauri reports drags for the whole window rather than per element, so the
 * pointer position decides who owns a drop. `onDrop` receives the element under
 * the pointer alongside the paths, which lets a caller treat a drop onto a row
 * differently from a drop onto the surrounding area.
 */
export function useFileDrop(
  target: RefObject<HTMLElement | null>,
  enabled: boolean,
  onDrop: (paths: string[], elementAtPointer: Element | null) => void,
) {
  const [dragging, setDragging] = useState(false)

  useEffect(() => {
    if (!enabled) return
    let unlisten: (() => void) | undefined
    let disposed = false

    // Tauri reports physical pixels; hit testing needs CSS pixels.
    const toClientPoint = (position: { x: number; y: number }) => {
      const ratio = window.devicePixelRatio || 1
      return { x: position.x / ratio, y: position.y / ratio }
    }
    const isInside = (point: { x: number; y: number }) => {
      const bounds = target.current?.getBoundingClientRect()
      if (!bounds) return false
      return point.x >= bounds.left && point.x <= bounds.right
        && point.y >= bounds.top && point.y <= bounds.bottom
    }

    void getCurrentWebview().onDragDropEvent((event) => {
      const payload = event.payload
      if (payload.type === 'leave') {
        setDragging(false)
        return
      }
      const point = toClientPoint(payload.position)
      const inside = isInside(point)
      if (payload.type === 'enter' || payload.type === 'over') {
        setDragging(inside)
        return
      }
      setDragging(false)
      if (inside && payload.paths.length) {
        onDrop(payload.paths, document.elementFromPoint(point.x, point.y))
      }
    }).then((stop) => {
      if (disposed) stop()
      else unlisten = stop
    })

    return () => {
      disposed = true
      unlisten?.()
    }
  }, [target, enabled, onDrop])

  return dragging
}
