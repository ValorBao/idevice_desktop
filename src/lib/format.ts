

export const bytes = (value: number | null | undefined) => {
  if (!value) return '—'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const power = Math.min(units.length - 1, Math.floor(Math.log(value) / Math.log(1024)))
  return `${(value / 1024 ** power).toFixed(power > 2 ? 1 : 0)} ${units[power]}`
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
