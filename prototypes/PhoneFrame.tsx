import { useRef, useState, type CSSProperties, type PointerEvent } from 'react'

export function PhoneFrame({
  className,
  caption,
}: {
  className?: string
  caption?: string
}) {
  const [rotation, setRotation] = useState({ x: -6, y: 14 })
  const [dragging, setDragging] = useState(false)
  const start = useRef<{ x: number; y: number; rx: number; ry: number } | null>(null)
  const onDown = (event: PointerEvent<HTMLDivElement>) => {
    event.currentTarget.setPointerCapture(event.pointerId)
    start.current = { x: event.clientX, y: event.clientY, rx: rotation.x, ry: rotation.y }
    setDragging(true)
  }
  const onMove = (event: PointerEvent<HTMLDivElement>) => {
    if (!start.current) return
    setRotation({
      x: Math.max(-16, Math.min(16, start.current.rx - (event.clientY - start.current.y) * 0.14)),
      y: Math.max(-32, Math.min(32, start.current.ry + (event.clientX - start.current.x) * 0.18)),
    })
  }
  const onUp = (event: PointerEvent<HTMLDivElement>) => {
    start.current = null
    setDragging(false)
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId)
    }
  }
  return (
    <div
      className={`proto-phone ${dragging ? 'is-dragging' : ''} ${className ?? ''}`}
      style={{ '--rx': `${rotation.x}deg`, '--ry': `${rotation.y}deg` } as CSSProperties}
      onPointerDown={onDown}
      onPointerMove={onMove}
      onPointerUp={onUp}
      onPointerCancel={onUp}
      onDoubleClick={() => setRotation({ x: -6, y: 14 })}
      title="Drag to rotate · double-click to reset"
    >
      <div className="proto-phone-body">
        <span className="proto-phone-notch" />
        <div className="proto-phone-screen" aria-hidden="true">
          <b>9:41</b>
          <div className="proto-phone-icons">
            {['#5b8cff', '#34d399', '#f59e0b', '#fb7185', '#a78bfa', '#38bdf8', '#f472b6', '#2dd4bf'].map((color) => (
              <i key={color} style={{ background: color }} />
            ))}
          </div>
          <span>StikDebug</span>
        </div>
      </div>
      {caption && <small className="proto-phone-caption">{caption}</small>}
    </div>
  )
}
