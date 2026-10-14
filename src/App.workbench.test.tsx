import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import App from './App'
import { isTrollStoreCandidateIos } from './pages/AppsWorkbench'

describe('App workbench UI', () => {
  it('renders LeftRail with all 4 primary workbenches', () => {
    render(<App />)
    expect(screen.getByRole('button', { name: 'INSPECT' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'FILES' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'APPS' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'WATCH' })).toBeInTheDocument()
  })

  it('switches workbenches smoothly and flattens Watch instruments', async () => {
    const user = userEvent.setup()
    render(<App />)

    expect(screen.getByText('Inspect Station')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'WATCH' }))
    expect(screen.getByText('Live Blackbox')).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: 'Monitor' })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: 'Processes & Performance' })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: 'Screen Studio' })).toBeInTheDocument()

    await user.click(screen.getByRole('tab', { name: 'Processes & Performance' }))
    expect(screen.getByRole('tab', { name: /Live Telemetry/ })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: /Process Control/ })).toBeInTheDocument()

    await user.click(screen.getByRole('tab', { name: 'Screen Studio' }))
    expect(screen.getByRole('tab', { name: /Live Preview/ })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: /USB Video Recorder/ })).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'APPS' }))
    expect(screen.getByText('Applications & JIT')).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: 'Applications & Sideload' })).toBeInTheDocument()
    expect(screen.queryByRole('tab', { name: 'TrollStore' })).not.toBeInTheDocument()
  })

  it('gates TrollStore visibility to iOS 15.0 through 17.0', () => {
    expect(isTrollStoreCandidateIos('14.2')).toBe(false)
    expect(isTrollStoreCandidateIos('15.0')).toBe(true)
    expect(isTrollStoreCandidateIos('16.6.1')).toBe(true)
    expect(isTrollStoreCandidateIos('17.0')).toBe(true)
    expect(isTrollStoreCandidateIos('17.0.1')).toBe(false)
    expect(isTrollStoreCandidateIos('17.4')).toBe(false)
    expect(isTrollStoreCandidateIos('17.5.1')).toBe(false)
    expect(isTrollStoreCandidateIos('26.5')).toBe(false)
  })
})
