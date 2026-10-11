import { describe, expect, it, vi } from 'vitest'
import { COMMAND_KIND, commandError, errorMessage, isCommandKind, reportCommand } from './commandError'

describe('commandError', () => {
  it('keeps kind and retryable from a rejected command', () => {
    const parsed = commandError({
      kind: COMMAND_KIND.ddiMissing,
      message: 'Download the Developer Disk Image once',
      retryable: false,
    })
    expect(parsed).toEqual({
      kind: 'ddi-missing',
      message: 'Download the Developer Disk Image once',
      retryable: false,
    })
  })

  it('reads a nested Tauri payload without dropping the kind', () => {
    const parsed = commandError({
      error: { kind: COMMAND_KIND.networkDiscovery, message: 'Connect over USB', retryable: true },
    })
    expect(parsed.kind).toBe('network_discovery')
    expect(parsed.retryable).toBe(true)
  })

  it('treats a plain Error as an unknown command', () => {
    const parsed = commandError(new Error('usbmuxd is unavailable'))
    expect(parsed.kind).toBe('unknown')
    expect(parsed.message).toBe('usbmuxd is unavailable')
    expect(parsed.retryable).toBe(false)
  })
})

describe('reportCommand', () => {
  it('does not toast a cancelled command as an error', () => {
    const onToast = vi.fn()
    const parsed = reportCommand({
      kind: COMMAND_KIND.cancelled,
      message: 'Live screen cancelled',
      retryable: false,
    }, onToast)
    expect(parsed.kind).toBe('cancelled')
    expect(onToast).not.toHaveBeenCalled()
  })

  it('toasts product kinds that still need a message', () => {
    const onToast = vi.fn()
    reportCommand({
      kind: COMMAND_KIND.developerMode,
      message: 'Turn on Developer Mode',
      retryable: false,
    }, onToast)
    expect(onToast).toHaveBeenCalledWith('Turn on Developer Mode')
  })
})

describe('isCommandKind', () => {
  it('matches a product kind without reading the message', () => {
    expect(isCommandKind({ kind: 'passcode', message: 'Device has a passcode set', retryable: false }, COMMAND_KIND.passcode)).toBe(true)
    expect(isCommandKind(new Error('Device has a passcode set'), COMMAND_KIND.passcode)).toBe(false)
  })
})

describe('errorMessage', () => {
  it('is the parsed message for string-only sinks', () => {
    expect(errorMessage({ kind: 'device', message: 'Device disconnected', retryable: true })).toBe('Device disconnected')
  })
})
