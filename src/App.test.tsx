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

  it('switches workbenches smoothly when clicked', async () => {
    const user = userEvent.setup()
    render(<App />)

    expect(screen.getByText('Inspect Bench')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'WATCH' }))
    expect(screen.getByText('Live Watch Station')).toBeInTheDocument()
    expect(screen.getByText('OS Logs Console')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'APPS' }))
    expect(screen.getByText('Applications & JIT')).toBeInTheDocument()
    expect(screen.getByText('Applications & Sideload')).toBeInTheDocument()
  })
})
