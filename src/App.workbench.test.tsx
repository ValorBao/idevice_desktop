import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import App from './App'

describe('App workbench UI', () => {
  it('renders LeftRail with all 4 primary workbenches', () => {
    render(<App />)
    expect(screen.getByRole('button', { name: 'INSPECT' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'FILES' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'APPS' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'WATCH' })).toBeInTheDocument()
  })

  it('switches workbenches smoothly and exposes only core streamlined tabs', async () => {
    const user = userEvent.setup()
    render(<App />)

    expect(screen.getByText('Inspect Station')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'WATCH' }))
    expect(screen.getByText('Live Blackbox')).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: 'Monitor' })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: 'Processes & Performance' })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: 'Screen Studio' })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: 'Location' })).toBeInTheDocument()
    expect(screen.queryByRole('tab', { name: 'Network Capture' })).not.toBeInTheDocument()
    expect(screen.queryByRole('tab', { name: 'Notification Observation' })).not.toBeInTheDocument()

    await user.click(screen.getByRole('tab', { name: 'Processes & Performance' }))
    expect(screen.getByRole('tab', { name: /Live Telemetry/ })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: /Process Control/ })).toBeInTheDocument()

    await user.click(screen.getByRole('tab', { name: 'Screen Studio' }))
    expect(screen.getByRole('tab', { name: /Live Preview/ })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: /USB Video Recorder/ })).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'FILES' }))
    expect(screen.getByRole('tab', { name: 'AFC & Sandboxes' })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: 'Pasteboard' })).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'APPS' }))
    expect(screen.getByText('Applications & JIT')).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: 'Applications & Sideload' })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: 'Personal Sign' })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: 'JIT & Debugger Tunnel' })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: 'TrollStore' })).toBeInTheDocument()
    expect(screen.queryByRole('tab', { name: 'Test Lab' })).not.toBeInTheDocument()
    expect(screen.queryByRole('tab', { name: 'Provisioning Profiles' })).not.toBeInTheDocument()
  })
})
