import { describe, expect, it } from 'vitest'
import { formatClock, keptSegments, normalizeCuts, skipCut, totalDuration } from './clipEdit'

describe('clip editing', () => {
  it('merges overlapping cuts and clamps them to the recording', () => {
    expect(normalizeCuts([
      { startMs: 4_000, endMs: 6_000 },
      { startMs: -500, endMs: 1_000 },
      { startMs: 5_000, endMs: 7_000 },
      { startMs: 9_000, endMs: 12_000 },
    ], 10_000)).toEqual([
      { startMs: 0, endMs: 1_000 },
      { startMs: 4_000, endMs: 7_000 },
      { startMs: 9_000, endMs: 10_000 },
    ])
  })

  it('keeps everything outside the cuts', () => {
    const kept = keptSegments(10_000, [{ startMs: 2_000, endMs: 4_000 }, { startMs: 8_000, endMs: 10_000 }])
    expect(kept).toEqual([{ startMs: 0, endMs: 2_000 }, { startMs: 4_000, endMs: 8_000 }])
    expect(totalDuration(kept)).toBe(6_000)
  })

  it('keeps the whole recording when nothing is cut and nothing when everything is', () => {
    expect(keptSegments(5_000, [])).toEqual([{ startMs: 0, endMs: 5_000 }])
    expect(keptSegments(5_000, [{ startMs: 0, endMs: 5_000 }])).toEqual([])
  })

  it('drops slivers the backend would reject', () => {
    expect(keptSegments(10_000, [{ startMs: 0, endMs: 4_950 }, { startMs: 5_000, endMs: 9_960 }]))
      .toEqual([])
  })

  it('jumps playback to the end of the cut it lands in', () => {
    const cuts = [{ startMs: 2_000, endMs: 4_000 }]
    expect(skipCut(2_500, cuts)).toBe(4_000)
    expect(skipCut(4_000, cuts)).toBeNull()
    expect(skipCut(1_000, cuts)).toBeNull()
  })

  it('formats positions as minutes, seconds and tenths', () => {
    expect(formatClock(0)).toBe('0:00.0')
    expect(formatClock(65_432)).toBe('1:05.4')
  })
})
