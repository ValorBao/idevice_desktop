import { useEffect, useMemo, useRef, useState } from 'react'
import { ArrowRight, ClipboardPaste, Download, Eraser, Image as ImageIcon, RefreshCw, Send, ShieldAlert, Upload, X } from 'lucide-react'
import {
  api,
  dialogs,
  errorMessage,
  type PasteboardImagePreparation,
  type PasteboardImageSnapshot,
  type PasteboardImageWriteResult,
  type PasteboardTextSnapshot,
  type PasteboardWriteResult,
} from '../api'

const MAX_TEXT_BYTES = 1024 * 1024
const DEMO_IMAGE_DATA_URL = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII='

const demoSnapshot = (): PasteboardTextSnapshot => {
  const text = 'TestFlight feedback\nReproduce on iOS 17.0 with the device unlocked.'
  return {
    text,
    byteLength: new TextEncoder().encode(text).length,
    characterCount: Array.from(text).length,
    changeCount: 18,
    itemCount: 1,
    state: 'text',
    message: null,
    transport: 'CoreDevice Pasteboard · demonstration',
  }
}

const demoImageSnapshot = (): PasteboardImageSnapshot => ({
  dataUrl: DEMO_IMAGE_DATA_URL,
  mimeType: 'image/png',
  width: 1,
  height: 1,
  byteLength: 68,
  changeCount: 18,
  itemCount: 1,
  state: 'image',
  message: null,
  transport: 'CoreDevice Pasteboard · demonstration',
})

const demoImagePreparation = (): PasteboardImagePreparation => ({
  preparationId: 'demo-image',
  dataUrl: DEMO_IMAGE_DATA_URL,
  mimeType: 'image/png',
  width: 1,
  height: 1,
  byteLength: 68,
  fileName: 'demo-pasteboard.png',
})

const bytes = (value: number) => {
  if (value < 1024) return `${value} B`
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(value < 10 * 1024 ? 1 : 0)} KB`
  return `${(value / 1024 / 1024).toFixed(2)} MB`
}

export function Pasteboard({
  desktop,
  udid,
  deviceName,
  onToast,
}: {
  desktop: boolean
  udid: string
  deviceName: string
  onToast: (message: string) => void
}) {
  const [mode, setMode] = useState<'text' | 'image'>('text')
  const [snapshot, setSnapshot] = useState<PasteboardTextSnapshot | null>(null)
  const [editor, setEditor] = useState('')
  const [lastWrite, setLastWrite] = useState<PasteboardWriteResult | null>(null)
  const [busy, setBusy] = useState<'idle' | 'reading' | 'writing'>('idle')
  const [failure, setFailure] = useState('')
  const [readStale, setReadStale] = useState(false)
  const requestRef = useRef(0)
  const [imageSnapshot, setImageSnapshot] = useState<PasteboardImageSnapshot | null>(null)
  const [imagePreparation, setImagePreparation] = useState<PasteboardImagePreparation | null>(null)
  const [lastImageWrite, setLastImageWrite] = useState<PasteboardImageWriteResult | null>(null)
  const [imageBusy, setImageBusy] = useState<'idle' | 'reading' | 'preparing' | 'writing'>('idle')
  const [imageFailure, setImageFailure] = useState('')
  const [imageReadStale, setImageReadStale] = useState(false)
  const imageRequestRef = useRef(0)
  const imagePreparationRef = useRef<PasteboardImagePreparation | null>(null)

  useEffect(() => () => { requestRef.current += 1 }, [])
  useEffect(() => {
    imageRequestRef.current += 1
    setImageSnapshot(null)
    setImagePreparation(null)
    setLastImageWrite(null)
    setImageFailure('')
    setImageReadStale(false)
    setImageBusy('idle')
    return () => {
      imageRequestRef.current += 1
      const prepared = imagePreparationRef.current
      imagePreparationRef.current = null
      if (desktop && prepared) void api.pasteboardImageDiscard(prepared.preparationId)
    }
  }, [desktop, udid])

  const editorBytes = useMemo(() => new TextEncoder().encode(editor).length, [editor])
  const editorCharacters = useMemo(() => Array.from(editor).length, [editor])
  const oversized = editorBytes > MAX_TEXT_BYTES

  const read = async () => {
    const request = ++requestRef.current
    setBusy('reading')
    setFailure('')
    try {
      const next = desktop ? await api.pasteboardTextRead(udid) : demoSnapshot()
      if (request !== requestRef.current) return
      setSnapshot(next)
      setReadStale(false)
    } catch (error) {
      if (request !== requestRef.current) return
      const message = errorMessage(error)
      setFailure(message)
      onToast(message)
    } finally {
      if (request === requestRef.current) setBusy('idle')
    }
  }

  const write = async () => {
    if (!editor || oversized || busy !== 'idle') return
    const lifecycle = requestRef.current
    const confirmed = await dialogs.confirmDestructive(
      `Replace the general pasteboard on ${deviceName} with ${editorCharacters.toLocaleString()} characters (${bytes(editorBytes)})? The existing pasteboard content will be replaced.`,
      'Replace Pasteboard',
    )
    if (!confirmed || lifecycle !== requestRef.current) return

    const request = ++requestRef.current
    setBusy('writing')
    setFailure('')
    try {
      const result = desktop
        ? await api.pasteboardTextWrite(editor, udid)
        : { byteLength: editorBytes, characterCount: editorCharacters, transport: 'CoreDevice Pasteboard · demonstration' }
      if (request !== requestRef.current) return
      setLastWrite(result)
      setReadStale(Boolean(snapshot))
      onToast(`Replaced ${deviceName} pasteboard with ${result.characterCount.toLocaleString()} characters`)
    } catch (error) {
      if (request !== requestRef.current) return
      const message = errorMessage(error)
      setFailure(message)
      onToast(message)
    } finally {
      if (request === requestRef.current) setBusy('idle')
    }
  }

  const readImage = async () => {
    const request = ++imageRequestRef.current
    setImageBusy('reading')
    setImageFailure('')
    try {
      const next = desktop ? await api.pasteboardImageRead(udid) : demoImageSnapshot()
      if (request !== imageRequestRef.current) return
      setImageSnapshot(next)
      setImageReadStale(false)
    } catch (error) {
      if (request !== imageRequestRef.current) return
      const message = errorMessage(error)
      setImageFailure(message)
      onToast(message)
    } finally {
      if (request === imageRequestRef.current) setImageBusy('idle')
    }
  }

  const chooseImage = async () => {
    if (imageBusy !== 'idle') return
    const request = ++imageRequestRef.current
    setImageBusy('preparing')
    setImageFailure('')
    try {
      let prepared: PasteboardImagePreparation
      if (desktop) {
        const localPath = await dialogs.file('Pasteboard image', ['png', 'jpg', 'jpeg'])
        if (request !== imageRequestRef.current || typeof localPath !== 'string') return
        prepared = await api.pasteboardImagePrepare(localPath, udid)
      } else {
        prepared = demoImagePreparation()
      }
      if (request !== imageRequestRef.current) {
        if (desktop) void api.pasteboardImageDiscard(prepared.preparationId)
        return
      }
      imagePreparationRef.current = prepared
      setImagePreparation(prepared)
    } catch (error) {
      if (request !== imageRequestRef.current) return
      const message = errorMessage(error)
      setImageFailure(message)
      onToast(message)
    } finally {
      if (request === imageRequestRef.current) setImageBusy('idle')
    }
  }

  const clearPreparedImage = async () => {
    const prepared = imagePreparationRef.current
    if (!prepared || imageBusy !== 'idle') return
    ++imageRequestRef.current
    imagePreparationRef.current = null
    setImagePreparation(null)
    if (desktop) {
      try {
        await api.pasteboardImageDiscard(prepared.preparationId)
      } catch (error) {
        const message = errorMessage(error)
        setImageFailure(message)
        onToast(message)
      }
    }
  }

  const writeImage = async () => {
    const prepared = imagePreparationRef.current
    if (!prepared || imageBusy !== 'idle') return
    const lifecycle = imageRequestRef.current
    const confirmed = await dialogs.confirmDestructive(
      `Replace the general pasteboard on ${deviceName} with ${prepared.fileName} (${prepared.width.toLocaleString()} × ${prepared.height.toLocaleString()}, ${bytes(prepared.byteLength)}, ${prepared.mimeType})? The existing pasteboard content will be replaced.`,
      'Replace Pasteboard',
    )
    if (!confirmed || lifecycle !== imageRequestRef.current) return

    const request = ++imageRequestRef.current
    setImageBusy('writing')
    setImageFailure('')
    try {
      const result = desktop
        ? await api.pasteboardImageWrite(prepared.preparationId, udid)
        : {
            mimeType: prepared.mimeType,
            width: prepared.width,
            height: prepared.height,
            byteLength: prepared.byteLength,
            transport: 'CoreDevice Pasteboard · demonstration',
          }
      if (request !== imageRequestRef.current) return
      imagePreparationRef.current = null
      setImagePreparation(null)
      setLastImageWrite(result)
      setImageReadStale(Boolean(imageSnapshot))
      onToast(`Replaced ${deviceName} pasteboard with ${result.width.toLocaleString()} × ${result.height.toLocaleString()} image`)
    } catch (error) {
      if (request !== imageRequestRef.current) return
      const message = errorMessage(error)
      setImageFailure(message)
      onToast(message)
    } finally {
      if (request === imageRequestRef.current) setImageBusy('idle')
    }
  }

  return (
    <section className="pasteboard-page">
      <div className="pasteboard-tabs" role="tablist" aria-label="Pasteboard transfer type">
        <button role="tab" aria-selected={mode === 'text'} className={mode === 'text' ? 'active' : ''} onClick={() => setMode('text')}>
          <ClipboardPaste size={14} /> Text
        </button>
        <button role="tab" aria-selected={mode === 'image'} className={mode === 'image' ? 'active' : ''} onClick={() => setMode('image')}>
          <ImageIcon size={14} /> Image
        </button>
      </div>

      <div className="pasteboard-summary">
        <div className="card"><small>Transfer mode</small><b>{mode === 'text' ? 'Plain text' : 'Image'}</b><span>{mode === 'text' ? 'UTF-8 only' : 'PNG or JPEG'}</span></div>
        <div className="card"><small>Safety limit</small><b>{mode === 'text' ? '1 MB' : '12 MB'}</b><span>per read or write</span></div>
        <div className="card"><small>Device items</small><b>{mode === 'text' ? snapshot?.itemCount ?? '—' : imageSnapshot?.itemCount ?? '—'}</b><span>{mode === 'text'
          ? snapshot?.changeCount === null || snapshot?.changeCount === undefined ? 'read on demand' : `change ${snapshot.changeCount}`
          : imageSnapshot?.changeCount === null || imageSnapshot?.changeCount === undefined ? 'read on demand' : `change ${imageSnapshot.changeCount}`}</span></div>
        <div className="card"><small>Last write</small><b>{mode === 'text'
          ? lastWrite ? bytes(lastWrite.byteLength) : '—'
          : lastImageWrite ? bytes(lastImageWrite.byteLength) : '—'}</b><span>{mode === 'text'
          ? lastWrite?.transport ?? 'no write this session'
          : lastImageWrite?.transport ?? 'no write this session'}</span></div>
      </div>

      {mode === 'text' && failure && <div className="process-error" role="alert">{failure}</div>}
      {mode === 'image' && imageFailure && <div className="process-error" role="alert">{imageFailure}</div>}

      {mode === 'text'
        ? <div className="pasteboard-layout">
            <div className="pasteboard-device card">
              <header>
                <div><span><ClipboardPaste size={17} /></span><div><small>Device → Mac</small><h3>Read device text</h3></div></div>
                <button disabled={busy !== 'idle'} onClick={() => void read()}>
                  <RefreshCw className={busy === 'reading' ? 'spinning' : ''} size={14} />
                  {busy === 'reading' ? 'Reading…' : 'Read from Device'}
                </button>
              </header>

              {snapshot && snapshot.text !== null
                ? <>
                    {readStale && <div className="pasteboard-stale">The device pasteboard may have changed after the last write. Read again to refresh.</div>}
                    <textarea aria-label="Device pasteboard text" value={snapshot.text} readOnly spellCheck={false} />
                    <footer>
                      <span>{snapshot.characterCount.toLocaleString()} characters · {bytes(snapshot.byteLength)}</span>
                      <button onClick={() => setEditor(snapshot.text ?? '')}><ArrowRight size={13} /> Use in Editor</button>
                    </footer>
                  </>
                : <div className="pasteboard-empty">
                    <Download size={25} />
                    <b>{snapshot ? 'No readable plain text' : 'Device text has not been read'}</b>
                    <p>{snapshot?.message ?? 'Reading happens only when you press Read from Device. No background clipboard monitoring is used.'}</p>
                  </div>}
              {snapshot && <div className="pasteboard-route"><span>{snapshot.state}</span><code>{snapshot.transport}</code></div>}
            </div>

            <div className="pasteboard-editor card">
              <header>
                <div><small>Mac → Device</small><h3>Text to send</h3></div>
                <button aria-label="Clear editor" disabled={!editor || busy !== 'idle'} onClick={() => setEditor('')}><Eraser size={14} /></button>
              </header>
              <textarea
                aria-label="Text to send"
                value={editor}
                disabled={busy !== 'idle'}
                onChange={(event) => setEditor(event.target.value)}
                placeholder="Enter or paste text here…"
                spellCheck={false}
              />
              <div className={`pasteboard-editor-count ${oversized ? 'oversized' : ''}`}>
                <span>{editorCharacters.toLocaleString()} characters</span>
                <span>{bytes(editorBytes)} / 1.00 MB</span>
              </div>
              {oversized && <div className="pasteboard-limit" role="alert">Text exceeds the 1 MB transfer limit.</div>}
              <button className="primary-button pasteboard-write" disabled={!editor || oversized || busy !== 'idle'} onClick={() => void write()}>
                <Send size={14} />{busy === 'writing' ? 'Replacing…' : 'Replace Device Pasteboard'}
              </button>
              <p>Writing replaces the device&apos;s general pasteboard with exactly this UTF-8 text. You will be asked to confirm every write.</p>
            </div>
          </div>
        : <div className="pasteboard-layout pasteboard-image-layout">
            <div className="pasteboard-device card">
              <header>
                <div><span><ImageIcon size={17} /></span><div><small>Device → Mac</small><h3>Read device image</h3></div></div>
                <button disabled={imageBusy !== 'idle'} onClick={() => void readImage()}>
                  <RefreshCw className={imageBusy === 'reading' ? 'spinning' : ''} size={14} />
                  {imageBusy === 'reading' ? 'Reading…' : 'Read Image from Device'}
                </button>
              </header>

              {imageSnapshot?.dataUrl && imageSnapshot.width !== null && imageSnapshot.height !== null
                ? <>
                    {imageReadStale && <div className="pasteboard-stale">The device pasteboard may have changed after the last write. Read again to refresh.</div>}
                    <div className="pasteboard-image-preview"><img src={imageSnapshot.dataUrl} alt="Device pasteboard preview" /></div>
                    <footer><span>{imageSnapshot.width.toLocaleString()} × {imageSnapshot.height.toLocaleString()} · {bytes(imageSnapshot.byteLength)} · {imageSnapshot.mimeType}</span></footer>
                  </>
                : <div className="pasteboard-empty">
                    <Download size={25} />
                    <b>{imageSnapshot ? 'No readable PNG or JPEG' : 'Device image has not been read'}</b>
                    <p>{imageSnapshot?.message ?? 'Reading happens only when you press Read Image from Device. Unknown-size and oversized images stay on the device.'}</p>
                  </div>}
              {imageSnapshot && <div className="pasteboard-route"><span>{imageSnapshot.state}</span><code>{imageSnapshot.transport}</code></div>}
            </div>

            <div className="pasteboard-editor pasteboard-image-editor card">
              <header>
                <div><small>Mac → Device</small><h3>Image to send</h3></div>
                <div className="pasteboard-image-actions">
                  <button disabled={imageBusy !== 'idle'} onClick={() => void chooseImage()}><Upload size={14} />{imageBusy === 'preparing' ? 'Preparing…' : 'Choose Image'}</button>
                  {imagePreparation && <button aria-label="Clear selected image" disabled={imageBusy !== 'idle'} onClick={() => void clearPreparedImage()}><X size={14} /></button>}
                </div>
              </header>
              {imagePreparation
                ? <>
                    <div className="pasteboard-image-preview prepared"><img src={imagePreparation.dataUrl} alt="Selected image preview" /></div>
                    <div className="pasteboard-image-details">
                      <b>{imagePreparation.fileName}</b>
                      <span>{imagePreparation.width.toLocaleString()} × {imagePreparation.height.toLocaleString()} · {bytes(imagePreparation.byteLength)} · {imagePreparation.mimeType}</span>
                    </div>
                  </>
                : <div className="pasteboard-empty pasteboard-image-pick">
                    <ImageIcon size={25} />
                    <b>No image selected</b>
                    <p>Choose a PNG or JPEG up to 12 MB. It is validated and previewed locally before any write.</p>
                  </div>}
              <button className="primary-button pasteboard-write" disabled={!imagePreparation || imageBusy !== 'idle'} onClick={() => void writeImage()}>
                <Send size={14} />{imageBusy === 'writing' ? 'Replacing…' : 'Replace Device Pasteboard'}
              </button>
              <p>Writing sends exactly the prepared image and replaces the device&apos;s general pasteboard. You will be asked to confirm the target, dimensions, format, and size.</p>
            </div>
          </div>}

      <div className="pasteboard-privacy">
        <ShieldAlert size={17} />
        {mode === 'text'
          ? <span><b>Clipboard contents may contain passwords, codes, or personal data.</b> Reads are manual and limited to a bounded text item. Images and other types are never downloaded by the text workflow, and clipboard changes are not monitored in the background.</span>
          : <span><b>Images may contain screenshots or other personal data.</b> Reads are manual and resolve only a declared PNG/JPEG of 12 MB or less. Other types, unknown-size items, and clipboard changes are not monitored or downloaded.</span>}
      </div>
    </section>
  )
}
