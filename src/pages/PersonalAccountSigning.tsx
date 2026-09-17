import { createSessionId } from '../lib/session'
import { useEffect, useRef, useState } from 'react'
import {
  AlertTriangle, BadgeCheck, CheckCircle2, FileArchive, KeyRound, LoaderCircle,
  LockKeyhole, LogOut, PackageCheck, ShieldCheck, Smartphone,
} from 'lucide-react'
import {
  api, dialogs, errorMessage, events, type OperationProgress,
  type PersonalAccountSigningResult, type PersonalAccountStatus,
  type PersonalAccountTwoFactor,
} from '../api'
import { fileName, packageSize } from '../lib/format'

const demoStatus: PersonalAccountStatus = {
  loggedIn: false,
  email: null,
  teamId: null,
  teamName: null,
  anisetteServer: 'https://ani.sidestore.io',
  credentialStorage: 'macOS Keychain',
  passwordStored: false,
}


export function PersonalAccountSigning({
  desktop,
  udid,
  deviceName,
  onToast,
}: {
  desktop: boolean
  udid: string
  deviceName: string
  onToast: (message: string) => void
}) {
  const [status, setStatus] = useState<PersonalAccountStatus>(demoStatus)
  const [loadingStatus, setLoadingStatus] = useState(desktop)
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [loggingIn, setLoggingIn] = useState(false)
  const [twoFactor, setTwoFactor] = useState<PersonalAccountTwoFactor | null>(null)
  const [verificationCode, setVerificationCode] = useState('')
  const [submittingCode, setSubmittingCode] = useState(false)
  const [ipaPath, setIpaPath] = useState('')
  const [signing, setSigning] = useState(false)
  const [installing, setInstalling] = useState(false)
  const [progress, setProgress] = useState<OperationProgress | null>(null)
  const [failure, setFailure] = useState('')
  const [result, setResult] = useState<PersonalAccountSigningResult | null>(null)
  const mounted = useRef(true)
  const lifecycle = useRef(0)
  const signingOperation = useRef<string | null>(null)
  const [cancelling, setCancelling] = useState(false)

  useEffect(() => {
    mounted.current = true
    let disposed = false
    lifecycle.current += 1
    if (!desktop) return () => { mounted.current = false }
    void api.personalAccountStatus()
      .then((next) => { if (!disposed) setStatus(next) })
      .catch((error) => { if (!disposed) setFailure(errorMessage(error)) })
      .finally(() => { if (!disposed) setLoadingStatus(false) })

    let twoFactorStop: (() => void) | undefined
    let progressStop: (() => void) | undefined
    void events.personalAccountTwoFactor((request) => {
      if (disposed) return
      setVerificationCode('')
      setTwoFactor(request)
    }).then((stop) => {
      if (disposed) stop()
      else twoFactorStop = stop
    }).catch((error) => { if (!disposed) setFailure(errorMessage(error)) })
    void events.personalAccountProgress((next) => {
      if (!disposed && next.operation === signingOperation.current) setProgress(next)
    }).then((stop) => {
      if (disposed) stop()
      else progressStop = stop
    }).catch((error) => { if (!disposed) setFailure(errorMessage(error)) })
    return () => {
      disposed = true
      mounted.current = false
      lifecycle.current += 1
      const operationId = signingOperation.current
      signingOperation.current = null
      if (operationId) void api.personalAccountSignCancel(operationId).catch(() => {})
      twoFactorStop?.()
      progressStop?.()
    }
  }, [desktop])

  useEffect(() => {
    setResult(null)
    setProgress(null)
    setSigning(false)
    setCancelling(false)
    return () => {
      lifecycle.current += 1
      const operationId = signingOperation.current
      signingOperation.current = null
      if (desktop && operationId) void api.personalAccountSignCancel(operationId).catch(() => {})
    }
  }, [desktop, udid])

  const login = async (event: React.FormEvent) => {
    event.preventDefault()
    if (!email.trim() || !password || loggingIn) return
    let submittedPassword = password
    setPassword('')
    setLoggingIn(true)
    setFailure('')
    try {
      const next = desktop
        ? await api.personalAccountLogin(email.trim(), submittedPassword)
        : { ...demoStatus, loggedIn: true, email: email.trim(), teamId: 'DEMO123456', teamName: 'Personal Team' }
      setStatus(next)
      setTwoFactor(null)
      onToast(`Signed in as ${next.email}`)
    } catch (error) {
      const message = errorMessage(error)
      setFailure(message)
      onToast(message)
    } finally {
      submittedPassword = ''
      setLoggingIn(false)
    }
  }

  const submitTwoFactor = async (event: React.FormEvent) => {
    event.preventDefault()
    if (!/^\d{6}$/.test(verificationCode) || submittingCode) return
    setSubmittingCode(true)
    try {
      if (desktop) await api.personalAccountSubmitTwoFactor(verificationCode)
      setTwoFactor(null)
      setVerificationCode('')
    } catch (error) {
      const message = errorMessage(error)
      setFailure(message)
      onToast(message)
    } finally {
      setSubmittingCode(false)
    }
  }

  const cancelTwoFactor = async () => {
    try {
      if (desktop) await api.personalAccountCancelTwoFactor()
    } finally {
      setTwoFactor(null)
      setVerificationCode('')
    }
  }

  const signOut = async () => {
    try {
      if (desktop) await api.personalAccountSignOut()
      lifecycle.current += 1
      signingOperation.current = null
      setSigning(false)
      setCancelling(false)
      setStatus(demoStatus)
      setIpaPath('')
      setResult(null)
      setProgress(null)
      setFailure('')
      onToast('Apple Account signed out')
    } catch (error) {
      setFailure(errorMessage(error))
    }
  }

  const cancelSigning = async () => {
    const operationId = signingOperation.current
    if (!operationId || cancelling) return
    setCancelling(true)
    try {
      if (desktop) await api.personalAccountSignCancel(operationId)
    } catch (error) {
      if (signingOperation.current === operationId) {
        setCancelling(false)
        setFailure(errorMessage(error))
      }
    }
  }

  const chooseIpa = async () => {
    const selected = desktop ? await dialogs.ipa() : '/Users/demo/Downloads/Example.ipa'
    if (typeof selected === 'string') {
      setIpaPath(selected)
      setResult(null)
      setFailure('')
    }
  }

  const signAndExport = async () => {
    if (!status.loggedIn || !ipaPath || signingOperation.current) return
    const operationId = createSessionId()
    const generation = lifecycle.current
    signingOperation.current = operationId
    const current = () => mounted.current && lifecycle.current === generation && signingOperation.current === operationId
    let submitted = false
    setSigning(true)
    setCancelling(false)
    setResult(null)
    setFailure('')
    setProgress(null)
    try {
      const baseName = fileName(ipaPath).replace(/\.ipa$/i, '')
      const outputPath = desktop
        ? await dialogs.signedIpaDestination(`${baseName}-apple-account-signed.ipa`)
        : `/Users/demo/Downloads/${baseName}-apple-account-signed.ipa`
      if (typeof outputPath !== 'string' || !current()) return
      setProgress({ operation: operationId, item: 'Preparing Apple account signing', percent: 0 })
      submitted = true
      const signed = desktop
        ? await api.personalAccountSignExport({ operationId, ipaPath, outputPath, udid, deviceName })
        : {
            outputPath,
            appName: 'Example',
            bundleId: 'com.example.demo.DEMO123456',
            version: '1.0',
            teamId: status.teamId ?? 'DEMO123456',
            teamName: status.teamName ?? 'Personal Team',
            sizeBytes: 12_000_000,
          }
      if (!current()) return
      setResult(signed)
      setProgress({ operation: operationId, item: 'Signed IPA ready', percent: 100 })
      onToast(`${signed.appName} signed with your Apple Account`)
    } catch (error) {
      if (!current()) return
      const message = errorMessage(error)
      setFailure(message)
      setProgress(null)
      onToast(message)
    } finally {
      if (current()) {
        signingOperation.current = null
        setSigning(false)
        setCancelling(false)
      } else if (desktop && submitted) {
        await api.personalAccountSignCancel(operationId).catch(() => {})
      }
    }
  }

  const installSigned = async () => {
    if (!result || installing) return
    setInstalling(true)
    try {
      if (desktop) await api.appInstall(result.outputPath, udid)
      onToast(`${result.appName} installed`)
    } catch (error) {
      const message = errorMessage(error)
      setFailure(message)
      onToast(message)
    } finally {
      setInstalling(false)
    }
  }

  return (
    <>
      <div className="account-signing-notice card">
        <span><ShieldCheck size={22} /></span>
        <div>
          <h2>Sign with your own Apple Account</h2>
          <p>Compatible with iLoader's open-source signing approach. Your password is used for this login only and is not saved; certificate material and Anisette state stay in macOS Keychain.</p>
        </div>
        <code>ISIDELOAD</code>
      </div>

      <div className="account-signing-layout">
        <section className="account-login-card card">
          <header>
            <div><small>APPLE ACCOUNT</small><h3>{status.loggedIn ? 'Account ready' : 'Sign in'}</h3></div>
            {status.loggedIn ? <CheckCircle2 className="ready" size={22} /> : <KeyRound size={21} />}
          </header>
          {loadingStatus ? <div className="account-loading" role="status"><LoaderCircle className="spinning" />Checking local session…</div>
            : status.loggedIn ? <div className="account-ready">
              <div className="account-avatar"><BadgeCheck size={22} /></div>
              <div><b>{status.email}</b><p>{status.teamName} · <code>{status.teamId}</code></p></div>
              <button onClick={() => void signOut()} aria-label="Sign out of Apple Account"><LogOut size={15} />Sign out</button>
            </div>
              : <form className="account-login-form" onSubmit={(event) => void login(event)}>
                <label htmlFor="apple-account-email">Apple Account</label>
                <input id="apple-account-email" type="email" autoComplete="username" value={email} onChange={(event) => setEmail(event.target.value)} placeholder="name@example.com" disabled={loggingIn} />
                <label htmlFor="apple-account-password">Password</label>
                <input id="apple-account-password" type="password" autoComplete="current-password" value={password} onChange={(event) => setPassword(event.target.value)} placeholder="Apple Account password" disabled={loggingIn} />
                <button className="primary-button" type="submit" disabled={!email.trim() || !password || loggingIn}>
                  {loggingIn ? <LoaderCircle className="spinning" size={15} /> : <LockKeyhole size={15} />}
                  {loggingIn ? 'Signing in…' : 'Sign in securely'}
                </button>
                <p><LockKeyhole size={12} />Password saving is disabled. The field is cleared as soon as login starts.</p>
              </form>}

          <div className="account-service-note">
            <AlertTriangle size={14} />
            <p>This is an unofficial compatibility flow using Apple private developer endpoints. Authentication contacts Apple and the community Anisette service <code>{status.anisetteServer}</code>; Apple may change or block the flow.</p>
          </div>
        </section>

        <section className="account-sign-card card">
          <header><div><small>SIGN &amp; EXPORT</small><h3>Create a personal signed IPA</h3></div><FileArchive size={21} /></header>
          <button className={`account-ipa-picker ${ipaPath ? 'selected' : ''}`} onClick={() => void chooseIpa()} disabled={!status.loggedIn || signing}>
            <span><FileArchive size={19} /></span>
            <div><b>{ipaPath ? fileName(ipaPath) : 'Choose an IPA'}</b><small>{ipaPath || (status.loggedIn ? 'The source file is never overwritten' : 'Sign in first')}</small></div>
            <em>{ipaPath ? 'Change' : 'Choose'}</em>
          </button>
          <ol className="account-sign-steps">
            <li className={status.loggedIn ? 'done' : ''}><span>1</span><div><b>Authenticate</b><small>Apple Account + 2FA</small></div></li>
            <li className={ipaPath ? 'done' : ''}><span>2</span><div><b>Prepare app</b><small>Register device and App ID</small></div></li>
            <li className={result ? 'done' : ''}><span>3</span><div><b>Sign IPA</b><small>Certificate and profile from your team</small></div></li>
          </ol>
          {progress && (signing || result) && <div className="signing-progress" role="status"><div><b>{progress.item}</b><code>{progress.percent}%</code></div><span><i style={{ width: `${progress.percent}%` }} /></span></div>}
          <button className="primary-button account-sign-action" onClick={() => void signAndExport()} disabled={!status.loggedIn || !ipaPath || signing || installing}>
            {signing ? <LoaderCircle className="spinning" size={16} /> : <PackageCheck size={16} />}
            {cancelling ? 'Cancelling…' : signing ? 'Signing with Apple…' : 'Sign and export IPA'}
          </button>
          {signing && <button onClick={() => void cancelSigning()} disabled={cancelling}>Cancel signing</button>}
          {cancelling && <p role="status">Waiting for the current step to stop. Registrations already completed with Apple are retained.</p>}
        </section>
      </div>

      {failure && <div className="account-signing-error card" role="alert"><AlertTriangle size={17} /><div><b>Action needed</b><p>{failure}</p></div></div>}

      {result && <div className="signing-result card">
        <span><PackageCheck size={22} /></span>
        <div><small>APPLE ACCOUNT SIGNED IPA</small><b>{fileName(result.outputPath)}</b><p>{result.appName} · {result.bundleId} · {packageSize(result.sizeBytes)}</p></div>
        <button className="primary-button" onClick={() => void installSigned()} disabled={installing || signing}>{installing ? <LoaderCircle className="spinning" size={15} /> : <Smartphone size={15} />}{installing ? 'Installing…' : 'Install on device'}</button>
      </div>}

      {twoFactor && <div className="account-2fa-backdrop">
        <section className="account-2fa-dialog card" role="dialog" aria-modal="true" aria-labelledby="account-2fa-title">
          <span><LockKeyhole size={23} /></span>
          <h3 id="account-2fa-title">Two-factor authentication</h3>
          <p>Enter the 6-digit code sent to your trusted Apple device{twoFactor.sms ? ' or phone number' : ''}.</p>
          {twoFactor.lastError && <div className="account-2fa-error">{twoFactor.lastError}</div>}
          <form onSubmit={(event) => void submitTwoFactor(event)}>
            <input aria-label="6-digit verification code" inputMode="numeric" autoComplete="one-time-code" maxLength={6} value={verificationCode} onChange={(event) => setVerificationCode(event.target.value.replace(/\D/g, '').slice(0, 6))} autoFocus />
            <div><button type="button" onClick={() => void cancelTwoFactor()}>Cancel</button><button className="primary-button" type="submit" disabled={!/^\d{6}$/.test(verificationCode) || submittingCode}>{submittingCode ? 'Submitting…' : 'Verify'}</button></div>
          </form>
        </section>
      </div>}
    </>
  )
}
