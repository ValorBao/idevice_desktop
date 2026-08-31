import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { ProvisioningProfileSnapshot, ProvisioningProfileSummary } from '../api'

const backend = vi.hoisted(() => ({
  api: { provisioningProfiles: vi.fn() },
}))

vi.mock('../api', () => ({
  ...backend,
  errorMessage: (error: unknown) => String(error),
}))

import { Profiles } from './Profiles'

const profile = (overrides: Partial<ProvisioningProfileSummary>): ProvisioningProfileSummary => ({
  id: 'valid-id',
  uuid: 'VALID-UUID',
  name: 'Device Lab Development',
  teamName: 'Example Team',
  teamIdentifier: 'TEAM123',
  applicationIdentifier: 'TEAM123.com.example.devicelab',
  createdAt: '2026-01-01T00:00:00Z',
  expiresAt: '2027-01-01T00:00:00Z',
  daysRemaining: 145,
  expirationState: 'valid',
  profileType: 'development',
  platforms: ['iOS'],
  deviceCount: 5,
  provisionsAllDevices: false,
  getTaskAllow: true,
  sizeBytes: 14_820,
  parseError: null,
  ...overrides,
})

const snapshot: ProvisioningProfileSnapshot = {
  transport: 'Misagent shim · CoreDeviceProxy/RSD',
  totalCount: 4,
  truncated: false,
  profiles: [
    profile({}),
    profile({
      id: 'expiring-id', uuid: 'EXPIRING-UUID', name: 'Beta Ad Hoc',
      applicationIdentifier: 'TEAM123.com.example.beta', daysRemaining: 18,
      expirationState: 'expiring', profileType: 'ad-hoc', getTaskAllow: false,
    }),
    profile({
      id: 'expired-id', uuid: 'EXPIRED-UUID', name: 'Old Enterprise', teamName: 'Enterprise Team',
      applicationIdentifier: 'ENTERPRISE.com.example.internal', daysRemaining: -45,
      expirationState: 'expired', profileType: 'enterprise', deviceCount: 0,
      provisionsAllDevices: true, getTaskAllow: false,
    }),
    profile({
      id: 'unreadable-4', uuid: null, name: 'Unreadable profile 4', teamName: null,
      teamIdentifier: null, applicationIdentifier: null, createdAt: null, expiresAt: null,
      daysRemaining: null, expirationState: 'unknown', profileType: 'unknown', platforms: [],
      deviceCount: 0, getTaskAllow: null, sizeBytes: 900,
      parseError: 'Signed profile contains no XML plist',
    }),
  ],
}

describe('Provisioning Profiles workflow', () => {
  beforeEach(() => {
    backend.api.provisioningProfiles.mockResolvedValue(snapshot)
  })

  it('loads the selected device through Misagent and renders every returned row', async () => {
    render(<Profiles desktop udid="device-1" onToast={vi.fn()} />)

    expect(await screen.findByRole('button', { name: /Device Lab Development/ })).toBeInTheDocument()
    expect(backend.api.provisioningProfiles).toHaveBeenCalledWith('device-1')
    expect(screen.getByRole('button', { name: /Beta Ad Hoc/ })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Old Enterprise/ })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Unreadable profile 4/ })).toBeInTheDocument()
    expect(screen.getByText('Misagent shim · CoreDeviceProxy/RSD')).toBeInTheDocument()
  })

  it('searches normalized metadata and opens the matching profile details', async () => {
    const user = userEvent.setup()
    render(<Profiles desktop udid="device-1" onToast={vi.fn()} />)
    const search = await screen.findByLabelText('Search profiles')

    await user.type(search, 'com.example.beta')

    expect(screen.getByRole('button', { name: /Beta Ad Hoc/ })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /Old Enterprise/ })).not.toBeInTheDocument()
    expect(screen.getByText('TEAM123.com.example.beta')).toBeInTheDocument()
    expect(screen.getAllByText('18 days remaining')).toHaveLength(2)
  })

  it('filters the list to profiles that need attention', async () => {
    const user = userEvent.setup()
    render(<Profiles desktop udid="device-1" onToast={vi.fn()} />)
    await screen.findByRole('button', { name: /Device Lab Development/ })

    await user.selectOptions(screen.getByLabelText('Filter profiles'), 'attention')

    expect(screen.queryByRole('button', { name: /Device Lab Development/ })).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Beta Ad Hoc/ })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Old Enterprise/ })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Unreadable profile 4/ })).toBeInTheDocument()
  })

  it('shows expiration and signing scope for the selected distribution profile', async () => {
    const user = userEvent.setup()
    render(<Profiles desktop udid="device-1" onToast={vi.fn()} />)
    await user.click(await screen.findByRole('button', { name: /Old Enterprise/ }))

    expect(screen.getAllByText('Expired 45 days ago')).toHaveLength(2)
    expect(screen.getByText('All devices')).toBeInTheDocument()
    expect(screen.getAllByText('Enterprise')).toHaveLength(2)
    expect(screen.getByText('ENTERPRISE.com.example.internal')).toBeInTheDocument()
    expect(screen.getByText('Not allowed')).toBeInTheDocument()
  })

  it('keeps a malformed profile visible with its local parse failure', async () => {
    const user = userEvent.setup()
    render(<Profiles desktop udid="device-1" onToast={vi.fn()} />)
    await user.click(await screen.findByRole('button', { name: /Unreadable profile 4/ }))

    expect(screen.getByText('Metadata could not be read')).toBeInTheDocument()
    expect(screen.getByText('Signed profile contains no XML plist')).toBeInTheDocument()
    expect(screen.getByText('Read-only inspection')).toBeInTheDocument()
  })

  it('reports a desktop loading failure without replacing it with demo data', async () => {
    const onToast = vi.fn()
    backend.api.provisioningProfiles.mockRejectedValue(new Error('Misagent unavailable'))
    render(<Profiles desktop udid="device-1" onToast={onToast} />)

    expect(await screen.findByRole('status')).toHaveTextContent('Error: Misagent unavailable')
    expect(onToast).toHaveBeenCalledWith('Error: Misagent unavailable')
    expect(screen.queryByText('Device Lab Development')).not.toBeInTheDocument()
  })

  it('uses bounded demonstration profiles without calling the desktop backend', async () => {
    render(<Profiles desktop={false} udid="demo-device" onToast={vi.fn()} />)

    expect(await screen.findByRole('button', { name: /Device Lab Development/ })).toBeInTheDocument()
    expect(screen.getByText('Misagent · demonstration')).toBeInTheDocument()
    expect(backend.api.provisioningProfiles).not.toHaveBeenCalled()
  })
})
