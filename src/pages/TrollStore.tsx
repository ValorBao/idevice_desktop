import { useCallback, useEffect, useRef, useState } from 'react'
import { Package, RefreshCw, Upload } from 'lucide-react'
import { api, dialogs, errorMessage, events, type TrollStoreStatus } from '../api'
import { useDeviceTask } from '../lib/hooks'

const demoStatus = (): TrollStoreStatus => ({
  productVersion: '17.0',
  buildVersion: '21A329',
  helperSupported: true,
  helperDetail: 'iOS 17.0 (21A329) is inside the helper-install window.',
  helperCaution: null,
  trollstoreInstalled: true,
  removableApps: [{ bundleId: 'com.apple.tips', name: 'Tips', bundleName: 'Tips.app' }],
  ipaInstallAvailable: true,
  ipaInstallDetail: 'Demonstration only. No device is contacted, and no IPA is served.',
})

export function TrollStore({ desktop, udid, onToast }: { desktop: boolean; udid: string; onToast: (message: string) => void }) {
  const [status, setStatus] = useState<TrollStoreStatus | null>(desktop ? null : demoStatus())
  const [loading, setLoading] = useState(desktop)
  const [busy, setBusy] = useState('')
  const [failure, setFailure] = useState('')
  const [selectedBundle, setSelectedBundle] = useState('com.apple.tips')
  const requestRef = useRef(0)
  const runTask = useDeviceTask(desktop, onToast)

  useEffect(() => () => { requestRef.current += 1 }, [])

  const refresh = useCallback(async () => {
    if (!desktop) {
      setStatus(demoStatus())
      setFailure('')
      setLoading(false)
      return
    }
    const request = ++requestRef.current
    setLoading(true)
    setFailure('')
    try {
      const next = await api.trollstoreStatus(udid)
      if (request !== requestRef.current) return
      setStatus(next)
      setSelectedBundle((current) => next.removableApps.some((app) => app.bundleId === current)
        ? current
        : next.removableApps.find((app) => app.bundleId === 'com.apple.tips')?.bundleId ?? next.removableApps[0]?.bundleId ?? '')
    } catch (error) {
      if (request !== requestRef.current) return
      const message = errorMessage(error)
      setFailure(message)
      onToast(message)
    } finally {
      if (request === requestRef.current) setLoading(false)
    }
  }, [desktop, onToast, udid])

  useEffect(() => { void refresh() }, [refresh])

  useEffect(() => {
    if (!desktop) return
    let unlisten: (() => void) | undefined
    let disposed = false
    void events.trollstoreProgress((progress) => setBusy(progress.item))
      .then((stop) => { if (disposed) stop(); else unlisten = stop })
    return () => { disposed = true; unlisten?.() }
  }, [desktop])

  const selected = status?.removableApps.find((app) => app.bundleId === selectedBundle) ?? status?.removableApps[0]
  const helperLabel = !status ? 'Checking' : status.helperSupported ? 'In range' : 'Refused'
  const installedLabel = !status ? 'Checking' : status.trollstoreInstalled ? 'Installed' : 'Not installed'

  const replaceHelper = () => runTask(async () => {
    if (!selected || !status?.helperSupported) return
    const confirmed = await dialogs.confirmDestructive(
      `Replacing ${selected.name} on this device reboots it and leaves that app replaced. The only way back is to delete it and download it again from the App Store. Find My has to be off. A restore the device has already accepted cannot be cancelled. iOS ${status.productVersion} (${status.buildVersion}).`,
      `Replace ${selected.name}`,
    )
    if (!confirmed) return
    setBusy('Starting the helper restore')
    try {
      const result = await api.trollstoreHelperInstall(selected.bundleId, udid)
      onToast(result.message)
      await refresh()
    } finally {
      setBusy('')
    }
  }, 'Replacing a system app with the TrollStore helper is available in the desktop app')

  const installIpa = () => runTask(async () => {
    const picked = await dialogs.ipa()
    if (typeof picked !== 'string' || !picked) return
    const name = picked.split(/[/\\]/).pop() ?? picked
    const confirmed = await dialogs.confirmDestructive(
      `TrollStore will be asked to download ${name} from this Mac. Turn on URL Scheme in TrollStore settings first. The phone still asks you to confirm the install, and this Mac cannot see that result.`,
      'Send to TrollStore',
    )
    if (!confirmed) return
    setBusy('Offering the IPA to TrollStore')
    try {
      const result = await api.trollstoreIpaInstall(picked, udid)
      onToast(result.message)
    } finally {
      setBusy('')
    }
  }, 'Sending an IPA to TrollStore is available in the desktop app')

  return (
    <section className="trollstore-page">
      <div className="trollstore-toolbar">
        <button disabled={loading || Boolean(busy)} onClick={() => void refresh()}><RefreshCw size={14} className={loading ? 'spinning' : ''} />Refresh</button>
        {status?.helperSupported && selected && (
          <button disabled={Boolean(busy)} onClick={replaceHelper}><Package size={14} />Replace {selected.name}</button>
        )}
        {status?.ipaInstallAvailable && (
          <button className="primary-button" disabled={Boolean(busy)} onClick={installIpa}><Upload size={14} />Install IPA</button>
        )}
      </div>

      {failure && <div className="process-error" role="status">{failure}</div>}
      {busy && <div className="process-notice" role="status">{busy}</div>}

      <div className="trollstore-summary">
        <div className="card">
          <small>Device</small>
          <b>{status ? `iOS ${status.productVersion}` : '…'}</b>
          <span>{status ? `Build ${status.buildVersion || 'unavailable'}` : 'Reading the paired device'}</span>
        </div>
        <div className={`card ${status?.helperSupported ? 'allowed' : status ? 'refused' : ''}`}>
          <small>Helper install</small>
          <b>{helperLabel}</b>
          <span>{status?.helperDetail ?? 'The version check has not finished.'}</span>
        </div>
        <div className="card">
          <small>TrollStore</small>
          <b>{installedLabel}</b>
          <span>{status?.trollstoreInstalled ? 'com.opa334.TrollStore is on this device.' : 'com.opa334.TrollStore was not found.'}</span>
        </div>
      </div>

      {status?.helperCaution && <div className="process-notice" role="status">{status.helperCaution}</div>}

      <div className="card trollstore-note">
        <b>Both actions wait for confirmation</b>
        <p>{status?.ipaInstallDetail ?? 'Install from the Mac is not available until TrollStore is on the device.'} Replacing an app uses a USB backup restore and reboots the phone.</p>
      </div>

      <div className="card trollstore-apps">
        <b>Removable system apps</b>
        {status && status.removableApps.length > 0 ? (
          <ul>
            {status.removableApps.map((app) => (
              <li key={app.bundleId} className={app.bundleId === selected?.bundleId ? 'selected' : undefined}>
                <button type="button" aria-pressed={app.bundleId === selected?.bundleId} onClick={() => setSelectedBundle(app.bundleId)}>{app.name}</button>
                <code>{app.bundleName}</code>
              </li>
            ))}
          </ul>
        ) : (
          <p className="trollstore-empty">{loading ? 'Looking for removable system apps.' : 'No removable system app was found. Tips is the usual choice once it is installed from the App Store.'}</p>
        )}
      </div>
    </section>
  )
}
