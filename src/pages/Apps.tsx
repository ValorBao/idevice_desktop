import { useCallback, useEffect, useState, type DragEvent } from 'react'
import { Search, Upload } from 'lucide-react'
import { installedApps, type AppInfo } from '../data'
import { api, dialogs, errorMessage, events } from '../api'
import { installedToApp } from '../lib/device'
import { AppIcon } from '../components/AppIcon'

export function Apps({ desktop, udid, onToast }: { desktop: boolean; udid: string; onToast: (message: string) => void }) {
  const [query, setQuery] = useState('')
  const [apps, setApps] = useState<AppInfo[]>(desktop ? [] : installedApps)
  const [selectedId, setSelectedId] = useState(desktop ? '' : 'a1')
  const [dragging, setDragging] = useState(false)
  const [install, setInstall] = useState<{ name: string; progress: number; phase: string } | null>(null)
  const selected = apps.find((app) => app.id === selectedId) ?? apps[0]
  const filtered = apps.filter((app) => `${app.name} ${app.bundle}`.toLowerCase().includes(query.toLowerCase()))

  const loadApps = useCallback(async () => {
    if (!desktop) return
    try {
      const loaded = (await api.appsList(udid)).filter((app) => !app.system).map(installedToApp)
      setApps(loaded)
      setSelectedId((current) => loaded.some((app) => app.id === current) ? current : loaded[0]?.id ?? '')
    } catch (error) { onToast(errorMessage(error)) }
  }, [desktop, udid, onToast])

  useEffect(() => { void loadApps() }, [loadApps])
  useEffect(() => {
    if (!desktop) return
    let unlisten: (() => void) | undefined
    events.appProgress((progress) => setInstall({ name: progress.item, progress: progress.percent, phase: progress.operation === 'uninstall' ? 'Uninstalling' : 'Installing' })).then((stop) => { unlisten = stop })
    return () => unlisten?.()
  }, [desktop])

  const startInstall = async (fileName = 'Sideloaded.ipa', suppliedPath?: string) => {
    if (install) return
    if (desktop) {
      try {
        const chosen = suppliedPath ?? await dialogs.ipa()
        if (!chosen || Array.isArray(chosen)) return
        const name = chosen.split(/[\\/]/).pop() ?? 'Application.ipa'
        setInstall({ name, progress: 0, phase: 'Preparing package' })
        await api.appInstall(chosen, udid)
        setInstall(null)
        await loadApps()
        onToast(`${name} installed`)
      } catch (error) {
        setInstall(null)
        onToast(errorMessage(error))
      }
      return
    }
    const safeName = fileName.toLowerCase().endsWith('.ipa') ? fileName : `${fileName}.ipa`
    setInstall({ name: safeName, progress: 0, phase: 'Verifying signature' })
  }

  useEffect(() => {
    if (!install || desktop) return
    const timer = window.setInterval(() => {
      setInstall((current) => {
        if (!current) return null
        const next = Math.min(100, current.progress + 8)
        if (next >= 100) {
          window.clearInterval(timer)
          const name = current.name.replace(/\.ipa$/i, '')
          const app: AppInfo = { id: `x${Date.now()}`, name, bundle: `com.sideload.${name.toLowerCase().replace(/[^a-z0-9]/g, '')}`, version: '1.0.0', size: '18.6 MB', color: '#e0566f', fresh: true }
          setApps((items) => [app, ...items])
          setSelectedId(app.id)
          onToast(`${name} installed`)
          return null
        }
        return { ...current, progress: next, phase: next < 32 ? 'Verifying signature' : next < 68 ? 'Transferring package' : next < 94 ? 'Installing' : 'Finalizing' }
      })
    }, 180)
    return () => window.clearInterval(timer)
  }, [install?.name, onToast, desktop])

  const handleDrop = (event: DragEvent<HTMLDivElement>) => {
    event.preventDefault()
    setDragging(false)
    const file = event.dataTransfer.files[0]
    void startInstall(file?.name ?? 'Sideloaded.ipa', (file as File & { path?: string })?.path)
  }

  const uninstallSelected = async () => {
    if (!selected) return
    if (!desktop) {
      setApps((items) => items.filter((item) => item.id !== selected.id))
      onToast(`${selected.name} uninstalled`)
      return
    }
    const confirmed = await dialogs.confirmDestructive(
      `Uninstall “${selected.name}”? Its data on the device is removed with it.`,
      'Uninstall',
    )
    if (!confirmed) return
    try {
      setInstall({ name: selected.name, progress: 0, phase: 'Uninstalling' })
      await api.appUninstall(selected.bundle, udid)
      setInstall(null)
      await loadApps()
      onToast(`${selected.name} uninstalled`)
    } catch (error) { setInstall(null); onToast(errorMessage(error)) }
  }

  return (
    <section className="apps-page page-padding compact-padding">
      <div className="apps-list-panel">
        <div className="apps-toolbar"><label><Search size={15} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search installed apps…" /></label><button className="primary-button" onClick={() => void startInstall()}><Upload size={15} />Install signed .ipa</button></div>
        <div className={`drop-zone ${dragging ? 'dragging' : ''}`} onClick={() => void startInstall()} onDragOver={(event) => { event.preventDefault(); setDragging(true) }} onDragLeave={() => setDragging(false)} onDrop={handleDrop}><span><Upload size={19} /></span><div><b>Drop a signed .ipa here to sideload</b><small>Code signature required · verified before transfer</small></div></div>
        {install && <div className="install-card card"><div><b>{install.name}</b><code>{install.progress}%</code></div><div className="progress"><span style={{ width: `${install.progress}%` }} /></div><small>{install.phase}…</small></div>}
        <div className="app-list">{filtered.map((app) => <button key={app.id} className={selectedId === app.id ? 'active' : ''} onClick={() => setSelectedId(app.id)}><AppIcon app={app} /><span><b>{app.name}</b><small>{app.bundle}</small></span><code className={app.fresh ? 'fresh' : ''}>{app.fresh ? 'new' : app.system ? 'system' : 'user'}</code><small className="app-size">{app.size}</small></button>)}</div>
      </div>
      {selected ? <aside className="app-detail card"><AppIcon app={selected} large /><h2>{selected.name}</h2><code>{selected.bundle}</code><div>{[['Version', selected.version], ['Size', selected.size], ['Type', selected.system ? 'System' : 'User'], ['Bundle', selected.bundle.split('.').slice(-1)[0] ?? '']].map(([label, value]) => <p key={label}><span>{label}</span><b>{value}</b></p>)}</div><button className="uninstall-button" onClick={() => void uninstallSelected()} disabled={selected.system}>Uninstall</button></aside> : <aside className="app-detail card empty-card">No applications returned.</aside>}
    </section>
  )
}
