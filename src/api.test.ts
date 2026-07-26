import { afterEach, describe, expect, it, vi } from 'vitest'

const tauriDialog = vi.hoisted(() => ({
  confirm: vi.fn(),
}))

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }))
vi.mock('@tauri-apps/plugin-dialog', () => ({
  confirm: tauriDialog.confirm,
  open: vi.fn(),
  save: vi.fn(),
}))

import { dialogs } from './api'

describe('destructive confirmation', () => {
  afterEach(() => {
    delete window.__TAURI_INTERNALS__
  })

  it('uses the Tauri dialog in the desktop runtime', async () => {
    window.__TAURI_INTERNALS__ = {}
    tauriDialog.confirm.mockResolvedValue(true)
    const browserConfirm = vi.spyOn(window, 'confirm')

    await expect(dialogs.confirmDestructive('Delete the file?', 'Delete')).resolves.toBe(true)

    expect(tauriDialog.confirm).toHaveBeenCalledWith('Delete the file?', {
      title: 'Delete',
      kind: 'warning',
      okLabel: 'Delete',
      cancelLabel: 'Cancel',
    })
    expect(browserConfirm).not.toHaveBeenCalled()
  })

  it('uses the browser confirmation only in demo mode', async () => {
    const browserConfirm = vi.spyOn(window, 'confirm').mockReturnValue(false)

    await expect(dialogs.confirmDestructive('Delete the file?', 'Delete')).resolves.toBe(false)

    expect(browserConfirm).toHaveBeenCalledWith('Delete the file?')
    expect(tauriDialog.confirm).not.toHaveBeenCalled()
  })
})
