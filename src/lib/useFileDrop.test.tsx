import { act, render, screen, waitFor } from '@testing-library/react'
import { useRef } from 'react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useFileDrop } from './useFileDrop'

const webview = vi.hoisted(() => ({
  onDragDropEvent: vi.fn(),
}))

vi.mock('@tauri-apps/api/webview', () => ({
  getCurrentWebview: () => webview,
}))

type DragPayload =
  | { type: 'leave' }
  | { type: 'enter' | 'over'; position: { x: number; y: number } }
  | { type: 'drop'; position: { x: number; y: number }; paths: string[] }

function Harness({
  enabled = true,
  onDrop,
}: {
  enabled?: boolean
  onDrop: (paths: string[], element: Element | null) => void
}) {
  const ref = useRef<HTMLDivElement>(null)
  const dragging = useFileDrop(ref, enabled, onDrop)
  return <div ref={ref} data-testid="target" data-dragging={dragging} />
}

describe('useFileDrop', () => {
  beforeEach(() => {
    Object.defineProperty(window, 'devicePixelRatio', { configurable: true, value: 2 })
    vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue({
      left: 10,
      top: 20,
      right: 110,
      bottom: 120,
      width: 100,
      height: 100,
      x: 10,
      y: 20,
      toJSON: () => ({}),
    })
  })

  it('scopes physical-pixel drag events to the target and forwards the hit element', async () => {
    const stop = vi.fn()
    webview.onDragDropEvent.mockResolvedValue(stop)
    const onDrop = vi.fn()
    const hit = document.createElement('button')
    Object.defineProperty(document, 'elementFromPoint', {
      configurable: true,
      value: vi.fn(() => hit),
    })

    render(<Harness onDrop={onDrop} />)
    await waitFor(() => expect(webview.onDragDropEvent).toHaveBeenCalledOnce())
    const listener = webview.onDragDropEvent.mock.calls[0][0] as (event: { payload: DragPayload }) => void

    act(() => listener({ payload: { type: 'enter', position: { x: 100, y: 100 } } }))
    expect(screen.getByTestId('target')).toHaveAttribute('data-dragging', 'true')

    act(() => listener({
      payload: { type: 'drop', position: { x: 100, y: 100 }, paths: ['/tmp/App.ipa'] },
    }))

    expect(screen.getByTestId('target')).toHaveAttribute('data-dragging', 'false')
    expect(onDrop).toHaveBeenCalledWith(['/tmp/App.ipa'], hit)

    act(() => listener({
      payload: { type: 'drop', position: { x: 400, y: 400 }, paths: ['/tmp/outside.ipa'] },
    }))
    expect(onDrop).toHaveBeenCalledOnce()
  })

  it('does not subscribe when desktop drop handling is disabled', () => {
    render(<Harness enabled={false} onDrop={vi.fn()} />)
    expect(webview.onDragDropEvent).not.toHaveBeenCalled()
  })

  it('unsubscribes when the target unmounts', async () => {
    const stop = vi.fn()
    webview.onDragDropEvent.mockResolvedValue(stop)
    const { unmount } = render(<Harness onDrop={vi.fn()} />)

    await waitFor(() => expect(webview.onDragDropEvent).toHaveBeenCalledOnce())
    unmount()

    expect(stop).toHaveBeenCalledOnce()
  })
})
