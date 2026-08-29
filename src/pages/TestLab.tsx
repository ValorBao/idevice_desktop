import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { AlertTriangle, Beaker, Bot, CheckCircle2, CircleDashed, LockKeyhole, RefreshCw, Search, ShieldCheck, Smartphone } from 'lucide-react'
import {
  api,
  errorMessage,
  type DeveloperStatus,
  type XCTestPlanRequest,
  type XCTestPreflightSnapshot,
  type XCTestRunPlan,
  type XCTestRunnerCandidate,
} from '../api'

const demoSnapshot = (): XCTestPreflightSnapshot => ({
  iosVersion: '17.4',
  transport: 'CoreDeviceProxy/RSD TestManager/DVT · demonstration',
  executionSupported: true,
  limitation: null,
  runnerTotal: 2,
  targetTotal: 3,
  truncated: false,
  runners: [
    {
      bundleId: 'com.example.WebDriverAgentRunner.xctrunner',
      name: 'WebDriverAgentRunner',
      version: '6.1.0',
      executable: 'WebDriverAgentRunner-Runner',
      debuggable: true,
      isWebdriverAgent: true,
      configurationReady: true,
      issues: [],
    },
    {
      bundleId: 'com.example.DeviceLabUITests.xctrunner',
      name: 'DeviceLabUITests',
      version: '1.4',
      executable: 'DeviceLabUITests-Runner',
      debuggable: false,
      isWebdriverAgent: false,
      configurationReady: true,
      issues: ['get-task-allow is not enabled'],
    },
  ],
  targets: [
    { bundleId: 'com.example.DeviceLab', name: 'Device Lab', version: '1.4', debuggable: true },
    { bundleId: 'com.example.Storefront', name: 'Storefront', version: '3.2', debuggable: false },
    { bundleId: 'com.example.SettingsSample', name: 'Settings Sample', version: '2.0', debuggable: true },
  ],
})

const demoStatus = (): DeveloperStatus => ({
  developerMode: true,
  ddiMounted: true,
  ddiImages: null,
  rsdAvailable: true,
})

const testIdentifiers = (value: string) => [...new Set(value
  .split(/\r?\n/)
  .map((identifier) => identifier.trim())
  .filter(Boolean))]

type CheckState = 'pass' | 'warn' | 'fail'

function ReadinessCheck({ state, label, detail }: { state: CheckState; label: string; detail: string }) {
  return (
    <li className={state}>
      <span>{state === 'pass' ? <CheckCircle2 size={14} /> : state === 'warn' ? <CircleDashed size={14} /> : <AlertTriangle size={14} />}</span>
      <div><b>{label}</b><small>{detail}</small></div>
    </li>
  )
}

export function TestLab({ desktop, udid, onToast }: { desktop: boolean; udid: string; onToast: (message: string) => void }) {
  const [snapshot, setSnapshot] = useState<XCTestPreflightSnapshot | null>(null)
  const [developer, setDeveloper] = useState<DeveloperStatus | null>(null)
  const [selectedRunnerId, setSelectedRunnerId] = useState('')
  const [selectedTargetId, setSelectedTargetId] = useState('')
  const [query, setQuery] = useState('')
  const [loading, setLoading] = useState(true)
  const [failure, setFailure] = useState('')
  const [planMode, setPlanMode] = useState<'test' | 'wda'>('test')
  const [timeoutSeconds, setTimeoutSeconds] = useState(900)
  const [testsToRun, setTestsToRun] = useState('')
  const [testsToSkip, setTestsToSkip] = useState('')
  const [plan, setPlan] = useState<XCTestRunPlan | null>(null)
  const [planBusy, setPlanBusy] = useState(false)
  const [planFailure, setPlanFailure] = useState('')
  const requestRef = useRef(0)
  const planRequestRef = useRef(0)

  useEffect(() => () => {
    requestRef.current += 1
    planRequestRef.current += 1
  }, [])

  const refresh = useCallback(async () => {
    const request = ++requestRef.current
    planRequestRef.current += 1
    setLoading(true)
    setFailure('')
    setPlan(null)
    setPlanBusy(false)
    setPlanFailure('')
    try {
      const [next, nextDeveloper] = desktop
        ? await Promise.all([api.xctestPreflight(udid), api.developerStatus(udid)])
        : [demoSnapshot(), demoStatus()]
      if (request !== requestRef.current) return
      setSnapshot(next)
      setDeveloper(nextDeveloper)
      setSelectedRunnerId((current) => next.runners.some((runner) => runner.bundleId === current)
        ? current
        : next.runners[0]?.bundleId ?? '')
      setSelectedTargetId((current) => next.targets.some((target) => target.bundleId === current) ? current : '')
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

  const runners = snapshot?.runners ?? []
  const shownRunners = useMemo(() => {
    const needle = query.trim().toLowerCase()
    if (!needle) return runners
    return runners.filter((runner) => [runner.name, runner.bundleId, runner.executable]
      .some((value) => value?.toLowerCase().includes(needle)))
  }, [query, runners])
  const selectedRunner = shownRunners.find((runner) => runner.bundleId === selectedRunnerId) ?? shownRunners[0] ?? null
  const selectedTarget = snapshot?.targets.find((target) => target.bundleId === selectedTargetId) ?? null
  const readyRunnerCount = runners.filter((runner) => runner.configurationReady && runner.debuggable).length
  const iosMajor = Number.parseInt(snapshot?.iosVersion.split('.')[0] ?? '0', 10)
  const iosSupported = iosMajor >= 11
  const developerModeRequired = iosMajor >= 16
  const developerModeReady = !developerModeRequired || developer?.developerMode === true
  const ddiReady = developer?.ddiMounted === true
  const serviceReady = snapshot?.executionSupported === true
    && (iosMajor < 17 || developer?.rsdAvailable === true)
  const preflightReady = Boolean(
    iosSupported
    && developerModeReady
    && ddiReady
    && serviceReady
    && selectedRunner?.configurationReady
    && selectedRunner.debuggable,
  )

  useEffect(() => {
    planRequestRef.current += 1
    setPlan(null)
    setPlanBusy(false)
    setPlanFailure('')
  }, [planMode, selectedRunner?.bundleId, selectedTargetId, testsToRun, testsToSkip, timeoutSeconds, udid])

  useEffect(() => {
    if (planMode === 'wda' && selectedRunner && !selectedRunner.isWebdriverAgent) {
      setPlanMode('test')
      setTimeoutSeconds(900)
    }
  }, [planMode, selectedRunner])

  const changePlanMode = (mode: 'test' | 'wda') => {
    setPlanMode(mode)
    if (mode === 'wda') {
      setSelectedTargetId('')
      setTestsToRun('')
      setTestsToSkip('')
      setTimeoutSeconds(30)
    } else {
      setTimeoutSeconds(900)
    }
  }

  const validatePlan = async () => {
    if (!selectedRunner || !preflightReady) return
    const request: XCTestPlanRequest = {
      runnerBundleId: selectedRunner.bundleId,
      targetBundleId: planMode === 'test' ? selectedTargetId || null : null,
      mode: planMode,
      testsToRun: planMode === 'test' ? testIdentifiers(testsToRun) : [],
      testsToSkip: planMode === 'test' ? testIdentifiers(testsToSkip) : [],
      timeoutSeconds,
    }
    const validation = ++planRequestRef.current
    setPlanBusy(true)
    setPlanFailure('')
    try {
      const next = desktop
        ? await api.xctestPlanPrepare(request, udid)
        : {
          runnerBundleId: selectedRunner.bundleId,
          runnerName: selectedRunner.name,
          targetBundleId: request.targetBundleId,
          targetName: selectedTarget?.name ?? null,
          mode: request.mode,
          testsToRun: request.testsToRun,
          testsToSkip: request.testsToSkip,
          timeoutSeconds: request.timeoutSeconds,
          wdaBridge: request.mode === 'wda',
          transport: snapshot?.transport ?? 'Demonstration',
        }
      if (validation !== planRequestRef.current) return
      setPlan(next)
    } catch (error) {
      if (validation !== planRequestRef.current) return
      const message = errorMessage(error)
      setPlanFailure(message)
      onToast(message)
    } finally {
      if (validation === planRequestRef.current) setPlanBusy(false)
    }
  }

  const runnerState = (runner: XCTestRunnerCandidate) => runner.configurationReady && runner.debuggable
    ? 'ready'
    : runner.configurationReady ? 'warning' : 'blocked'

  return (
    <section className="test-lab-page">
      <div className="test-lab-toolbar">
        <label><Search size={14} /><input aria-label="Search XCTest runners" value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Runner name, bundle ID, or executable" /></label>
        <button disabled={loading} onClick={() => void refresh()}><RefreshCw className={loading ? 'spinning' : ''} size={14} />Refresh</button>
        <small>{snapshot?.transport ?? 'Installation Proxy + XCTest preflight'}</small>
      </div>

      {failure && <div className="process-error" role="alert">{failure}</div>}
      {snapshot?.truncated && <div className="process-notice" role="status">The device returned more candidates than this bounded view can display.</div>}

      <div className="test-lab-summary">
        <div className="card"><small>Installed runners</small><b>{snapshot?.runnerTotal ?? 0}</b><span>{readyRunnerCount} metadata-ready</span></div>
        <div className="card"><small>Target apps</small><b>{snapshot?.targetTotal ?? 0}</b><span>optional selection</span></div>
        <div className="card"><small>Device system</small><b>{snapshot ? `iOS ${snapshot.iosVersion}` : '—'}</b><span>{iosSupported ? 'XCTest generation supported' : 'requires iOS 11+'}</span></div>
        <div className={`card ${preflightReady ? 'ready' : 'attention'}`}><small>Selected preflight</small><b>{preflightReady ? 'Ready' : 'Attention'}</b><span>{selectedRunner ? selectedRunner.name : 'choose an installed runner'}</span></div>
      </div>

      <div className="test-lab-layout">
        <div className="card test-lab-runners">
          <header><span>Runner</span><span>Kind</span><span>Readiness</span></header>
          <div>
            {loading && !snapshot && <div className="test-lab-loading"><RefreshCw size={20} />Inspecting installed applications…</div>}
            {!loading && !failure && !shownRunners.length && <div className="test-lab-empty"><Beaker size={26} /><b>No XCTest runners found</b><p>Install a signed .xctrunner application, then refresh this preflight.</p></div>}
            {shownRunners.map((runner) => (
              <button key={runner.bundleId} className={selectedRunner?.bundleId === runner.bundleId ? 'selected' : ''} onClick={() => setSelectedRunnerId(runner.bundleId)}>
                <span><b>{runner.name}</b><small>{runner.bundleId}</small></span>
                <code>{runner.isWebdriverAgent ? 'WDA' : 'XCTest'}</code>
                <em className={runnerState(runner)}>{runnerState(runner)}</em>
              </button>
            ))}
          </div>
          <footer><span>{shownRunners.length} shown</span><span>Read-only metadata</span></footer>
        </div>

        <aside className="card test-lab-detail">
          {selectedRunner ? <>
            <header>
              <span className={selectedRunner.isWebdriverAgent ? 'wda' : ''}>{selectedRunner.isWebdriverAgent ? <Bot size={17} /> : <Beaker size={17} />}</span>
              <div><small>Selected test runner</small><h3>{selectedRunner.name}</h3><p>{selectedRunner.bundleId}</p></div>
              <em className={preflightReady ? 'ready' : 'attention'}>{preflightReady ? 'Preflight ready' : 'Needs attention'}</em>
            </header>

            <dl>
              <div><dt>Version</dt><dd>{selectedRunner.version || 'Unavailable'}</dd></div>
              <div><dt>Runner kind</dt><dd>{selectedRunner.isWebdriverAgent ? 'WebDriverAgent' : 'XCTest'}</dd></div>
              <div className="wide"><dt>Executable</dt><dd><code>{selectedRunner.executable ?? 'Unavailable'}</code></dd></div>
            </dl>

            <label className="test-lab-target">
              <span>Optional target application</span>
              <select aria-label="Target application" disabled={planMode === 'wda'} value={selectedTargetId} onChange={(event) => setSelectedTargetId(event.target.value)}>
                <option value="">No target application</option>
                {snapshot?.targets.map((target) => <option key={target.bundleId} value={target.bundleId}>{target.name} · {target.bundleId}</option>)}
              </select>
              <small>{selectedTarget ? `${selectedTarget.bundleId}${selectedTarget.debuggable ? ' · debug entitlement allowed' : ''}` : 'The runner can execute without launching a separate target app.'}</small>
            </label>

            <ul className="test-lab-checks" aria-label="XCTest readiness checks">
              <ReadinessCheck state={iosSupported ? 'pass' : 'fail'} label="Supported iOS generation" detail={snapshot ? `iOS ${snapshot.iosVersion}` : 'Waiting for device metadata'} />
              <ReadinessCheck
                state={developerModeReady ? 'pass' : developer?.developerMode === null ? 'warn' : 'fail'}
                label="Developer Mode"
                detail={developerModeRequired ? developer?.developerMode === true ? 'Enabled' : developer?.developerMode === false ? 'Disabled on device' : 'Status unavailable' : 'Not required before iOS 16'}
              />
              <ReadinessCheck state={ddiReady ? 'pass' : 'fail'} label="Developer Disk Image" detail={ddiReady ? 'Mounted' : 'Mount a matching image in Debug Tools'} />
              <ReadinessCheck state={serviceReady ? 'pass' : 'fail'} label="TestManager and DVT route" detail={snapshot?.limitation ?? snapshot?.transport ?? 'Waiting for route inspection'} />
              <ReadinessCheck state={selectedRunner.configurationReady ? 'pass' : 'fail'} label="Runner application metadata" detail={selectedRunner.configurationReady ? 'Path, container, and -Runner executable are present' : selectedRunner.issues.filter((issue) => !issue.includes('get-task-allow')).join(' · ')} />
              <ReadinessCheck state={selectedRunner.debuggable ? 'pass' : 'fail'} label="Runner debug entitlement" detail={selectedRunner.debuggable ? 'get-task-allow is enabled' : 'Re-sign the runner with a development profile'} />
              <ReadinessCheck state="pass" label="Target selection" detail={planMode === 'wda' ? 'Not used by WebDriverAgent bridge plans' : selectedTarget ? `${selectedTarget.name} selected` : 'Optional; no target application selected'} />
            </ul>

            <div className="test-lab-plan">
              <header><div><small>Non-executing configuration</small><b>Execution plan</b></div><em>{plan ? 'Validated' : 'Draft'}</em></header>
              <div className="test-lab-plan-fields">
                <label><span>Mode</span><select aria-label="Plan mode" value={planMode} onChange={(event) => changePlanMode(event.target.value as 'test' | 'wda')}>
                  <option value="test">Standard XCTest</option>
                  <option value="wda" disabled={!selectedRunner.isWebdriverAgent}>WebDriverAgent bridge</option>
                </select></label>
                <label><span>{planMode === 'wda' ? 'Readiness timeout' : 'Wall-clock timeout'}</span><select aria-label="Plan timeout" value={timeoutSeconds} onChange={(event) => setTimeoutSeconds(Number(event.target.value))}>
                  {(planMode === 'wda' ? [15, 30, 60, 120] : [300, 900, 1800, 3600]).map((seconds) => <option key={seconds} value={seconds}>{seconds < 60 ? `${seconds} seconds` : `${seconds / 60} minutes`}</option>)}
                </select></label>
                <label className="wide"><span>Tests to include · one per line</span><textarea aria-label="Tests to include" disabled={planMode === 'wda'} value={testsToRun} onChange={(event) => setTestsToRun(event.target.value)} placeholder="SuiteName/testMethod" /></label>
                <label className="wide"><span>Tests to skip · one per line</span><textarea aria-label="Tests to skip" disabled={planMode === 'wda'} value={testsToSkip} onChange={(event) => setTestsToSkip(event.target.value)} placeholder="SuiteName/testMethod" /></label>
              </div>
              <button className="test-lab-validate" disabled={planBusy || !preflightReady || (planMode === 'wda' && !selectedRunner.isWebdriverAgent)} onClick={() => void validatePlan()}>
                {planBusy ? <RefreshCw className="spinning" size={14} /> : <ShieldCheck size={14} />}{planBusy ? 'Validating…' : 'Validate run plan'}
              </button>
              {planFailure && <div className="process-error" role="alert">{planFailure}</div>}
              {plan && <div className="test-lab-plan-preview" role="status">
                <header><CheckCircle2 size={15} /><div><b>Validated execution plan</b><small>Still read-only; no process was launched</small></div></header>
                <dl>
                  <div><dt>Mode</dt><dd>{plan.mode === 'wda' ? 'WebDriverAgent bridge' : 'Standard XCTest'}</dd></div>
                  <div><dt>Timeout</dt><dd>{plan.timeoutSeconds} seconds</dd></div>
                  <div><dt>Target</dt><dd>{plan.targetName ?? 'No target app'}</dd></div>
                  <div><dt>Filters</dt><dd>{plan.testsToRun.length} included · {plan.testsToSkip.length} skipped</dd></div>
                  <div className="wide"><dt>Runner</dt><dd>{plan.runnerName} · {plan.runnerBundleId}</dd></div>
                  <div className="wide"><dt>Transport intent</dt><dd>{plan.wdaBridge ? 'Local WDA bridge requested; networking remains locked' : plan.transport}</dd></div>
                </dl>
              </div>}
            </div>

            <div className="test-lab-run-note">
              <LockKeyhole size={16} />
              <div><b>Execution controls are intentionally locked</b><p>Plan validation records mode, filters, target, timeout, and WDA bridge intent only. Event output, deterministic Stop/cleanup, and WDA networking must be completed together before a test process can be started.</p></div>
            </div>
          </> : <div className="test-lab-detail-empty"><Smartphone size={29} /><b>Select an XCTest runner</b><p>Its signing and developer-service readiness will appear here.</p></div>}
        </aside>
      </div>

      <div className="test-lab-safety">
        <ShieldCheck size={16} />
        <span><b>No test process is launched by this page.</b> Refresh and plan validation only inspect current metadata and normalize configuration. Runner execution and WDA networking remain unavailable.</span>
      </div>
    </section>
  )
}
