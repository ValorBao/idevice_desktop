import { useCallback, useEffect, useState } from 'react'
import { devices, fileSystem, installedApps, initialLogs, liveLogPool, type Device, type LogLine } from '../src/data'

export type Station = 'inspect' | 'files' | 'apps' | 'watch'

export function useLab() {
  const [station, setStation] = useState<Station>('inspect')
  const [deviceId, setDeviceId] = useState(devices[0].id)
  const [menu, setMenu] = useState(false)
  const [toast, setToast] = useState('')
  const [path, setPath] = useState<string[]>([])
  const [paused, setPaused] = useState(false)
  const [logs, setLogs] = useState<LogLine[]>(initialLogs)
  const [appId, setAppId] = useState(installedApps[0].id)
  const [apps, setApps] = useState(installedApps)
  const device = devices.find((item) => item.id === deviceId) ?? devices[0]
  const currentPath = `/${path.join('/')}`
  const files = fileSystem[currentPath] ?? []
  const selectedApp = apps.find((app) => app.id === appId) ?? apps[0]

  const notify = useCallback((message: string) => {
    setToast(message)
  }, [])

  useEffect(() => {
    if (!toast) return
    const timer = window.setTimeout(() => setToast(''), 2200)
    return () => window.clearTimeout(timer)
  }, [toast])

  useEffect(() => {
    if (paused || station !== 'watch') return
    const timer = window.setInterval(() => {
      const [level, process, message] = liveLogPool[Math.floor(Math.random() * liveLogPool.length)]
      const now = new Date()
      const time = `${now.toLocaleTimeString('en-GB', { hour12: false })}.${String(now.getMilliseconds()).padStart(3, '0')}`
      setLogs((items) => [...items, { time, level: level as LogLine['level'], process, message }].slice(-40))
    }, 1400)
    return () => window.clearInterval(timer)
  }, [paused, station])

  const openFolder = (name: string) => setPath((items) => [...items, name])
  const goUp = () => setPath((items) => items.slice(0, -1))
  const uninstall = () => {
    if (!selectedApp || selectedApp.system) return
    setApps((items) => items.filter((item) => item.id !== selectedApp.id))
    notify(`${selectedApp.name} removed`)
  }

  return {
    station,
    setStation,
    device,
    catalog: devices,
    deviceId,
    setDeviceId,
    menu,
    setMenu,
    toast,
    notify,
    path,
    files,
    currentPath,
    openFolder,
    goUp,
    paused,
    setPaused,
    logs,
    apps,
    appId,
    setAppId,
    selectedApp,
    uninstall,
  }
}

export type Lab = ReturnType<typeof useLab>

export const stations: { id: Station; label: string; hint: string }[] = [
  { id: 'inspect', label: 'Inspect', hint: 'Hardware' },
  { id: 'files', label: 'Files', hint: 'AFC' },
  { id: 'apps', label: 'Apps', hint: 'JIT' },
  { id: 'watch', label: 'Watch', hint: 'Blackbox' },
]
