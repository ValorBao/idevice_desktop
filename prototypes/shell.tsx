import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import ReactDOM from 'react-dom/client'
import { GlyphShell } from './GlyphShell'
import { IonShell } from './IonShell'
import { VoidShell } from './VoidShell'
import { useLab } from './useLab'
import './harness.css'
import './picker.css'
import './phone.css'

const names = ['Void', 'Ion', 'Glyph'] as const

function Picker({
  index,
  onChange,
  onReplay,
}: {
  index: number
  onChange: (index: number) => void
  onReplay: () => void
}) {
  const navRef = useRef<HTMLElement>(null)
  const highlightRef = useRef<HTMLSpanElement>(null)
  const [ready, setReady] = useState(false)

  useLayoutEffect(() => {
    const nav = navRef.current
    const highlight = highlightRef.current
    if (!nav || !highlight) return
    const items = [...nav.querySelectorAll<HTMLElement>('.proto-picker-item:not(.proto-picker-replay)')]
    const el = items[index]
    if (!el) return
    highlight.style.width = `${el.offsetWidth}px`
    highlight.style.transform = `translateX(${el.offsetLeft}px)`
  }, [index])

  useEffect(() => {
    const id = requestAnimationFrame(() => requestAnimationFrame(() => setReady(true)))
    return () => cancelAnimationFrame(id)
  }, [])

  return (
    <nav ref={navRef} className="proto-picker" aria-label="Prototype variants" {...(ready ? { 'data-ready': '' } : {})}>
      <span ref={highlightRef} className="proto-picker-highlight" aria-hidden="true" />
      {names.map((name, itemIndex) => (
        <button
          key={name}
          type="button"
          className="proto-picker-item"
          data-active={itemIndex === index || undefined}
          aria-current={itemIndex === index ? 'true' : undefined}
          onClick={() => onChange(itemIndex)}
        >
          {name}
        </button>
      ))}
      <span className="proto-picker-divider" aria-hidden="true" />
      <button type="button" className="proto-picker-item proto-picker-replay" aria-label="Replay animation (R)" onClick={onReplay}>
        ↻
      </button>
    </nav>
  )
}

function Stage({ index }: { index: number }) {
  const lab = useLab()
  if (index === 0) return <VoidShell lab={lab} />
  if (index === 1) return <IonShell lab={lab} />
  return <GlyphShell lab={lab} />
}

function Harness() {
  const initial = Math.min(Math.max((Number(new URLSearchParams(location.search).get('v')) || 1) - 1, 0), names.length - 1)
  const [index, setIndex] = useState(initial)
  const [nonce, setNonce] = useState(0)

  const setActive = (next: number) => {
    if (next < 0 || next >= names.length) return
    setIndex(next)
    const url = new URL(location.href)
    url.searchParams.set('v', String(next + 1))
    history.replaceState(null, '', url)
  }

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement
      if (/^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName) || target.isContentEditable) return
      if (event.metaKey || event.ctrlKey || event.altKey) return
      const num = Number.parseInt(event.key, 10)
      if (num >= 1 && num <= names.length) setActive(num - 1)
      else if (event.key === 'ArrowRight') setActive((index + 1) % names.length)
      else if (event.key === 'ArrowLeft') setActive((index - 1 + names.length) % names.length)
      else if (event.key === 'r' || event.key === 'R') setNonce((value) => value + 1)
    }
    document.addEventListener('keydown', onKey)
    return () => document.removeEventListener('keydown', onKey)
  }, [index])

  return (
    <>
      <div className="proto-stage" key={`${index}-${nonce}`}>
        <Stage index={index} />
      </div>
      <Picker index={index} onChange={setActive} onReplay={() => setNonce((value) => value + 1)} />
    </>
  )
}

ReactDOM.createRoot(document.getElementById('root')!).render(<Harness />)
