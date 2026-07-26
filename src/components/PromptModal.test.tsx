import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { PromptModal } from './PromptModal'

const renderModal = (overrides: Partial<React.ComponentProps<typeof PromptModal>> = {}) => {
  const props: React.ComponentProps<typeof PromptModal> = {
    title: 'New folder',
    description: 'Created in Documents',
    label: 'Folder name',
    confirmLabel: 'Create',
    onSubmit: vi.fn(),
    onClose: vi.fn(),
    ...overrides,
  }
  render(<PromptModal {...props} />)
  return props
}

describe('PromptModal', () => {
  it('focuses the input and rejects an empty name', async () => {
    const user = userEvent.setup()
    const props = renderModal()
    const input = screen.getByRole('textbox', { name: 'Folder name' })

    expect(input).toHaveFocus()
    await user.click(screen.getByRole('button', { name: 'Create' }))

    expect(screen.getByText('Folder name cannot be empty')).toBeVisible()
    expect(props.onSubmit).not.toHaveBeenCalled()
  })

  it('trims and submits a valid value after applying caller validation', async () => {
    const user = userEvent.setup()
    const validate = vi.fn((value: string) => value === 'Existing' ? 'Already exists' : undefined)
    const props = renderModal({ validate })
    const input = screen.getByRole('textbox', { name: 'Folder name' })

    await user.type(input, ' Existing ')
    await user.click(screen.getByRole('button', { name: 'Create' }))
    expect(screen.getByText('Already exists')).toBeVisible()

    await user.clear(input)
    await user.type(input, ' Exports ')
    await user.click(screen.getByRole('button', { name: 'Create' }))

    expect(validate).toHaveBeenLastCalledWith('Exports')
    expect(props.onSubmit).toHaveBeenCalledWith('Exports')
  })

  it('closes on Escape and backdrop interaction but not inside the form', () => {
    const props = renderModal()
    const dialog = screen.getByRole('heading', { name: 'New folder' }).closest('form')!

    fireEvent.mouseDown(dialog)
    expect(props.onClose).not.toHaveBeenCalled()

    fireEvent.keyDown(window, { key: 'Escape' })
    expect(props.onClose).toHaveBeenCalledOnce()

    fireEvent.mouseDown(dialog.parentElement!)
    expect(props.onClose).toHaveBeenCalledTimes(2)
  })
})
