import { useEffect, useRef, useState, type FormEvent } from 'react'
import { X } from 'lucide-react'

/**
 * In-app replacement for `window.prompt`, which never opens on the desktop:
 * wry does not implement the WKWebView text-input panel, so the native call
 * resolves to null and the action it guarded silently does nothing.
 *
 * `validate` returns an error message for a rejected value, or undefined to
 * accept it, so the caller keeps its own naming rules.
 */
export function PromptModal({
  title, description, label, placeholder, confirmLabel = 'Create', initialValue = '',
  validate, onSubmit, onClose,
}: {
  title: string
  description?: string
  label: string
  placeholder?: string
  confirmLabel?: string
  initialValue?: string
  validate?: (value: string) => string | undefined
  onSubmit: (value: string) => void
  onClose: () => void
}) {
  const [value, setValue] = useState(initialValue)
  const [error, setError] = useState<string>()
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => { inputRef.current?.focus() }, [])
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => { if (event.key === 'Escape') onClose() }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onClose])

  const submit = (event: FormEvent) => {
    event.preventDefault()
    const trimmed = value.trim()
    if (!trimmed) {
      setError(`${label} cannot be empty`)
      return
    }
    const failure = validate?.(trimmed)
    if (failure) {
      setError(failure)
      return
    }
    onSubmit(trimmed)
  }

  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <form className="pair-modal prompt-modal" onMouseDown={(event) => event.stopPropagation()} onSubmit={submit}>
        <header>
          <div><h2>{title}</h2>{description && <p>{description}</p>}</div>
          <button type="button" onClick={onClose} aria-label="Close"><X size={18} /></button>
        </header>
        <div className="prompt-body">
          <label>
            <span>{label}</span>
            <input
              ref={inputRef}
              value={value}
              placeholder={placeholder}
              onChange={(event) => { setValue(event.target.value); setError(undefined) }}
              aria-invalid={Boolean(error)}
            />
          </label>
          {error && <small className="prompt-error">{error}</small>}
        </div>
        <footer>
          <button type="button" onClick={onClose}>Cancel</button>
          <button type="submit" className="primary-button">{confirmLabel}</button>
        </footer>
      </form>
    </div>
  )
}
