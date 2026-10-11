import { useEffect, useMemo, useRef, useState } from 'react'
import {
  AlertTriangle, BadgeCheck, CheckCircle2, FileArchive, FileBadge, HardDriveDownload,
  KeyRound, LoaderCircle, PackageCheck, ShieldCheck, Smartphone,
} from 'lucide-react'
import {
  api, dialogs, errorMessage, events, type OperationProgress, type PersonalSigningPreflight,
  type PersonalSigningResult,
} from '../api'
import { fileName, packageSize } from '../lib/format'
import { PersonalAccountSigning } from './PersonalAccountSigning'

const demoPreflight: PersonalSigningPreflight = {
  ipaName: 'FieldNotes.ipa',
  appName: 'Field Notes',
  bundleId: 'com.example.fieldnotes',
  version: '1.4.0',
  profileName: 'iOS Team Provisioning Profile: *',
  profileUuid: 'DEMO-PROFILE-UUID',
  teamIdentifier: 'TEAM123456',
  applicationIdentifier: 'TEAM123456.*',
  expiresAt: '2027-08-30T00:00:00Z',
  deviceCount: 2,
  deviceIncluded: true,
  identities: [{
    hash: '0123456789ABCDEF0123456789ABCDEF01234567',
    name: 'Apple Development: Local Developer (TEAM123456)',
    teamIdentifier: 'TEAM123456',
    matchesProfile: true,
  }],
  selectedIdentityHash: '0123456789ABCDEF0123456789ABCDEF01234567',
  ready: true,
  blockers: [],
  warnings: ['Wildcard entitlements will be specialized to this app bundle identifier.'],
}


export function PersonalSigning({ desktop, udid, deviceName = 'iPhone', onToast }: { desktop: boolean; udid: string; deviceName?: string; onToast: (message: string) => void }) {
  const [mode, setMode] = useState<'account' | 'local'>('account')
  const [ipaPath, setIpaPath] = useState('')
  const [profilePath, setProfilePath] = useState('')
  const [preflight, setPreflight] = useState<PersonalSigningPreflight | null>(null)
  const [identityHash, setIdentityHash] = useState('')
  const [checking, setChecking] = useState(false)
  const [signing, setSigning] = useState(false)
  const [installing, setInstalling] = useState(false)
  const [progress, setProgress] = useState<OperationProgress | null>(null)
  const [failure, setFailure] = useState('')
  const [result, setResult] = useState<PersonalSigningResult | null>(null)
  const requestGeneration = useRef(0)

  useEffect(() => {
    if (!desktop) return
    let signingStop: (() => void) | undefined
    let installStop: (() => void) | undefined
    void events.personalSigningProgress(setProgress).then((stop) => { signingStop = stop })
    void events.appProgress((event) => {
      if (event.operation === 'install') setProgress(event)
    }).then((stop) => { installStop = stop })
    return () => { signingStop?.(); installStop?.() }
  }, [desktop])

  useEffect(() => {
    const generation = ++requestGeneration.current
    setPreflight(null)
    setIdentityHash('')
    setResult(null)
    setFailure('')
    if (!ipaPath || !profilePath) return
    setChecking(true)
    const check = desktop
      ? api.personalSigningPreflight(ipaPath, profilePath, udid)
      : Promise.resolve(demoPreflight)
    void check.then((next) => {
      if (generation !== requestGeneration.current) return
      setPreflight(next)
      setIdentityHash(next.selectedIdentityHash ?? next.identities.find((identity) => identity.matchesProfile)?.hash ?? '')
    }).catch((error) => {
      if (generation !== requestGeneration.current) return
      const message = errorMessage(error)
      setFailure(message)
      onToast(message)
    }).finally(() => {
      if (generation === requestGeneration.current) setChecking(false)
    })
  }, [desktop, ipaPath, onToast, profilePath, udid])

  const matchingIdentities = useMemo(
    () => preflight?.identities.filter((identity) => identity.matchesProfile) ?? [],
    [preflight],
  )

  const chooseIpa = async () => {
    const selected = desktop ? await dialogs.ipa() : '/Users/demo/Downloads/FieldNotes.ipa'
    if (typeof selected === 'string') setIpaPath(selected)
  }

  const chooseProfile = async () => {
    const selected = desktop
      ? await dialogs.provisioningProfile()
      : '/Users/demo/Downloads/Development.mobileprovision'
    if (typeof selected === 'string') setProfilePath(selected)
  }

  const signAndExport = async () => {
    if (!preflight?.ready || !identityHash || signing) return
    const baseName = preflight.ipaName.replace(/\.ipa$/i, '')
    const outputPath = desktop
      ? await dialogs.signedIpaDestination(`${baseName}-personal-signed.ipa`)
      : `/Users/demo/Downloads/${baseName}-personal-signed.ipa`
    if (typeof outputPath !== 'string') return
    setSigning(true)
    setResult(null)
    setFailure('')
    setProgress({ operation: 'personal-signing', item: 'Preparing signing workspace', percent: 0 })
    try {
      const signed = desktop
        ? await api.personalSigningExport({ ipaPath, profilePath, identityHash, outputPath }, udid)
        : {
            outputPath,
            appName: preflight.appName,
            bundleId: preflight.bundleId,
            profileName: preflight.profileName,
            identityName: matchingIdentities[0]?.name ?? 'Demo identity',
            sizeBytes: 18_400_000,
          }
      setResult(signed)
      setProgress({ operation: 'personal-signing', item: 'Signed IPA ready', percent: 100 })
      onToast(`${signed.appName} signed successfully`)
    } catch (error) {
      const message = errorMessage(error)
      setFailure(message)
      setProgress(null)
      onToast(message)
    } finally {
      setSigning(false)
    }
  }

  const installSigned = async () => {
    if (!result || installing) return
    if (!desktop) {
      onToast(`${result.appName} installed`)
      return
    }
    setInstalling(true)
    setProgress({ operation: 'install', item: result.appName, percent: 0 })
    try {
      await api.appInstall(result.outputPath, udid)
      onToast(`${result.appName} installed`)
      setProgress({ operation: 'install', item: 'Installed on device', percent: 100 })
    } catch (error) {
      const message = errorMessage(error)
      setFailure(message)
      onToast(message)
    } finally {
      setInstalling(false)
    }
  }

  return (
    <section className="personal-signing-page page-padding compact-padding">
      <div className="signing-mode-tabs" role="tablist" aria-label="Signing method">
        <button role="tab" aria-selected={mode === 'account'} className={mode === 'account' ? 'active' : ''} onClick={() => setMode('account')}><KeyRound size={14} />Apple Account</button>
        <button role="tab" aria-selected={mode === 'local'} className={mode === 'local' ? 'active' : ''} onClick={() => setMode('local')}><ShieldCheck size={14} />Local profile</button>
      </div>
      {mode === 'account' ? <PersonalAccountSigning desktop={desktop} udid={udid} deviceName={deviceName} onToast={onToast} /> : <>
      <div className="signing-intro card">
        <span><ShieldCheck size={22} /></span>
        <div><h2>Sign locally, keep credentials local</h2><p>The assistant uses an Apple code-signing identity already stored in macOS Keychain. It never asks for or saves your Apple ID password.</p></div>
        <code>LOCAL ONLY</code>
      </div>

      <div className="signing-layout">
        <div className="signing-setup card">
          <header><div><small>STEP 1</small><h3>Choose signing inputs</h3></div><FileArchive size={20} /></header>
          <button className={ipaPath ? 'selected' : ''} onClick={() => void chooseIpa()}>
            <span><FileArchive size={18} /></span><div><b>{ipaPath ? fileName(ipaPath) : 'Choose an IPA'}</b><small>{ipaPath || 'The source file is never modified'}</small></div><em>{ipaPath ? 'Change' : 'Choose'}</em>
          </button>
          <button className={profilePath ? 'selected' : ''} onClick={() => void chooseProfile()}>
            <span><FileBadge size={18} /></span><div><b>{profilePath ? fileName(profilePath) : 'Choose a provisioning profile'}</b><small>{profilePath || '.mobileprovision or .provisionprofile'}</small></div><em>{profilePath ? 'Change' : 'Choose'}</em>
          </button>

          <div className="signing-identity">
            <label htmlFor="signing-identity"><small>STEP 2</small><b>Matching Keychain identity</b></label>
            <select id="signing-identity" value={identityHash} onChange={(event) => setIdentityHash(event.target.value)} disabled={!matchingIdentities.length || checking}>
              {!matchingIdentities.length && <option value="">No matching identity</option>}
              {matchingIdentities.map((identity) => <option key={identity.hash} value={identity.hash}>{identity.name}</option>)}
            </select>
            <p><KeyRound size={13} />Only identities whose certificate is embedded in the selected profile can be used.</p>
          </div>

          {(signing || installing) && progress && <div className="signing-progress" role="status"><div><b>{progress.item}</b><code>{progress.percent}%</code></div><span><i style={{ width: `${progress.percent}%` }} /></span></div>}

          <button className="signing-export primary-button" onClick={() => void signAndExport()} disabled={!preflight?.ready || !identityHash || signing || installing}>
            {signing ? <LoaderCircle className="spinning" size={16} /> : <HardDriveDownload size={16} />}
            {signing ? 'Signing…' : 'Sign and export IPA'}
          </button>
          <p className="signing-limit"><AlertTriangle size={13} />First release supports a single iOS app bundle. App extensions and Watch apps are intentionally blocked.</p>
        </div>

        <aside className="signing-preflight card">
          <header><div><small>PRE-FLIGHT</small><h3>Compatibility check</h3></div>{checking ? <LoaderCircle className="spinning" /> : preflight?.ready ? <CheckCircle2 className="ready" /> : <BadgeCheck />}</header>
          {!ipaPath || !profilePath ? <div className="signing-empty"><PackageCheck size={30} /><b>Two files are required</b><p>Choose the IPA and provisioning profile to check the bundle, device, certificate, and expiry.</p></div>
            : checking ? <div className="signing-empty" role="status"><LoaderCircle className="spinning" size={28} /><b>Checking locally…</b><p>No file contents leave this Mac.</p></div>
              : failure ? <div className="signing-failure" role="alert"><AlertTriangle size={18} /><div><b>Pre-flight failed</b><p>{failure}</p></div></div>
                : preflight && <>
                  <div className={`signing-verdict ${preflight.ready ? 'ready' : 'blocked'}`}><span>{preflight.ready ? <CheckCircle2 size={17} /> : <AlertTriangle size={17} />}</span><div><b>{preflight.ready ? 'Ready to sign' : 'Action required'}</b><p>{preflight.ready ? 'All required local checks passed.' : `${preflight.blockers.length} blocking check${preflight.blockers.length === 1 ? '' : 's'} found.`}</p></div></div>
                  <dl>
                    <div><dt>Application</dt><dd>{preflight.appName} <small>v{preflight.version}</small></dd></div>
                    <div><dt>Bundle ID</dt><dd><code>{preflight.bundleId}</code></dd></div>
                    <div><dt>Profile</dt><dd>{preflight.profileName}</dd></div>
                    <div><dt>Team</dt><dd><code>{preflight.teamIdentifier ?? 'Unknown'}</code></dd></div>
                    <div><dt>Expires</dt><dd>{preflight.expiresAt ? new Date(preflight.expiresAt).toLocaleDateString() : 'Unknown'}</dd></div>
                    <div><dt>Current device</dt><dd className={preflight.deviceIncluded ? 'good' : 'bad'}>{preflight.deviceIncluded ? `Included · ${preflight.deviceCount} total` : 'Not included'}</dd></div>
                  </dl>
                  {!!preflight.blockers.length && <div className="signing-messages blocked">{preflight.blockers.map((message) => <p key={message}><AlertTriangle size={13} />{message}</p>)}</div>}
                  {!!preflight.warnings.length && <div className="signing-messages warning">{preflight.warnings.map((message) => <p key={message}><AlertTriangle size={13} />{message}</p>)}</div>}
                </>}
        </aside>
      </div>

      {result && <div className="signing-result card">
        <span><PackageCheck size={22} /></span>
        <div><small>SIGNED IPA</small><b>{fileName(result.outputPath)}</b><p>{result.appName} · {result.bundleId} · {packageSize(result.sizeBytes)}</p></div>
        <button className="primary-button" onClick={() => void installSigned()} disabled={installing || signing}>{installing ? <LoaderCircle className="spinning" size={15} /> : <Smartphone size={15} />}{installing ? 'Installing…' : 'Install on device'}</button>
      </div>}
      </>}
    </section>
  )
}
