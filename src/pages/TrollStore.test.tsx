import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { TrollStoreStatus } from '../api'
import { TrollStore } from './TrollStore'

const backend = vi.hoisted(() => ({
  api: {
    trollstoreStatus: vi.fn(),
    trollstoreHelperInstall: vi.fn(),
    trollstoreIpaInstall: vi.fn(),
  },
  dialogs: {
    confirmDestructive: vi.fn(),
    ipa: vi.fn(),
  },
  events: {
    trollstoreProgress: vi.fn(),
  },
}))

vi.mock('../api', () => ({
  ...backend,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}))

const supported: TrollStoreStatus = {
  productVersion: '17.0',
  buildVersion: '21A329',
  helperSupported: true,
  helperDetail: 'iOS 17.0 (21A329) is inside the helper-install window.',
  helperCaution: null,
  trollstoreInstalled: false,
  removableApps: [{ bundleId: 'com.apple.tips', name: 'Tips', bundleName: 'Tips.app' }],
  ipaInstallAvailable: false,
  ipaInstallDetail: 'TrollStore is not installed, so an IPA cannot be handed to it.',
}

describe('TrollStore status', () => {
  beforeEach(() => {
    backend.api.trollstoreStatus.mockReset()
    backend.api.trollstoreHelperInstall.mockReset()
    backend.api.trollstoreIpaInstall.mockReset()
    backend.api.trollstoreStatus.mockResolvedValue(supported)
    backend.api.trollstoreHelperInstall.mockResolvedValue({ appName: 'Tips', message: 'The helper replaced Tips and the device is rebooting.' })
    backend.api.trollstoreIpaInstall.mockResolvedValue({ fileName: 'App.ipa', message: 'TrollStore downloaded the IPA.' })
    backend.dialogs.confirmDestructive.mockReset()
    backend.dialogs.confirmDestructive.mockResolvedValue(true)
    backend.dialogs.ipa.mockReset()
    backend.dialogs.ipa.mockResolvedValue('/tmp/App.ipa')
    backend.events.trollstoreProgress.mockReset()
    backend.events.trollstoreProgress.mockResolvedValue(vi.fn())
  })

  it('offers to replace Tips on an eligible device and does not offer an IPA install yet', async () => {
    render(<TrollStore desktop udid="udid-17" onToast={vi.fn()} />)

    expect(await screen.findByRole('button', { name: 'Replace Tips' })).toBeEnabled()
    expect(screen.getByText('iOS 17.0')).toBeInTheDocument()
    expect(screen.getByText('Build 21A329')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Install IPA' })).not.toBeInTheDocument()
    expect(backend.api.trollstoreStatus).toHaveBeenCalledWith('udid-17')
  })

  it('asks before replacing the app and skips the command when confirmation is cancelled', async () => {
    backend.dialogs.confirmDestructive.mockResolvedValue(false)
    render(<TrollStore desktop udid="udid-17" onToast={vi.fn()} />)
    await userEvent.click(await screen.findByRole('button', { name: 'Replace Tips' }))

    expect(backend.dialogs.confirmDestructive).toHaveBeenCalled()
    expect(backend.api.trollstoreHelperInstall).not.toHaveBeenCalled()
  })

  it('replaces the selected app only after confirmation', async () => {
    const onToast = vi.fn()
    render(<TrollStore desktop udid="udid-17" onToast={onToast} />)
    await userEvent.click(await screen.findByRole('button', { name: 'Replace Tips' }))

    await waitFor(() => expect(backend.api.trollstoreHelperInstall).toHaveBeenCalledWith('com.apple.tips', 'udid-17'))
    expect(onToast).toHaveBeenCalledWith('The helper replaced Tips and the device is rebooting.')
  })

  it('shows a refusal and no replace action for an unsupported build', async () => {
    backend.api.trollstoreStatus.mockResolvedValue({
      ...supported,
      productVersion: '14.2',
      buildVersion: '18B92',
      helperSupported: false,
      helperDetail: 'iOS 14.2 is below 15.0. Helper install supports iOS 15.0 through 16.7 RC (build 20H18) and iOS 17.0.',
      removableApps: [],
    })
    render(<TrollStore desktop udid="udid-14" onToast={vi.fn()} />)

    expect(await screen.findByText('Refused')).toBeInTheDocument()
    expect(screen.getByText(/below 15.0/)).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /Replace/ })).not.toBeInTheDocument()
  })

  it('offers an IPA only when TrollStore is installed', async () => {
    backend.api.trollstoreStatus.mockResolvedValue({
      ...supported,
      trollstoreInstalled: true,
      ipaInstallAvailable: true,
      ipaInstallDetail: 'TrollStore is installed. Choosing an IPA serves it on this Mac.',
    })
    const onToast = vi.fn()
    render(<TrollStore desktop udid="udid-17" onToast={onToast} />)
    await userEvent.click(await screen.findByRole('button', { name: 'Install IPA' }))

    await waitFor(() => expect(backend.api.trollstoreIpaInstall).toHaveBeenCalledWith('/tmp/App.ipa', 'udid-17'))
    expect(onToast).toHaveBeenCalledWith('TrollStore downloaded the IPA.')
  })

  it('keeps the failure on the page', async () => {
    const onToast = vi.fn()
    backend.api.trollstoreStatus.mockRejectedValue(new Error('Device locked'))
    render(<TrollStore desktop udid="udid-17" onToast={onToast} />)

    expect(await screen.findByRole('status')).toHaveTextContent('Device locked')
    expect(onToast).toHaveBeenCalledWith('Device locked')
  })

  it('does not call the desktop command in demo mode', async () => {
    render(<TrollStore desktop={false} udid="demo" onToast={vi.fn()} />)

    expect(screen.getByText('In range')).toBeInTheDocument()
    expect(screen.getByText(/Demonstration only/)).toBeInTheDocument()
    expect(backend.api.trollstoreStatus).not.toHaveBeenCalled()
    await userEvent.click(screen.getByRole('button', { name: 'Replace Tips' }))
    expect(backend.api.trollstoreHelperInstall).not.toHaveBeenCalled()
  })

  it('loads the device again when refresh is pressed', async () => {
    render(<TrollStore desktop udid="udid-17" onToast={vi.fn()} />)
    await screen.findByRole('button', { name: 'Replace Tips' })

    await userEvent.click(screen.getByRole('button', { name: 'Refresh' }))
    await waitFor(() => expect(backend.api.trollstoreStatus).toHaveBeenCalledTimes(2))
  })
})