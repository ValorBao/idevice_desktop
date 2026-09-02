

export const bytes = (value: number | null | undefined) => {
  if (value == null || Number.isNaN(value)) return '—'
  if (value < 1024) return `${Math.round(value)} B`
  const units = ['KB', 'MB', 'GB', 'TB'] as const
  let size = value / 1024
  let unit = 0
  while (size >= 1024 && unit < units.length - 1) {
    size /= 1024
    unit += 1
  }
  return `${size < 10 ? size.toFixed(1) : size.toFixed(0)} ${units[unit]}`
}

export const duration = (milliseconds: number) => {
  const totalSeconds = Math.floor(Math.max(0, milliseconds) / 1000)
  const hours = Math.floor(totalSeconds / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)
  const seconds = totalSeconds % 60
  return hours
    ? `${hours}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`
    : `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`
}

export const displaySizeToBytes = (value?: string) => {
  if (!value) return 0
  const match = value.match(/^([\d.]+)\s*(KB|MB|GB|TB|B)$/i)
  if (!match) return 0
  const power = ['B', 'KB', 'MB', 'GB', 'TB'].indexOf(match[2].toUpperCase())
  return Number(match[1]) * 1024 ** Math.max(0, power)
}

export const flatten = (value: unknown, prefix = ''): Array<[string, string]> => {
  if (value === null || value === undefined) return [[prefix || 'value', 'null']]
  if (typeof value !== 'object') return [[prefix || 'value', String(value)]]
  if (Array.isArray(value)) return value.flatMap((item, index) => flatten(item, `${prefix}[${index}]`))
  return Object.entries(value as Record<string, unknown>).flatMap(([key, item]) => flatten(item, prefix ? `${prefix}.${key}` : key))
}

export const appColor = (bundleId: string) => {
  let hash = 0
  for (const character of bundleId) hash = (hash * 31 + character.charCodeAt(0)) | 0
  return `hsl(${Math.abs(hash) % 360} 68% 55%)`
}
