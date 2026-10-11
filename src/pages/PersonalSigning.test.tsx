import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { PersonalSigningPreflight, PersonalSigningResult } from '../api'

const backend = vi.hoisted(() => ({
  api: {
    personalSigningPreflight: vi.fn(),
    personalSigningExport: vi.fn(),
    appInstall: vi.fn(),
    personalAccountStatus: vi.fn(),
  },
  dialogs: {
    ipa: vi.fn(),
    provisioningProfile: vi.fn(),
    signedIpaDestination: vi.fn(),
  },
  events: {
    personalSigningProgress: vi.fn(),
    appProgress: vi.fn(),
    personalAccountTwoFactor: vi.fn(),
    personalAccountProgress: vi.fn(),
  },
}))

vi.mock('../api', () => ({
  ...backend,
  errorMessage: (error: unknown) => error instanceof Error ? error.message : String(error),
}))

import { PersonalSigning } from './PersonalSigning'

const preflight: PersonalSigningPreflight = {
  ipaName: 'Example.ipa',
  appName: 'Example',
  bundleId: 'com.example.app',
  version: '1.0',
  profileName: 'Example Development',
  profileUuid: 'PROFILE-UUID',
  teamIdentifier: 'TEAM123456',
  applicationIdentifier: 'TEAM123456.com.example.app',
  expiresAt: '2027-08-30T00:00:00Z',
  deviceCount: 2,
  deviceIncluded: true,
  identities: [{
    hash: '0123456789ABCDEF0123456789ABCDEF01234567',
    name: 'Apple Development: Example (TEAM123456)',
    teamIdentifier: 'TEAM123456',
    matchesProfile: true,
  }],
  selectedIdentityHash: '0123456789ABCDEF0123456789ABCDEF01234567',
  ready: true,
  blockers: [],
  warnings: [],
}

const signed: PersonalSigningResult = {
  outputPath: '/tmp/Example-personal-signed.ipa',
  appName: 'Example',
  bundleId: 'com.example.app',
  profileName: 'Example Development',
  identityName: 'Apple Development: Example (TEAM123456)',
  sizeBytes: 12_000_000,
}

describe('Personal Signing Assistant', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    backend.dialogs.ipa.mockResolvedValue('/tmp/Example.ipa')
    backend.dialogs.provisioningProfile.mockResolvedValue('/tmp/Example.mobileprovision')
    backend.dialogs.signedIpaDestination.mockResolvedValue(signed.outputPath)
    backend.api.personalSigningPreflight.mockResolvedValue(preflight)
    backend.api.personalSigningExport.mockResolvedValue(signed)
    backend.api.appInstall.mockResolvedValue(undefined)
    backend.api.personalAccountStatus.mockResolvedValue({
      loggedIn: false,
      email: null,
      teamId: null,
      teamName: null,
      anisetteServer: 'https://ani.sidestore.io',
      credentialStorage: 'macOS Keychain',
      passwordStored: false,
    })
    backend.events.personalSigningProgress.mockResolvedValue(vi.fn())
    backend.events.appProgress.mockResolvedValue(vi.fn())
    backend.events.personalAccountTwoFactor.mockResolvedValue(vi.fn())
    backend.events.personalAccountProgress.mockResolvedValue(vi.fn())
  })

  it('preflights two local inputs and exports with the matching identity', async () => {
    const user = userEvent.setup()
    const onToast = vi.fn()
    render(<PersonalSigning desktop udid="device-1" onToast={onToast} />)

    await user.click(screen.getByRole('tab', { name: 'Local profile' }))
    await user.click(screen.getByRole('button', { name: /Choose an IPA/ }))
    await user.click(screen.getByRole('button', { name: /Choose a provisioning profile/ }))

    expect(await screen.findByText('Ready to sign')).toBeInTheDocument()
    expect(backend.api.personalSigningPreflight).toHaveBeenCalledWith(
      '/tmp/Example.ipa',
      '/tmp/Example.mobileprovision',
      'device-1',
    )

    await user.click(screen.getByRole('button', { name: 'Sign and export IPA' }))

    await waitFor(() => expect(backend.api.personalSigningExport).toHaveBeenCalledWith({
      ipaPath: '/tmp/Example.ipa',
      profilePath: '/tmp/Example.mobileprovision',
      identityHash: preflight.identities[0].hash,
      outputPath: signed.outputPath,
    }, 'device-1'))
    expect(await screen.findByText('Example-personal-signed.ipa')).toBeInTheDocument()
    expect(onToast).toHaveBeenCalledWith('Example signed successfully')
  })

  it('installs only the newly exported IPA', async () => {
    const user = userEvent.setup()
    render(<PersonalSigning desktop udid="device-1" onToast={vi.fn()} />)
    await user.click(screen.getByRole('tab', { name: 'Local profile' }))
    await user.click(screen.getByRole('button', { name: /Choose an IPA/ }))
    await user.click(screen.getByRole('button', { name: /Choose a provisioning profile/ }))
    await user.click(await screen.findByRole('button', { name: 'Sign and export IPA' }))
    await user.click(await screen.findByRole('button', { name: 'Install on device' }))

    expect(backend.api.appInstall).toHaveBeenCalledWith(signed.outputPath, 'device-1')
  })

  it('shows backend blockers and keeps export disabled', async () => {
    const user = userEvent.setup()
    backend.api.personalSigningPreflight.mockResolvedValue({
      ...preflight,
      ready: false,
      deviceIncluded: false,
      blockers: ['The connected device is not included in this provisioning profile.'],
    })
    render(<PersonalSigning desktop udid="device-1" onToast={vi.fn()} />)
    await user.click(screen.getByRole('tab', { name: 'Local profile' }))
    await user.click(screen.getByRole('button', { name: /Choose an IPA/ }))
    await user.click(screen.getByRole('button', { name: /Choose a provisioning profile/ }))

    expect(await screen.findByText('Action required')).toBeInTheDocument()
    expect(screen.getByText('The connected device is not included in this provisioning profile.')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Sign and export IPA' })).toBeDisabled()
  })

  it('keeps demonstration mode local and does not invoke the desktop backend', async () => {
    const user = userEvent.setup()
    render(<PersonalSigning desktop={false} udid="demo-device" onToast={vi.fn()} />)
    await user.click(screen.getByRole('tab', { name: 'Local profile' }))
    await user.click(screen.getByRole('button', { name: /Choose an IPA/ }))
    await user.click(screen.getByRole('button', { name: /Choose a provisioning profile/ }))

    expect(await screen.findByText('Ready to sign')).toBeInTheDocument()
    expect(backend.api.personalSigningPreflight).not.toHaveBeenCalled()
  })
})
