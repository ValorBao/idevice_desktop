import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { PasteboardImagePreparation, PasteboardImageSnapshot, PasteboardTextSnapshot } from '../api'

const backend = vi.hoisted(() => ({
  api: {
    pasteboardTextRead: vi.fn(),
    pasteboardTextWrite: vi.fn(),
    pasteboardImageRead: vi.fn(),
    pasteboardImagePrepare: vi.fn(),
    pasteboardImageWrite: vi.fn(),
    pasteboardImageDiscard: vi.fn(),
  },
  dialogs: {
    confirmDestructive: vi.fn(),
    file: vi.fn(),
  },
}))

vi.mock('../api', () => ({
  ...backend,
  errorMessage: (error: unknown) => String(error),
}))

import { Pasteboard } from './Pasteboard'

const snapshot = (overrides: Partial<PasteboardTextSnapshot> = {}): PasteboardTextSnapshot => ({
  text: 'hello device',
  byteLength: 12,
  characterCount: 12,
  changeCount: 42,
  itemCount: 1,
  state: 'text',
  message: null,
  transport: 'CoreDevice Pasteboard · RemotePairing/RSD',
  ...overrides,
})

const imageSnapshot = (overrides: Partial<PasteboardImageSnapshot> = {}): PasteboardImageSnapshot => ({
  dataUrl: 'data:image/png;base64,device-preview',
  mimeType: 'image/png',
  width: 1179,
  height: 2556,
  byteLength: 245760,
  changeCount: 43,
  itemCount: 1,
  state: 'image',
  message: null,
  transport: 'CoreDevice Pasteboard · RemotePairing/RSD',
  ...overrides,
})

const preparation = (overrides: Partial<PasteboardImagePreparation> = {}): PasteboardImagePreparation => ({
  preparationId: 'prepared-1',
  dataUrl: 'data:image/png;base64,local-preview',
  mimeType: 'image/png',
  width: 120,
  height: 80,
  byteLength: 4096,
  fileName: 'photo.png',
  ...overrides,
})

describe('Pasteboard transfer', () => {
  beforeEach(() => {
    backend.api.pasteboardTextRead.mockResolvedValue(snapshot())
    backend.api.pasteboardTextWrite.mockResolvedValue({
      byteLength: 12,
      characterCount: 12,
      transport: 'CoreDevice Pasteboard · RemotePairing/RSD',
    })
    backend.api.pasteboardImageRead.mockResolvedValue(imageSnapshot())
    backend.api.pasteboardImagePrepare.mockResolvedValue(preparation())
    backend.api.pasteboardImageWrite.mockResolvedValue({
      mimeType: 'image/png',
      width: 120,
      height: 80,
      byteLength: 4096,
      transport: 'CoreDevice Pasteboard · RemotePairing/RSD',
    })
    backend.api.pasteboardImageDiscard.mockResolvedValue(true)
    backend.dialogs.confirmDestructive.mockResolvedValue(true)
    backend.dialogs.file.mockResolvedValue('/tmp/photo.png')
  })

  it('does not inspect the device until the user explicitly requests a read', async () => {
    const user = userEvent.setup()
    render(<Pasteboard desktop udid="device-1" deviceName="Test iPhone" onToast={vi.fn()} />)

    expect(backend.api.pasteboardTextRead).not.toHaveBeenCalled()
    expect(screen.getByText('Device text has not been read')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Read from Device' }))

    await waitFor(() => expect(backend.api.pasteboardTextRead).toHaveBeenCalledWith('device-1'))
    expect(await screen.findByDisplayValue('hello device')).toHaveAttribute('readonly')
    expect(screen.getByText('CoreDevice Pasteboard · RemotePairing/RSD')).toBeInTheDocument()
  })

  it('shows a bounded non-text result without inventing clipboard text', async () => {
    const user = userEvent.setup()
    backend.api.pasteboardTextRead.mockResolvedValue(snapshot({
      text: null,
      byteLength: 0,
      characterCount: 0,
      itemCount: 2,
      state: 'non-text',
      message: 'The device pasteboard contains no supported plain-text item',
    }))
    render(<Pasteboard desktop udid="device-1" deviceName="Test iPhone" onToast={vi.fn()} />)

    await user.click(screen.getByRole('button', { name: 'Read from Device' }))

    expect(await screen.findByText('No readable plain text')).toBeInTheDocument()
    expect(screen.getByText('The device pasteboard contains no supported plain-text item')).toBeInTheDocument()
    expect(screen.queryByLabelText('Device pasteboard text')).not.toBeInTheDocument()
  })

  it('moves read text into the editor without touching either clipboard', async () => {
    const user = userEvent.setup()
    render(<Pasteboard desktop udid="device-1" deviceName="Test iPhone" onToast={vi.fn()} />)
    await user.click(screen.getByRole('button', { name: 'Read from Device' }))
    await user.click(await screen.findByRole('button', { name: 'Use in Editor' }))

    expect(screen.getByLabelText('Text to send')).toHaveValue('hello device')
    expect(backend.api.pasteboardTextWrite).not.toHaveBeenCalled()
    expect(backend.dialogs.confirmDestructive).not.toHaveBeenCalled()
  })

  it('confirms the target and exact size before replacing device text', async () => {
    const user = userEvent.setup()
    const onToast = vi.fn()
    render(<Pasteboard desktop udid="device-1" deviceName="Test iPhone" onToast={onToast} />)
    await user.type(screen.getByLabelText('Text to send'), 'hello device')
    await user.click(screen.getByRole('button', { name: 'Replace Device Pasteboard' }))

    expect(backend.dialogs.confirmDestructive).toHaveBeenCalledWith(
      expect.stringContaining('Test iPhone'),
      'Replace Pasteboard',
    )
    await waitFor(() => {
      expect(backend.api.pasteboardTextWrite).toHaveBeenCalledWith('hello device', 'device-1')
    })
    expect(onToast).toHaveBeenCalledWith('Replaced Test iPhone pasteboard with 12 characters')
  })

  it('does not write when replacement confirmation is cancelled', async () => {
    const user = userEvent.setup()
    backend.dialogs.confirmDestructive.mockResolvedValue(false)
    render(<Pasteboard desktop udid="device-1" deviceName="Test iPhone" onToast={vi.fn()} />)
    await user.type(screen.getByLabelText('Text to send'), 'keep this local')
    await user.click(screen.getByRole('button', { name: 'Replace Device Pasteboard' }))

    expect(backend.dialogs.confirmDestructive).toHaveBeenCalledOnce()
    expect(backend.api.pasteboardTextWrite).not.toHaveBeenCalled()
    expect(screen.getByLabelText('Text to send')).toHaveValue('keep this local')
  })

  it('blocks text larger than one megabyte before confirmation', () => {
    render(<Pasteboard desktop udid="device-1" deviceName="Test iPhone" onToast={vi.fn()} />)
    fireEvent.change(screen.getByLabelText('Text to send'), { target: { value: 'x'.repeat(1024 * 1024 + 1) } })

    expect(screen.getByText('Text exceeds the 1 MB transfer limit.')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Replace Device Pasteboard' })).toBeDisabled()
    expect(backend.dialogs.confirmDestructive).not.toHaveBeenCalled()
    expect(backend.api.pasteboardTextWrite).not.toHaveBeenCalled()
  })

  it('keeps the browser demonstration local while preserving confirmation', async () => {
    const user = userEvent.setup()
    render(<Pasteboard desktop={false} udid="demo-1" deviceName="Demo iPhone" onToast={vi.fn()} />)
    await user.click(screen.getByRole('button', { name: 'Read from Device' }))
    expect(await screen.findByDisplayValue(/TestFlight feedback/)).toBeInTheDocument()
    expect(backend.api.pasteboardTextRead).not.toHaveBeenCalled()

    await user.type(screen.getByLabelText('Text to send'), 'demo text')
    await user.click(screen.getByRole('button', { name: 'Replace Device Pasteboard' }))

    expect(backend.dialogs.confirmDestructive).toHaveBeenCalledOnce()
    expect(backend.api.pasteboardTextWrite).not.toHaveBeenCalled()
  })

  it('does not inspect device images until the image read is explicitly requested', async () => {
    const user = userEvent.setup()
    render(<Pasteboard desktop udid="device-1" deviceName="Test iPhone" onToast={vi.fn()} />)

    await user.click(screen.getByRole('tab', { name: 'Image' }))
    expect(backend.api.pasteboardImageRead).not.toHaveBeenCalled()
    expect(screen.getByText('Device image has not been read')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Read Image from Device' }))

    await waitFor(() => expect(backend.api.pasteboardImageRead).toHaveBeenCalledWith('device-1'))
    expect(await screen.findByRole('img', { name: 'Device pasteboard preview' })).toHaveAttribute(
      'src',
      'data:image/png;base64,device-preview',
    )
    expect(screen.getByText(/1,179 × 2,556/)).toBeInTheDocument()
  })

  it('previews a validated local image and confirms its exact transfer details', async () => {
    const user = userEvent.setup()
    const onToast = vi.fn()
    render(<Pasteboard desktop udid="device-1" deviceName="Test iPhone" onToast={onToast} />)
    await user.click(screen.getByRole('tab', { name: 'Image' }))
    await user.click(screen.getByRole('button', { name: 'Choose Image' }))

    await waitFor(() => expect(backend.api.pasteboardImagePrepare).toHaveBeenCalledWith('/tmp/photo.png', 'device-1'))
    expect(await screen.findByRole('img', { name: 'Selected image preview' })).toHaveAttribute(
      'src',
      'data:image/png;base64,local-preview',
    )
    expect(screen.getByText('photo.png')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Replace Device Pasteboard' }))

    expect(backend.dialogs.confirmDestructive).toHaveBeenCalledWith(
      expect.stringMatching(/Test iPhone.*120 × 80.*4\.0 KB.*image\/png/),
      'Replace Pasteboard',
    )
    await waitFor(() => expect(backend.api.pasteboardImageWrite).toHaveBeenCalledWith('prepared-1', 'device-1'))
    expect(onToast).toHaveBeenCalledWith('Replaced Test iPhone pasteboard with 120 × 80 image')
  })

  it('keeps the prepared image local when replacement confirmation is cancelled', async () => {
    const user = userEvent.setup()
    backend.dialogs.confirmDestructive.mockResolvedValue(false)
    render(<Pasteboard desktop udid="device-1" deviceName="Test iPhone" onToast={vi.fn()} />)
    await user.click(screen.getByRole('tab', { name: 'Image' }))
    await user.click(screen.getByRole('button', { name: 'Choose Image' }))
    await screen.findByRole('img', { name: 'Selected image preview' })
    await user.click(screen.getByRole('button', { name: 'Replace Device Pasteboard' }))

    expect(backend.api.pasteboardImageWrite).not.toHaveBeenCalled()
    expect(screen.getByRole('img', { name: 'Selected image preview' })).toBeInTheDocument()
  })

  it('discards prepared bytes when the selection is cleared', async () => {
    const user = userEvent.setup()
    render(<Pasteboard desktop udid="device-1" deviceName="Test iPhone" onToast={vi.fn()} />)
    await user.click(screen.getByRole('tab', { name: 'Image' }))
    await user.click(screen.getByRole('button', { name: 'Choose Image' }))
    await screen.findByRole('img', { name: 'Selected image preview' })
    await user.click(screen.getByRole('button', { name: 'Clear selected image' }))

    await waitFor(() => expect(backend.api.pasteboardImageDiscard).toHaveBeenCalledWith('prepared-1'))
    expect(screen.queryByRole('img', { name: 'Selected image preview' })).not.toBeInTheDocument()
  })

  it('discards prepared bytes when the page is closed', async () => {
    const user = userEvent.setup()
    const view = render(<Pasteboard desktop udid="device-1" deviceName="Test iPhone" onToast={vi.fn()} />)
    await user.click(screen.getByRole('tab', { name: 'Image' }))
    await user.click(screen.getByRole('button', { name: 'Choose Image' }))
    await screen.findByRole('img', { name: 'Selected image preview' })

    view.unmount()

    await waitFor(() => expect(backend.api.pasteboardImageDiscard).toHaveBeenCalledWith('prepared-1'))
  })

  it('keeps the image demonstration local while retaining explicit read and confirmation', async () => {
    const user = userEvent.setup()
    render(<Pasteboard desktop={false} udid="demo-1" deviceName="Demo iPhone" onToast={vi.fn()} />)
    await user.click(screen.getByRole('tab', { name: 'Image' }))
    await user.click(screen.getByRole('button', { name: 'Read Image from Device' }))
    expect(await screen.findByRole('img', { name: 'Device pasteboard preview' })).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Choose Image' }))
    await screen.findByRole('img', { name: 'Selected image preview' })
    await user.click(screen.getByRole('button', { name: 'Replace Device Pasteboard' }))

    expect(backend.dialogs.confirmDestructive).toHaveBeenCalledOnce()
    expect(backend.dialogs.file).not.toHaveBeenCalled()
    expect(backend.api.pasteboardImageRead).not.toHaveBeenCalled()
    expect(backend.api.pasteboardImagePrepare).not.toHaveBeenCalled()
    expect(backend.api.pasteboardImageWrite).not.toHaveBeenCalled()
  })
})
