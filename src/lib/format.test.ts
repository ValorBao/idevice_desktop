import { describe, expect, it } from 'vitest'
import { bytes, duration } from './format'

describe('bytes', () => {
  it('renders missing values as an em dash and zero as a real size', () => {
    expect(bytes(null)).toBe('—')
    expect(bytes(undefined)).toBe('—')
    expect(bytes(0)).toBe('0 B')
    expect(bytes(512)).toBe('512 B')
  })

  it('keeps one decimal below 10 of a unit and rounds above that', () => {
    expect(bytes(1536)).toBe('1.5 KB')
    expect(bytes(10 * 1024)).toBe('10 KB')
    expect(bytes(1.5 * 1024 * 1024)).toBe('1.5 MB')
  })
})

describe('duration', () => {
  it('formats elapsed capture time as mm:ss or h:mm:ss', () => {
    expect(duration(0)).toBe('00:00')
    expect(duration(65_000)).toBe('01:05')
    expect(duration(3_661_000)).toBe('1:01:01')
  })
})
