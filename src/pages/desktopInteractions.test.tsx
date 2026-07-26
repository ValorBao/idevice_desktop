import { act, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { Apps } from './Apps'
import { Files } from './Files'

const backend = vi.hoisted(() => ({
  api: {
    fileSharingApps: vi.fn(),
    afcList: vi.fn(),
    afcMkdir: vi.fn(),
    afcCreateFile: vi.fn(),
    afcRename: vi.fn(),
    afcRemove: vi.fn(),
    afcUpload: vi.fn(),
    afcDownload: vi.fn(),
    afcTransferCancel: vi.fn(),
    appsList: vi.fn(),
    appInstall: vi.fn(),
    appUninstall: vi.fn(),
  },
  dialogs: {
    anyFile: vi.fn(),
    saveFile: vi.fn(),
    ipa: vi.fn(),
    confirmDestructive: vi.fn(),
  },
  events: {
    transferProgress: vi.fn(),
    appProgress: vi.fn(),
  },
}))

const nativeDrop = vi.hoisted(() => ({
  handler: undefined as undefined | ((paths: string[], element: Element | null) => void),
}))

const eventHandlers = vi.hoisted(() => ({
  transfer: undefined as undefined | ((progress: { operation: string; item: string; percent: number }) => void),
}))

vi.mock('../api', () => ({
  ...backend,
  errorMessage: (error: unknown) =>
    typeof error === 'object' && error && 'message' in error
      ? String((error as { message: unknown }).message)
      : String(error),
}))

vi.mock('../lib/useFileDrop', () => ({
  useFileDrop: (
    _target: unknown,
    _enabled: boolean,
    onDrop: (paths: string[], element: Element | null) => void,
  ) => {
    nativeDrop.handler = onDrop
    return false
  },
}))

const remoteFile = {
  name: 'notes.txt',
  path: '/notes.txt',
  kind: 'Document',
  isDirectory: false,
  size: 1024,
  modified: '2026-07-26 10:00:00',
  unreadable: false,
}

const installedApp = {
  bundleId: 'com.example.sample',
  name: 'Sample App',
  version: '1.2.3',
  sizeBytes: 2048,
  system: false,
  iconDataUrl: null,
  raw: null,
}

describe('desktop interaction regressions', () => {
  beforeEach(() => {
    backend.api.fileSharingApps.mockResolvedValue([])
    backend.api.afcList.mockResolvedValue([remoteFile])
    backend.api.afcMkdir.mockResolvedValue(undefined)
    backend.api.afcRemove.mockResolvedValue(undefined)
    backend.api.afcUpload.mockResolvedValue(undefined)
    backend.api.afcTransferCancel.mockResolvedValue(undefined)
    backend.api.appsList.mockResolvedValue([installedApp])
    backend.api.appInstall.mockResolvedValue(undefined)
    backend.api.appUninstall.mockResolvedValue(undefined)
    backend.events.transferProgress.mockImplementation((handler) => {
      eventHandlers.transfer = handler
      return Promise.resolve(vi.fn())
    })
    backend.events.appProgress.mockResolvedValue(vi.fn())
    nativeDrop.handler = undefined
    eventHandlers.transfer = undefined
  })

  it('creates a folder through the in-app prompt', async () => {
    const user = userEvent.setup()
    const onToast = vi.fn()
    render(<Files desktop udid="device-1" onToast={onToast} />)

    await screen.findByText('notes.txt')
    await user.click(screen.getByRole('button', { name: 'New folder' }))
    await user.type(screen.getByRole('textbox', { name: 'Folder name' }), 'Exports')
    await user.click(screen.getByRole('button', { name: 'Create' }))

    await waitFor(() => {
      expect(backend.api.afcMkdir).toHaveBeenCalledWith('/Exports', 'device-1', undefined)
    })
    expect(onToast).toHaveBeenCalledWith('Exports created')
  })

  it('does not delete until the desktop confirmation resolves true', async () => {
    const user = userEvent.setup()
    const onToast = vi.fn()
    backend.dialogs.confirmDestructive.mockResolvedValueOnce(false).mockResolvedValueOnce(true)
    render(<Files desktop udid="device-1" onToast={onToast} />)

    const fileName = await screen.findByText('notes.txt')
    await user.click(fileName.closest('button')!)
    const deleteButton = screen.getByRole('button', { name: 'Delete' })

    await user.click(deleteButton)
    expect(backend.dialogs.confirmDestructive).toHaveBeenCalledOnce()
    expect(backend.api.afcRemove).not.toHaveBeenCalled()

    await user.click(deleteButton)
    await waitFor(() => {
      expect(backend.api.afcRemove).toHaveBeenCalledWith('/notes.txt', false, 'device-1', undefined)
    })
    expect(onToast).toHaveBeenCalledWith('notes.txt removed')
  })

  it('uploads a Finder drop into the folder row under the pointer', async () => {
    backend.api.afcList.mockResolvedValue([{
      ...remoteFile,
      name: 'Uploads',
      path: '/Uploads',
      kind: 'Folder',
      isDirectory: true,
      size: 0,
    }])
    const onToast = vi.fn()
    render(<Files desktop udid="device-1" onToast={onToast} />)

    const folderName = await screen.findByText('Uploads')
    expect(nativeDrop.handler).toBeDefined()
    act(() => nativeDrop.handler?.(['/tmp/photo.jpg'], folderName.closest('button')))

    await waitFor(() => {
      expect(backend.api.afcUpload).toHaveBeenCalledWith(
        '/tmp/photo.jpg',
        '/Uploads/photo.jpg',
        'device-1',
        undefined,
      )
    })
    expect(onToast).toHaveBeenCalledWith('photo.jpg uploaded')
  })

  it('renders transfer progress and cancels through the backend task registry', async () => {
    const user = userEvent.setup()
    render(<Files desktop udid="device-1" onToast={vi.fn()} />)
    await screen.findByText('notes.txt')
    expect(eventHandlers.transfer).toBeDefined()

    act(() => eventHandlers.transfer?.({
      operation: 'upload',
      item: 'video.mov',
      percent: 37,
    }))

    expect(screen.getByRole('status')).toHaveTextContent('Uploading video.mov')
    expect(screen.getByRole('status')).toHaveTextContent('37%')
    await user.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(backend.api.afcTransferCancel).toHaveBeenCalledOnce()
  })

  it('does not uninstall until the desktop confirmation resolves true', async () => {
    const user = userEvent.setup()
    const onToast = vi.fn()
    backend.dialogs.confirmDestructive.mockResolvedValueOnce(false).mockResolvedValueOnce(true)
    render(<Apps desktop udid="device-1" onToast={onToast} />)

    await screen.findAllByText('Sample App')
    const uninstallButton = screen.getByRole('button', { name: 'Uninstall' })

    await user.click(uninstallButton)
    expect(backend.api.appUninstall).not.toHaveBeenCalled()

    await user.click(uninstallButton)
    await waitFor(() => {
      expect(backend.api.appUninstall).toHaveBeenCalledWith('com.example.sample', 'device-1')
    })
    expect(onToast).toHaveBeenCalledWith('Sample App uninstalled')
  })

  it('accepts only IPA paths from the Tauri file-drop event', async () => {
    const onToast = vi.fn()
    render(<Apps desktop udid="device-1" onToast={onToast} />)
    await screen.findAllByText('Sample App')
    expect(nativeDrop.handler).toBeDefined()

    act(() => nativeDrop.handler?.(['/tmp/readme.txt'], null))
    expect(onToast).toHaveBeenCalledWith('Only a signed .ipa can be sideloaded')
    expect(backend.api.appInstall).not.toHaveBeenCalled()

    act(() => nativeDrop.handler?.(['/tmp/Sample.ipa'], null))
    await waitFor(() => {
      expect(backend.api.appInstall).toHaveBeenCalledWith('/tmp/Sample.ipa', 'device-1')
    })
    expect(onToast).toHaveBeenCalledWith('Sample.ipa installed')
  })

  it('keeps the browser demo path interactive without calling Tauri', async () => {
    const user = userEvent.setup()
    const onToast = vi.fn()
    render(<Apps desktop={false} udid="demo-device" onToast={onToast} />)

    await screen.findAllByText('StikDebug')
    await user.click(screen.getByRole('button', { name: 'Uninstall' }))

    expect(onToast).toHaveBeenCalledWith('StikDebug uninstalled')
    expect(backend.dialogs.confirmDestructive).not.toHaveBeenCalled()
    expect(backend.api.appUninstall).not.toHaveBeenCalled()
    expect(backend.api.appsList).not.toHaveBeenCalled()
  })
})
