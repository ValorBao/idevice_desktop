export type TimeRange = { startMs: number; endMs: number }

/** Matches `MIN_SEGMENT_MS` in `src-tauri/src/screen_capture.rs`, which rejects shorter segments. */
export const MIN_SEGMENT_MS = 100

/** Cuts clamped to the recording, sorted, and with overlaps merged. */
export const normalizeCuts = (cuts: TimeRange[], durationMs: number): TimeRange[] => {
  const clamped = cuts
    .map(({ startMs, endMs }) => ({
      startMs: Math.max(0, Math.min(startMs, durationMs)),
      endMs: Math.max(0, Math.min(endMs, durationMs)),
    }))
    .filter(({ startMs, endMs }) => endMs > startMs)
    .sort((a, b) => a.startMs - b.startMs)
  const merged: TimeRange[] = []
  for (const cut of clamped) {
    const last = merged[merged.length - 1]
    if (last && cut.startMs <= last.endMs) last.endMs = Math.max(last.endMs, cut.endMs)
    else merged.push({ ...cut })
  }
  return merged
}

/**
 * The parts of the recording an export keeps. Slivers shorter than
 * {@link MIN_SEGMENT_MS} left between cuts are dropped rather than exported.
 */
export const keptSegments = (durationMs: number, cuts: TimeRange[]): TimeRange[] => {
  const kept: TimeRange[] = []
  let cursor = 0
  for (const cut of normalizeCuts(cuts, durationMs)) {
    if (cut.startMs - cursor >= MIN_SEGMENT_MS) kept.push({ startMs: cursor, endMs: cut.startMs })
    cursor = cut.endMs
  }
  if (durationMs - cursor >= MIN_SEGMENT_MS) kept.push({ startMs: cursor, endMs: durationMs })
  return kept.map(({ startMs, endMs }) => ({ startMs: Math.round(startMs), endMs: Math.round(endMs) }))
}

export const totalDuration = (segments: TimeRange[]) =>
  segments.reduce((sum, { startMs, endMs }) => sum + endMs - startMs, 0)

/** Where playback should jump to when it lands inside a cut, or null when it is in kept footage. */
export const skipCut = (positionMs: number, cuts: TimeRange[]): number | null => {
  const cut = cuts.find(({ startMs, endMs }) => positionMs >= startMs && positionMs < endMs)
  return cut ? cut.endMs : null
}

export const formatClock = (milliseconds: number) => {
  const tenths = Math.max(0, Math.floor(milliseconds / 100))
  const minutes = Math.floor(tenths / 600)
  const seconds = Math.floor(tenths / 10) % 60
  return `${minutes}:${String(seconds).padStart(2, '0')}.${tenths % 10}`
}
