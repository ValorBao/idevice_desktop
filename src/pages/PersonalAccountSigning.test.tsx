import { StrictMode } from 'react'
import { act, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { PersonalAccountStatus, PersonalAccountTwoFactor } from '../api'

const backend = vi.hoisted(() => ({
  api: {
    personalAccountStatus: vi.fn(),
    personalAccountLogin: vi.fn(),
    personalAccountSubmitTwoFactor: vi.fn(),
    personalAccountCancelTwoFactor: vi.fn(),
    personalAccountSignOut: vi.fn(),
    personalAccountSignExport: vi.fn(),
    personalAccountSignCancel: vi.fn(),
    appInstall: vi.fn(),
  },
  dialogs: {
    ipa: vi.fn(),
    signedIpaDestination: vi.fn(),
  },
  events: {
    personalAccountTwoFactor: vi.fn(),
    personalAccountProgress: vi.fn(),
  },
}))

vi.mock('../api', () => ({
  ...backend,
  errorMessage: (error: unknown) => error instanceof Error ? error.message : String(error),
}))

import { PersonalAccountSigning } from './PersonalAccountSigning'

const signedOut: PersonalAccountStatus = {
  loggedIn: false,
  email: null,
  teamId: null,
  teamName: null,
  anisetteServer: 'https://ani.sidestore.io',
  credentialStorage: 'macOS Keychain',
  passwordStored: false,
}

const signedIn: PersonalAccountStatus = {
  ...signedOut,
  loggedIn: true,
  email: 'person@example.com',
  teamId: 'TEAM123456',
  teamName: 'Personal Team',
}

const deferred = <T,>() => {
  let resolve!: (value: T) => void
  let reject!: (error: Error) => void
  const promise = new Promise<T>((done, fail) => { resolve = done; reject = fail })
  return { promise, resolve, reject }
}

describe('Apple Account personal signing', () => {
  let twoFactorHandler: ((payload: PersonalAccountTwoFactor) => void) | undefined

  beforeEach(() => {
    vi.clearAllMocks()
    twoFactorHandler = undefined
    backend.api.personalAccountStatus.mockResolvedValue(signedOut)
    backend.api.personalAccountSubmitTwoFactor.mockResolvedValue(undefined)
    backend.api.personalAccountCancelTwoFactor.mockResolvedValue(undefined)
    backend.api.personalAccountSignOut.mockResolvedValue(undefined)
    backend.api.personalAccountSignCancel.mockResolvedValue(undefined)
    backend.api.appInstall.mockResolvedValue(undefined)
    backend.dialogs.ipa.mockResolvedValue('/tmp/Example.ipa')
    backend.dialogs.signedIpaDestination.mockResolvedValue('/tmp/Example-account-signed.ipa')
    backend.events.personalAccountTwoFactor.mockImplementation((handler: (payload: PersonalAccountTwoFactor) => void) => {
      twoFactorHandler = handler
      return Promise.resolve(vi.fn())
    })
    backend.events.personalAccountProgress.mockResolvedValue(vi.fn())
  })

  it('clears the password field immediately while login is pending', async () => {
    const user = userEvent.setup()
    const login = deferred<PersonalAccountStatus>()
    backend.api.personalAccountLogin.mockReturnValue(login.promise)
    render(<PersonalAccountSigning desktop udid="device-1" deviceName="Test iPhone" onToast={vi.fn()} />)

    await screen.findByRole('heading', { name: 'Sign in' })
    await user.type(screen.getByLabelText('Apple Account'), 'person@example.com')
    await user.type(screen.getByLabelText('Password'), 'very-secret-password')
    await user.click(screen.getByRole('button', { name: 'Sign in securely' }))

    expect(screen.getByLabelText('Password')).toHaveValue('')
    expect(backend.api.personalAccountLogin).toHaveBeenCalledWith('person@example.com', 'very-secret-password')

    await act(async () => login.resolve(signedIn))
    expect(await screen.findByText('person@example.com')).toBeInTheDocument()
  })

  it('submits a six-digit code only when the backend requests 2FA', async () => {
    const user = userEvent.setup()
    render(<PersonalAccountSigning desktop udid="device-1" deviceName="Test iPhone" onToast={vi.fn()} />)
    await waitFor(() => expect(twoFactorHandler).toBeDefined())

    act(() => twoFactorHandler?.({
      lastError: null,
      unknown: false,
      sms: false,
      numbers: [],
      selectedNumberId: null,
    }))
    const code = screen.getByRole('textbox', { name: '6-digit verification code' })
    await user.type(code, '12a3456')
    expect(code).toHaveValue('123456')
    await user.click(screen.getByRole('button', { name: 'Verify' }))

    expect(backend.api.personalAccountSubmitTwoFactor).toHaveBeenCalledWith('123456')
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
  })

  it('signs the selected IPA with the active account and current device', async () => {
    const user = userEvent.setup()
    backend.api.personalAccountStatus.mockResolvedValue(signedIn)
    backend.api.personalAccountSignExport.mockResolvedValue({
      outputPath: '/tmp/Example-account-signed.ipa',
      appName: 'Example',
      bundleId: 'com.example.demo.TEAM123456',
      version: '1.0',
      teamId: 'TEAM123456',
      teamName: 'Personal Team',
      sizeBytes: 12_000_000,
    })
    render(<PersonalAccountSigning desktop udid="device-1" deviceName="Test iPhone" onToast={vi.fn()} />)

    expect(await screen.findByText('person@example.com')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: /Choose an IPA/ }))
    await user.click(screen.getByRole('button', { name: 'Sign and export IPA' }))

    await waitFor(() => expect(backend.api.personalAccountSignExport).toHaveBeenCalledWith({
      operationId: expect.any(String),
      ipaPath: '/tmp/Example.ipa',
      outputPath: '/tmp/Example-account-signed.ipa',
      udid: 'device-1',
      deviceName: 'Test iPhone',
    }))
    expect(await screen.findByText('Example-account-signed.ipa')).toBeInTheDocument()
  })
  it('cancels only the active signing operation and waits for cleanup before retry', async () => {
    const user = userEvent.setup()
    backend.api.personalAccountStatus.mockResolvedValue(signedIn)
    const signing = deferred<never>()
    backend.api.personalAccountSignExport.mockReturnValue(signing.promise)
    render(<PersonalAccountSigning desktop udid="device-1" deviceName="Test iPhone" onToast={vi.fn()} />)
    await screen.findByText('person@example.com')
    await user.click(screen.getByRole('button', { name: /Choose an IPA/ }))
    await user.click(screen.getByRole('button', { name: 'Sign and export IPA' }))
    const request = backend.api.personalAccountSignExport.mock.calls[0][0]
    await user.click(screen.getByRole('button', { name: 'Cancel signing' }))
    expect(backend.api.personalAccountSignCancel).toHaveBeenCalledWith(request.operationId)
    expect(screen.getByRole('button', { name: 'Cancelling…' })).toBeDisabled()
    await act(async () => signing.reject(new Error('Signing IPA cancelled')))
    expect(screen.getByRole('button', { name: 'Sign and export IPA' })).toBeEnabled()
    expect(screen.getByRole('alert')).toHaveTextContent('Signing IPA cancelled')
  })

  it('cancels a previous device operation and ignores its late result', async () => {
    const user = userEvent.setup()
    backend.api.personalAccountStatus.mockResolvedValue(signedIn)
    const signing = deferred<{ outputPath: string }>()
    backend.api.personalAccountSignExport.mockReturnValue(signing.promise)
    const toast = vi.fn()
    const view = render(<PersonalAccountSigning desktop udid="device-1" deviceName="One" onToast={toast} />)
    await screen.findByText('person@example.com')
    await user.click(screen.getByRole('button', { name: /Choose an IPA/ }))
    await user.click(screen.getByRole('button', { name: 'Sign and export IPA' }))
    const request = backend.api.personalAccountSignExport.mock.calls[0][0]
    view.rerender(<PersonalAccountSigning desktop udid="device-2" deviceName="Two" onToast={toast} />)
    expect(backend.api.personalAccountSignCancel).toHaveBeenCalledWith(request.operationId)
    await act(async () => signing.resolve({ outputPath: '/tmp/stale.ipa' }))
    expect(screen.queryByText('stale.ipa')).not.toBeInTheDocument()
    expect(toast).not.toHaveBeenCalled()
  })

  it('discards old progress while accepting current operation progress', async () => {
    const user = userEvent.setup()
    backend.api.personalAccountStatus.mockResolvedValue(signedIn)
    backend.api.personalAccountSignExport.mockReturnValue(new Promise(() => {}))
    render(<PersonalAccountSigning desktop udid="device-1" deviceName="One" onToast={vi.fn()} />)
    await screen.findByText('person@example.com')
    await user.click(screen.getByRole('button', { name: /Choose an IPA/ }))
    await user.click(screen.getByRole('button', { name: 'Sign and export IPA' }))
    const request = backend.api.personalAccountSignExport.mock.calls[0][0]
    const handler = backend.events.personalAccountProgress.mock.calls[0][0]
    act(() => handler({ operation: 'old', item: 'Old operation', percent: 90 }))
    expect(screen.queryByText('Old operation')).not.toBeInTheDocument()
    act(() => handler({ operation: request.operationId, item: 'Current operation', percent: 20 }))
    expect(screen.getByText('Current operation')).toBeInTheDocument()
  })

  it('releases a subscription that resolves after the StrictMode effect is replaced', async () => {
    const subscription = deferred<() => void>()
    const stop = vi.fn()
    backend.events.personalAccountProgress.mockReturnValueOnce(subscription.promise)
    render(<StrictMode><PersonalAccountSigning desktop udid="device-1" deviceName="One" onToast={vi.fn()} /></StrictMode>)
    await act(async () => subscription.resolve(stop))
    expect(stop).toHaveBeenCalledOnce()
  })

  it('signs out during a pending export and ignores its late completion', async () => {
    const user = userEvent.setup()
    backend.api.personalAccountStatus.mockResolvedValue(signedIn)
    const signing = deferred<{ outputPath: string }>()
    backend.api.personalAccountSignExport.mockReturnValue(signing.promise)
    render(<PersonalAccountSigning desktop udid="device-1" deviceName="One" onToast={vi.fn()} />)
    await screen.findByText('person@example.com')
    await user.click(screen.getByRole('button', { name: /Choose an IPA/ }))
    await user.click(screen.getByRole('button', { name: 'Sign and export IPA' }))
    await user.click(screen.getByRole('button', { name: 'Sign out of Apple Account' }))
    expect(backend.api.personalAccountSignOut).toHaveBeenCalledOnce()
    expect(await screen.findByRole('heading', { name: 'Sign in' })).toBeInTheDocument()
    await act(async () => signing.resolve({ outputPath: '/tmp/stale.ipa' }))
    expect(screen.queryByText('stale.ipa')).not.toBeInTheDocument()
  })

})
