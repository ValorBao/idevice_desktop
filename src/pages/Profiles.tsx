import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { AlertTriangle, Clock3, RefreshCw, Search, ShieldCheck } from 'lucide-react'
import { api, errorMessage, type ProvisioningProfileSnapshot, type ProvisioningProfileSummary } from '../api'

const dateFromNow = (days: number) => new Date(Date.now() + days * 86_400_000).toISOString()

const demoSnapshot = (): ProvisioningProfileSnapshot => ({
  transport: 'Misagent · demonstration',
  totalCount: 4,
  truncated: false,
  profiles: [
    {
      id: 'E926A0D1-DEMO-4D65-AE8F-1D2A88579001', uuid: 'E926A0D1-DEMO-4D65-AE8F-1D2A88579001', name: 'Device Lab Development',
      teamName: 'Example Development Team', teamIdentifier: '7XAMPLE123', applicationIdentifier: '7XAMPLE123.com.example.devicelab',
      createdAt: dateFromNow(-120), expiresAt: dateFromNow(244), daysRemaining: 244, expirationState: 'valid', profileType: 'development',
      platforms: ['iOS'], deviceCount: 5, provisionsAllDevices: false, getTaskAllow: true, sizeBytes: 14_820, parseError: null,
    },
    {
      id: '11D23E7A-DEMO-4982-B669-A6197C79A002', uuid: '11D23E7A-DEMO-4982-B669-A6197C79A002', name: 'Beta Ad Hoc',
      teamName: 'Example Development Team', teamIdentifier: '7XAMPLE123', applicationIdentifier: '7XAMPLE123.com.example.beta',
      createdAt: dateFromNow(-330), expiresAt: dateFromNow(18), daysRemaining: 18, expirationState: 'expiring', profileType: 'ad-hoc',
      platforms: ['iOS'], deviceCount: 27, provisionsAllDevices: false, getTaskAllow: false, sizeBytes: 18_402, parseError: null,
    },
    {
      id: '8C70CA14-DEMO-4B63-9D6F-B3BD30BAA003', uuid: '8C70CA14-DEMO-4B63-9D6F-B3BD30BAA003', name: 'App Store Distribution',
      teamName: 'Example Development Team', teamIdentifier: '7XAMPLE123', applicationIdentifier: '7XAMPLE123.com.example.release',
      createdAt: dateFromNow(-42), expiresAt: dateFromNow(323), daysRemaining: 323, expirationState: 'valid', profileType: 'app-store',
      platforms: ['iOS'], deviceCount: 0, provisionsAllDevices: false, getTaskAllow: false, sizeBytes: 12_925, parseError: null,
    },
    {
      id: 'F3F5F77C-DEMO-445D-B091-AC6F0000A004', uuid: 'F3F5F77C-DEMO-445D-B091-AC6F0000A004', name: 'Old Internal Distribution',
      teamName: 'Example Enterprise', teamIdentifier: 'ENTERPRISE1', applicationIdentifier: 'ENTERPRISE1.com.example.internal',
      createdAt: dateFromNow(-410), expiresAt: dateFromNow(-45), daysRemaining: -45, expirationState: 'expired', profileType: 'enterprise',
      platforms: ['iOS'], deviceCount: 0, provisionsAllDevices: true, getTaskAllow: false, sizeBytes: 11_630, parseError: null,
    },
  ],
})

const typeLabel = (value: string) => ({
  development: 'Development',
  'ad-hoc': 'Ad Hoc',
  'app-store': 'App Store',
  enterprise: 'Enterprise',
  unknown: 'Unknown',
}[value] ?? value)

const dateLabel = (value: string | null) => {
  if (!value) return 'Unavailable'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium' }).format(date)
}

const bytes = (value: number) => value < 1024
  ? `${value} B`
  : `${(value / 1024).toFixed(value < 10 * 1024 ? 1 : 0)} KB`

const expiryLabel = (profile: ProvisioningProfileSummary) => {
  if (profile.daysRemaining === null) return 'Expiry unavailable'
  if (profile.daysRemaining < 0) return `Expired ${Math.abs(profile.daysRemaining)} days ago`
  if (profile.daysRemaining === 0) return 'Expires today'
  return `${profile.daysRemaining} days remaining`
}

type ProfileFilter = 'all' | 'attention' | 'development' | 'distribution'

export function Profiles({ desktop, udid, onToast }: { desktop: boolean; udid: string; onToast: (message: string) => void }) {
  const [snapshot, setSnapshot] = useState<ProvisioningProfileSnapshot | null>(null)
  const [selectedId, setSelectedId] = useState('')
  const [query, setQuery] = useState('')
  const [filter, setFilter] = useState<ProfileFilter>('all')
  const [loading, setLoading] = useState(true)
  const [failure, setFailure] = useState('')
  const requestRef = useRef(0)

  useEffect(() => () => { requestRef.current += 1 }, [])

  const refresh = useCallback(async () => {
    const request = ++requestRef.current
    setLoading(true)
    setFailure('')
    try {
      const next = desktop ? await api.provisioningProfiles(udid) : demoSnapshot()
      if (request !== requestRef.current) return
      setSnapshot(next)
      setSelectedId((current) => next.profiles.some((profile) => profile.id === current) ? current : next.profiles[0]?.id ?? '')
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

  const profiles = snapshot?.profiles ?? []
  const shown = useMemo(() => {
    const needle = query.trim().toLowerCase()
    return profiles.filter((profile) => {
      const matchesQuery = !needle || [
        profile.name,
        profile.uuid,
        profile.teamName,
        profile.teamIdentifier,
        profile.applicationIdentifier,
      ].some((value) => value?.toLowerCase().includes(needle))
      const matchesFilter = filter === 'all'
        || (filter === 'attention' && ['expired', 'expiring', 'unknown'].includes(profile.expirationState))
        || (filter === 'development' && profile.profileType === 'development')
        || (filter === 'distribution' && ['ad-hoc', 'app-store', 'enterprise'].includes(profile.profileType))
      return matchesQuery && matchesFilter
    })
  }, [filter, profiles, query])
  const selected = shown.find((profile) => profile.id === selectedId) ?? shown[0] ?? null
  const counts = useMemo(() => ({
    valid: profiles.filter((profile) => profile.expirationState === 'valid').length,
    expiring: profiles.filter((profile) => profile.expirationState === 'expiring').length,
    expired: profiles.filter((profile) => profile.expirationState === 'expired').length,
  }), [profiles])

  return (
    <section className="profiles-page">
      <div className="profiles-toolbar">
        <label><Search size={14} /><input aria-label="Search profiles" value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Name, team, UUID, or application identifier" /></label>
        <select aria-label="Filter profiles" value={filter} onChange={(event) => setFilter(event.target.value as ProfileFilter)}>
          <option value="all">All profiles</option>
          <option value="attention">Needs attention</option>
          <option value="development">Development</option>
          <option value="distribution">Distribution</option>
        </select>
        <button disabled={loading} onClick={() => void refresh()}><RefreshCw size={14} className={loading ? 'spinning' : ''} />Refresh</button>
        <small>{snapshot?.transport ?? 'Misagent'}</small>
      </div>

      {failure && <div className="process-error" role="status">{failure}</div>}
      {snapshot?.truncated && <div className="process-notice" role="status">Showing the first {profiles.length} of {snapshot.totalCount.toLocaleString()} profiles.</div>}

      <div className="profiles-summary">
        <div className="card"><small>Installed</small><b>{snapshot?.totalCount ?? 0}</b><span>provisioning profiles</span></div>
        <div className="card valid"><small>Valid</small><b>{counts.valid}</b><span>more than 30 days</span></div>
        <div className="card expiring"><small>Expiring</small><b>{counts.expiring}</b><span>within 30 days</span></div>
        <div className="card expired"><small>Expired</small><b>{counts.expired}</b><span>should be replaced</span></div>
      </div>

      <div className="profiles-layout">
        <div className="card profiles-list">
          <header><span>Profile</span><span>Type</span><span>Expiry</span></header>
          <div>
            {loading && !snapshot && <div className="profiles-loading"><RefreshCw size={20} />Reading device profiles…</div>}
            {!loading && !failure && !shown.length && <div className="profiles-empty">No provisioning profiles match this view.</div>}
            {shown.map((profile) => (
              <button key={profile.id} className={selected?.id === profile.id ? 'selected' : ''} onClick={() => setSelectedId(profile.id)}>
                <span><b>{profile.name}</b><small>{profile.teamName ?? profile.teamIdentifier ?? profile.uuid ?? 'Metadata unavailable'}</small></span>
                <code>{typeLabel(profile.profileType)}</code>
                <em className={profile.expirationState}>{profile.parseError ? 'Unreadable' : expiryLabel(profile)}</em>
              </button>
            ))}
          </div>
          <footer><span>{shown.length} shown</span><span>Read only</span></footer>
        </div>

        <aside className="card profiles-detail">
          {selected ? <>
            <header>
              <span className={selected.expirationState}>{selected.expirationState === 'valid' ? <ShieldCheck size={16} /> : <AlertTriangle size={16} />}</span>
              <div><small>Provisioning profile</small><h3>{selected.name}</h3><p>{expiryLabel(selected)}</p></div>
            </header>
            {selected.parseError && <div className="profiles-parse-error" role="status"><b>Metadata could not be read</b><span>{selected.parseError}</span></div>}
            <dl>
              <div><dt>Type</dt><dd>{typeLabel(selected.profileType)}</dd></div>
              <div><dt>Platform</dt><dd>{selected.platforms.join(', ') || 'Unavailable'}</dd></div>
              <div><dt>Created</dt><dd>{dateLabel(selected.createdAt)}</dd></div>
              <div><dt>Expires</dt><dd>{dateLabel(selected.expiresAt)}</dd></div>
              <div><dt>Team</dt><dd>{selected.teamName ?? 'Unavailable'}</dd></div>
              <div><dt>Team identifier</dt><dd><code>{selected.teamIdentifier ?? 'Unavailable'}</code></dd></div>
              <div className="wide"><dt>Application identifier</dt><dd><code>{selected.applicationIdentifier ?? 'Unavailable'}</code></dd></div>
              <div className="wide"><dt>UUID</dt><dd><code>{selected.uuid ?? 'Unavailable'}</code></dd></div>
              <div><dt>Registered devices</dt><dd>{selected.provisionsAllDevices ? 'All devices' : selected.deviceCount.toLocaleString()}</dd></div>
              <div><dt>Debug entitlement</dt><dd>{selected.getTaskAllow === null ? 'Unavailable' : selected.getTaskAllow ? 'Allowed' : 'Not allowed'}</dd></div>
              <div><dt>Signed size</dt><dd>{bytes(selected.sizeBytes)}</dd></div>
              <div><dt>Status</dt><dd className={selected.expirationState}><Clock3 size={12} />{selected.expirationState}</dd></div>
            </dl>
            <div className="profiles-readonly"><b>Read-only inspection</b><p>Metadata is extracted from profiles returned by the device. Installation, removal, and cryptographic signer-chain verification are outside this first slice.</p></div>
          </> : <div className="profiles-detail-empty"><ShieldCheck size={28} /><b>Select a profile</b><p>Its signing scope and expiry details will appear here.</p></div>}
        </aside>
      </div>
    </section>
  )
}
