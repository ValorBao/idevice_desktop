import { useCallback, useEffect, useState } from 'react'
import { AppWindow, ChevronRight, File, Folder, HardDrive, Plus, Upload } from 'lucide-react'
import { fileSystem, installedApps } from '../data'
import { api, dialogs, errorMessage, events, type FileSharingApp, type OperationProgress, type RemoteFileEntry } from '../api'
import { bytes, displaySizeToBytes } from '../lib/format'
import { useDeviceTask } from '../lib/hooks'
import { PromptModal } from '../components/PromptModal'

type FileSource = 'media' | 'app'

const protectedMediaRoots = new Set(['Books', 'DCIM', 'PhotoData', 'Purchases', 'iTunes_Control'])
const mediaFolderDescriptions: Record<string, string> = {
  Books: 'Synced books · iOS managed',
  DCIM: 'Photos and camera videos · iOS managed',
  Downloads: 'Device media downloads',
  PhotoData: 'Photos database and thumbnails · iOS managed',
  Purchases: 'Purchased media data · iOS managed',
  iTunes_Control: 'Synced music library data · iOS managed',
}
const demoAppDocumentEntries: RemoteFileEntry[] = [
  { name: 'Exports', path: '/Exports', kind: 'Folder', isDirectory: true, size: 0, modified: '2026-07-18 10:32:00' },
  { name: 'settings.json', path: '/settings.json', kind: 'JSON', isDirectory: false, size: 14682, modified: '2026-07-17 21:08:14' },
  { name: 'session.log', path: '/session.log', kind: 'LOG', isDirectory: false, size: 32768, modified: '2026-07-18 09:46:03' },
]

const mediaPathIsProtected = (path: string) => protectedMediaRoots.has(path.split('/').filter(Boolean)[0] ?? '')

export function Files({ desktop, udid, onToast }: { desktop: boolean; udid: string; onToast: (message: string) => void }) {
  const runTask = useDeviceTask(desktop, onToast)
  const demoSharingApps: FileSharingApp[] = installedApps.filter((app) => !app.system).slice(0, 3).map((app) => ({ bundleId: app.bundle, name: app.name }))
  const [source, setSource] = useState<FileSource>('media')
  const [path, setPath] = useState<string[]>([])
  const [remoteEntries, setRemoteEntries] = useState<RemoteFileEntry[]>([])
  const [remoteSharingApps, setRemoteSharingApps] = useState<FileSharingApp[]>([])
  const sharingApps = desktop ? remoteSharingApps : demoSharingApps
  const [selectedBundle, setSelectedBundle] = useState(desktop ? '' : demoSharingApps[0]?.bundleId ?? '')
  const [selected, setSelected] = useState<RemoteFileEntry | null>(null)
  const [namingFolder, setNamingFolder] = useState(false)
  const [transfer, setTransfer] = useState<OperationProgress | null>(null)
  const currentPath = `/${path.join('/')}`
  const selectedApp = sharingApps.find((app) => app.bundleId === selectedBundle)
  const bundleId = source === 'app' ? selectedBundle || undefined : undefined
  const sourceAvailable = source === 'media' || Boolean(bundleId)
  const currentReadOnly = source === 'media' && mediaPathIsProtected(currentPath)
  const selectedReadOnly = source === 'media' && Boolean(selected && mediaPathIsProtected(selected.path))

  useEffect(() => {
    if (!desktop) return
    let unlisten: (() => void) | undefined
    void events.transferProgress(setTransfer).then((stop) => { unlisten = stop })
    return () => unlisten?.()
  }, [desktop])

  useEffect(() => {
    if (!desktop) return
    void api.fileSharingApps(udid).then((apps) => {
      setRemoteSharingApps(apps)
      setSelectedBundle((current) => apps.some((app) => app.bundleId === current) ? current : apps[0]?.bundleId ?? '')
    }).catch((error) => onToast(errorMessage(error)))
  }, [desktop, udid, onToast])

  const refresh = useCallback(async () => {
    if (!desktop) return
    if (!sourceAvailable) {
      setRemoteEntries([])
      setSelected(null)
      return
    }
    try {
      setRemoteEntries(await api.afcList(currentPath, udid, bundleId))
      setSelected(null)
    } catch (error) {
      setRemoteEntries([])
      onToast(errorMessage(error))
    }
  }, [desktop, sourceAvailable, currentPath, udid, bundleId, onToast])
  useEffect(() => { void refresh() }, [refresh])

  const entries: RemoteFileEntry[] = desktop
    ? remoteEntries
    : source === 'app'
      ? currentPath === '/' ? demoAppDocumentEntries : []
      : (fileSystem[currentPath] ?? []).map((entry) => ({
        name: entry.name, path: `${currentPath === '/' ? '' : currentPath}/${entry.name}`, kind: entry.folder ? 'Folder' : entry.kind ?? 'Document',
        isDirectory: Boolean(entry.folder), size: displaySizeToBytes(entry.size), modified: entry.date,
      }))

  const selectSource = (nextSource: FileSource) => {
    setSource(nextSource)
    setPath([])
    setSelected(null)
  }
  const selectApp = (nextBundle: string) => {
    setSelectedBundle(nextBundle)
    setPath([])
    setSelected(null)
  }
  const remoteChild = (name: string) => `${currentPath === '/' ? '' : currentPath}/${name}`

  const uploadFile = () => runTask(async () => {
    if (!sourceAvailable || currentReadOnly) return
    const chosen = await dialogs.anyFile()
    if (!chosen || Array.isArray(chosen)) return
    const name = chosen.split(/[\\/]/).pop() ?? 'upload.bin'
    try {
      await api.afcUpload(chosen, remoteChild(name), udid, bundleId)
      onToast(`${name} uploaded`)
    } finally {
      setTransfer(null)
    }
    await refresh()
  }, 'File operations are available in the desktop app')
  const downloadFile = () => runTask(async () => {
    if (!selected || selected.isDirectory) return
    const target = await dialogs.saveFile(selected.name)
    if (!target) return
    try {
      await api.afcDownload(selected.path, target, udid, bundleId)
      onToast(`${selected.name} downloaded`)
    } finally {
      setTransfer(null)
    }
  })
  const cancelTransfer = () => { void api.afcTransferCancel() }
  const openFolderPrompt = () => {
    if (!sourceAvailable || currentReadOnly) return
    setNamingFolder(true)
  }
  const createFolder = (name: string) => {
    setNamingFolder(false)
    void runTask(async () => {
      await api.afcMkdir(remoteChild(name), udid, bundleId)
      onToast(`${name} created`)
      await refresh()
    }, 'Folder creation is available in the desktop app')
  }
  const removeEntry = () => runTask(async () => {
    if (!selected || selectedReadOnly) return
    const entry = selected
    const confirmed = await dialogs.confirmDestructive(
      entry.isDirectory
        ? `Delete the folder “${entry.name}” and everything inside it? This cannot be undone.`
        : `Delete “${entry.name}”? This cannot be undone.`,
      'Delete',
    )
    if (!confirmed) return
    await api.afcRemove(entry.path, entry.isDirectory, udid, bundleId)
    onToast(`${entry.name} removed`)
    await refresh()
  })

  const rootLabel = source === 'media' ? 'iPhone Media' : 'Documents'
  const displayedPath = source === 'media'
    ? `/var/mobile/Media${currentPath === '/' ? '' : currentPath}`
    : `${selectedApp?.name ?? 'App'} sandbox/Documents${currentPath === '/' ? '' : currentPath}`

  return (
    <section className="files-page page-padding compact-padding">
      <div className="file-source-bar">
        <div className="file-source-tabs" role="tablist" aria-label="iPhone file source">
          <button role="tab" aria-selected={source === 'media'} className={source === 'media' ? 'active' : ''} onClick={() => selectSource('media')}><HardDrive size={15} />Device Media</button>
          <button role="tab" aria-selected={source === 'app'} className={source === 'app' ? 'active' : ''} onClick={() => selectSource('app')}><AppWindow size={15} />App Documents</button>
        </div>
        {source === 'app' && <label className="file-app-select"><span>File-sharing app</span><select value={selectedBundle} onChange={(event) => selectApp(event.target.value)} disabled={!sharingApps.length}>{sharingApps.length ? sharingApps.map((app) => <option key={app.bundleId} value={app.bundleId}>{app.name} · {app.bundleId}</option>) : <option value="">No compatible apps</option>}</select></label>}
        <div className="file-source-copy"><b>{source === 'media' ? 'AFC media storage' : 'Shared app documents'}</b><small>{source === 'media' ? 'Maps to /var/mobile/Media; this is not the iOS system root.' : 'Only apps that enable iOS File Sharing appear here.'}</small></div>
      </div>
      <div className="breadcrumbs"><span>{source === 'media' ? 'com.apple.afc' : 'house_arrest'}</span><ChevronRight size={13} /><button onClick={() => setPath([])}>{rootLabel}</button>{path.map((part, index) => <span className="crumb-pair" key={`${part}-${index}`}><ChevronRight size={13} /><button onClick={() => setPath(path.slice(0, index + 1))}>{part}</button></span>)}{currentReadOnly && <em className="read-only-badge">Read only</em>}</div>
      <div className="file-actions"><button className="primary-button" onClick={() => void uploadFile()} disabled={!sourceAvailable || currentReadOnly}><Upload size={14} />Upload</button><button onClick={() => void downloadFile()} disabled={!selected || selected.isDirectory}>Download</button><button onClick={openFolderPrompt} disabled={!sourceAvailable || currentReadOnly}><Plus size={14} />New folder</button><button className="danger-action" onClick={() => void removeEntry()} disabled={!selected || selectedReadOnly}>Delete</button></div>
      <div className="file-table card">
        <div className="file-head"><span>Name</span><span>Kind</span><span>Modified</span><span>Size</span></div>
        {!sourceAvailable ? <div className="empty-card">No installed apps currently expose Documents through iOS File Sharing.</div> : !entries.length ? <div className="empty-card">This folder is empty.</div> : entries.map((entry) => {
          const description = source === 'media' && currentPath === '/' ? mediaFolderDescriptions[entry.name] : ''
          const readOnlyEntry = source === 'media' && mediaPathIsProtected(entry.path)
          return <button key={entry.name} className={selected?.path === entry.path ? 'selected' : ''} onClick={() => setSelected(entry)} onDoubleClick={() => entry.isDirectory && setPath([...path, entry.name])}><span>{entry.isDirectory ? <Folder size={17} /> : <File size={17} />}<span className="file-name-copy"><b>{entry.name}</b>{description && <small>{description}</small>}</span></span><small>{readOnlyEntry ? `${entry.kind} · read only` : entry.kind}</small><small>{entry.modified}</small><small>{entry.isDirectory ? '—' : bytes(entry.size)}</small></button>
        })}
      </div>
      {transfer && (
        <div className="transfer-bar" role="status">
          <div className="transfer-copy">
            <b>{transfer.operation === 'upload' ? 'Uploading' : 'Downloading'} {transfer.item}</b>
            <small>{transfer.percent}%</small>
          </div>
          <div className="progress"><span style={{ width: `${transfer.percent}%` }} /></div>
          <button onClick={cancelTransfer}>Cancel</button>
        </div>
      )}
      <div className="table-footer"><span>{entries.length} items</span><span>{displayedPath}</span></div>
      {namingFolder && (
        <PromptModal
          title="New folder"
          description={`Created in ${displayedPath}`}
          label="Folder name"
          placeholder="Exports"
          validate={(name) => {
            if (name === '.' || name === '..') return 'That name is reserved by the file system'
            if (/[\\/]/.test(name)) return 'A folder name cannot contain a slash'
            if (entries.some((entry) => entry.name === name)) return 'Something with that name already exists here'
            return undefined
          }}
          onSubmit={createFolder}
          onClose={() => setNamingFolder(false)}
        />
      )}
    </section>
  )
}
