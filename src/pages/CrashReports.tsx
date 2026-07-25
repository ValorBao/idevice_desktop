import { useCallback, useEffect, useMemo, useState } from 'react'
import { Bug, Download, RefreshCw, Search, X } from 'lucide-react'
import { api, dialogs, errorMessage, type CrashReportContent, type CrashReportSummary } from '../api'
import { bytes } from '../lib/format'

const demoCrashReports: CrashReportSummary[] = [
  { name: 'StikDebug-2026-07-23-184205.ips', path: '/StikDebug-2026-07-23-184205.ips', kind: 'IPS', process: 'StikDebug', sizeBytes: 184326, modified: '2026-07-23 18:42:05' },
  { name: 'JetsamEvent-2026-07-22-091814.ips', path: '/JetsamEvent-2026-07-22-091814.ips', kind: 'IPS', process: 'JetsamEvent', sizeBytes: 42812, modified: '2026-07-22 09:18:14' },
  { name: 'CrossCode-2026-07-20-221037.crash', path: '/Retired/CrossCode-2026-07-20-221037.crash', kind: 'CRASH', process: 'CrossCode', sizeBytes: 96340, modified: '2026-07-20 22:10:37' },
]

const demoCrashContent = `{"app_name":"StikDebug","timestamp":"2026-07-23 18:42:05.00 +0800","app_version":"1.4.2","slice_uuid":"4A9B1E5C-97B2-4D19-97F2-7BD6404C470A","build_version":"142","platform":2,"bundleID":"com.stik.debug","share_with_app_devs":1,"is_first_party":0,"bug_type":"309","os_version":"iPhone OS 17.5.1 (21F90)","incident_id":"A72B817D-8250-42A1-AC3B-9566A69D2F90"}
{
  "uptime" : 42000,
  "procRole" : "Foreground",
  "version" : 2,
  "userID" : 501,
  "modelCode" : "iPhone16,1",
  "coalitionName" : "com.stik.debug",
  "captureTime" : "2026-07-23 18:42:05.128 +0800",
  "exception" : {
    "type" : "EXC_BAD_ACCESS",
    "signal" : "SIGSEGV",
    "subtype" : "KERN_INVALID_ADDRESS at 0x0000000000000010"
  },
  "termination" : {
    "namespace" : "SIGNAL",
    "code" : 11,
    "indicator" : "Segmentation fault: 11"
  },
  "faultingThread" : 0,
  "threads" : [
    {
      "triggered" : true,
      "name" : "com.apple.main-thread",
      "frames" : [
        { "imageOffset" : 42824, "symbol" : "DebugSession.start()", "symbolLocation" : 184 },
        { "imageOffset" : 21760, "symbol" : "AppDelegate.application(_:didFinishLaunchingWithOptions:)", "symbolLocation" : 92 }
      ]
    }
  ]
}`

export function CrashReports({ desktop, udid, onToast }: { desktop: boolean; udid: string; onToast: (message: string) => void }) {
  const [reports, setReports] = useState<CrashReportSummary[]>(desktop ? [] : demoCrashReports)
  const [selectedPath, setSelectedPath] = useState(desktop ? '' : demoCrashReports[0].path)
  const [content, setContent] = useState<CrashReportContent | null>(desktop ? null : {
    path: demoCrashReports[0].path,
    content: demoCrashContent,
    truncated: false,
    sizeBytes: demoCrashReports[0].sizeBytes ?? 0,
  })
  const [query, setQuery] = useState('')
  const [kind, setKind] = useState<'all' | 'ips' | 'crash'>('all')
  const [loading, setLoading] = useState(false)
  const selected = reports.find((report) => report.path === selectedPath)

  const loadReports = useCallback(async () => {
    if (!desktop) return
    setLoading(true)
    try {
      const loaded = await api.crashReportsList(udid)
      setReports(loaded)
      setSelectedPath((current) => loaded.some((report) => report.path === current) ? current : loaded[0]?.path ?? '')
    } catch (error) {
      onToast(errorMessage(error))
    } finally {
      setLoading(false)
    }
  }, [desktop, udid, onToast])

  useEffect(() => { void loadReports() }, [loadReports])
  useEffect(() => {
    if (!desktop || !selectedPath) {
      if (!selectedPath) setContent(null)
      return
    }
    let disposed = false
    setContent(null)
    void api.crashReportRead(selectedPath, udid)
      .then((result) => {
        if (disposed) return
        setContent(result)
        setReports((items) => items.map((report) => report.path === result.path ? { ...report, sizeBytes: result.sizeBytes } : report))
      })
      .catch((error) => { if (!disposed) onToast(errorMessage(error)) })
    return () => { disposed = true }
  }, [desktop, selectedPath, udid, onToast])

  const filtered = useMemo(() => reports.filter((report) => {
    const kindMatches = kind === 'all' || report.kind.toLowerCase() === kind
    const queryMatches = `${report.name} ${report.process} ${report.path}`.toLowerCase().includes(query.toLowerCase())
    return kindMatches && queryMatches
  }), [reports, kind, query])
  useEffect(() => {
    if (!filtered.some((report) => report.path === selectedPath)) {
      setSelectedPath(filtered[0]?.path ?? '')
    }
  }, [filtered, selectedPath])

  const exportSelected = async () => {
    if (!selected) return
    if (!desktop) {
      const blob = new Blob([content?.content ?? demoCrashContent], { type: 'text/plain;charset=utf-8' })
      const url = URL.createObjectURL(blob)
      const anchor = document.createElement('a')
      anchor.href = url
      anchor.download = selected.name
      anchor.click()
      URL.revokeObjectURL(url)
      onToast(`${selected.name} exported`)
      return
    }
    try {
      const target = await dialogs.saveFile(selected.name)
      if (!target) return
      await api.crashReportExport(selected.path, target, udid)
      onToast(`${selected.name} exported`)
    } catch (error) {
      onToast(errorMessage(error))
    }
  }

  return (
    <section className="crash-page page-padding compact-padding">
      <div className="crash-toolbar">
        <label><Search size={15} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search reports by app, filename, or path…" />{query && <button onClick={() => setQuery('')} aria-label="Clear report filter"><X size={14} /></button>}</label>
        <div className="crash-kind-filter">{(['all', 'ips', 'crash'] as const).map((item) => <button key={item} className={kind === item ? 'active' : ''} onClick={() => setKind(item)}>{item === 'all' ? 'All' : `.${item}`}</button>)}</div>
        <button className="crash-refresh" onClick={() => void loadReports()} disabled={loading}><RefreshCw className={loading ? 'spinning' : ''} size={14} />Refresh</button>
      </div>
      <div className="crash-workspace">
        <div className="crash-list card">
          <header><span>{filtered.length} reports</span><small>Newest first</small></header>
          <div>
            {filtered.map((report) => <button key={report.path} className={selectedPath === report.path ? 'active' : ''} onClick={() => setSelectedPath(report.path)}><span className="crash-kind"><Bug size={15} /></span><span><b>{report.process}</b><small>{report.name}</small><time>{report.modified}</time></span><code>{report.kind}</code><em>{bytes(report.sizeBytes)}</em></button>)}
            {!filtered.length && <div className="crash-empty">{loading ? 'Loading crash reports…' : reports.length ? 'No reports match this filter.' : 'No crash reports were returned by the device.'}</div>}
          </div>
        </div>
        <div className="crash-preview card">
          {selected ? <>
            <header><div><b>{selected.name}</b><small>{selected.path}</small></div><button className="primary-button" onClick={() => void exportSelected()} disabled={!content}><Download size={14} />Export</button></header>
            <div className="crash-summary"><span><small>Process</small><b>{selected.process}</b></span><span><small>Type</small><b>{selected.kind}</b></span><span><small>Modified</small><b>{selected.modified || '—'}</b></span><span><small>Size</small><b>{bytes(content?.sizeBytes ?? selected.sizeBytes)}</b></span></div>
            {content ? <><pre>{content.content}</pre>{content.truncated && <footer>Preview limited to 4 MB. Export saves the complete report.</footer>}</> : <div className="crash-empty">Loading report contents…</div>}
          </> : <div className="crash-empty">Select a crash report to preview its contents.</div>}
        </div>
      </div>
    </section>
  )
}
