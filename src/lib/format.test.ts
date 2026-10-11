import { describe, expect, it } from 'vitest'
import { byteSize, bytes, duration } from './format'

describe('duration', () => {
  it('formats elapsed capture time as mm:ss or h:mm:ss', () => {
    expect(duration(0)).toBe('00:00')
    expect(duration(999)).toBe('00:00')
    expect(duration(65_000)).toBe('01:05')
    expect(duration(3_661_000)).toBe('1:01:01')
  })
})

describe('bytes', () => {
  it('renders zero and missing catalog sizes as an em dash', () => {
    expect(bytes(null)).toBe('—')
    expect(bytes(undefined)).toBe('—')
    expect(bytes(0)).toBe('—')
  })

  it('keeps whole units below a gigabyte', () => {
    expect(bytes(512)).toBe('512 B')
    expect(bytes(1536)).toBe('2 KB')
    expect(bytes(10 * 1024)).toBe('10 KB')
  })
})

describe('byteSize', () => {
  it('keeps a live zero visible and one decimal below ten of a unit', () => {
    expect(byteSize(null)).toBe('—')
    expect(byteSize(0)).toBe('0 B')
    expect(byteSize(1536)).toBe('1.5 KB')
    expect(byteSize(10 * 1024)).toBe('10 KB')
  })
})
