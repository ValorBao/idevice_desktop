import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { DeveloperStatus, XCTestPreflightSnapshot, XCTestRunPlan } from '../api'

const backend = vi.hoisted(() => ({
  api: {
    xctestPreflight: vi.fn(),
    xctestPlanPrepare: vi.fn(),
    developerStatus: vi.fn(),
  },
}))

vi.mock('../api', () => ({
  ...backend,
  errorMessage: (error: unknown) => String(error),
}))

import { TestLab } from './TestLab'

const snapshot = (overrides: Partial<XCTestPreflightSnapshot> = {}): XCTestPreflightSnapshot => ({
  iosVersion: '17.4',
  transport: 'CoreDeviceProxy/RSD TestManager/DVT',
  executionSupported: true,
  limitation: null,
  runnerTotal: 2,
  targetTotal: 2,
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
      bundleId: 'com.example.BrokenRunner.xctrunner',
      name: 'BrokenRunner',
      version: '1.0',
      executable: null,
      debuggable: false,
      isWebdriverAgent: false,
      configurationReady: false,
      issues: ['Application container is missing', 'Executable must end with -Runner', 'get-task-allow is not enabled'],
    },
  ],
  targets: [
    { bundleId: 'com.example.DeviceLab', name: 'Device Lab', version: '1.4', debuggable: true },
    { bundleId: 'com.example.Storefront', name: 'Storefront', version: '3.2', debuggable: false },
  ],
  ...overrides,
})

const developerStatus: DeveloperStatus = {
  developerMode: true,
  ddiMounted: true,
  ddiImages: null,
  rsdAvailable: true,
}

const runPlan = (overrides: Partial<XCTestRunPlan> = {}): XCTestRunPlan => ({
  runnerBundleId: 'com.example.WebDriverAgentRunner.xctrunner',
  runnerName: 'WebDriverAgentRunner',
  targetBundleId: 'com.example.DeviceLab',
  targetName: 'Device Lab',
  mode: 'test',
  testsToRun: ['Suite/TestA'],
  testsToSkip: ['Suite/TestB'],
  timeoutSeconds: 900,
  wdaBridge: false,
  transport: 'CoreDeviceProxy/RSD TestManager/DVT',
  ...overrides,
})

describe('XCTest preflight workflow', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    backend.api.xctestPreflight.mockResolvedValue(snapshot())
    backend.api.xctestPlanPrepare.mockResolvedValue(runPlan())
    backend.api.developerStatus.mockResolvedValue(developerStatus)
  })

  it('loads runner metadata and developer readiness for the selected device', async () => {
    render(<TestLab desktop udid="device-1" onToast={vi.fn()} />)

    expect(await screen.findByRole('button', { name: /WebDriverAgentRunner/ })).toBeInTheDocument()
    expect(backend.api.xctestPreflight).toHaveBeenCalledWith('device-1')
    expect(backend.api.developerStatus).toHaveBeenCalledWith('device-1')
    expect(screen.getAllByText('CoreDeviceProxy/RSD TestManager/DVT')).toHaveLength(2)
    expect(screen.getByText('Preflight ready')).toBeInTheDocument()
    expect(screen.getByText('No test process is launched by this page.')).toBeInTheDocument()
  })

  it('selects an optional target without starting a process', async () => {
    const user = userEvent.setup()
    render(<TestLab desktop udid="device-1" onToast={vi.fn()} />)
    await screen.findByRole('button', { name: /WebDriverAgentRunner/ })

    await user.selectOptions(screen.getByLabelText('Target application'), 'com.example.DeviceLab')

    expect(screen.getByText('Device Lab selected')).toBeInTheDocument()
    expect(screen.getByText(/com\.example\.DeviceLab · debug entitlement allowed/)).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Start XCTest' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Run XCTest' })).not.toBeInTheDocument()
  })

  it('filters candidates by bundle ID and updates the inspected runner', async () => {
    const user = userEvent.setup()
    render(<TestLab desktop udid="device-1" onToast={vi.fn()} />)
    const search = await screen.findByLabelText('Search XCTest runners')

    await user.type(search, 'BrokenRunner.xctrunner')

    expect(screen.queryByRole('button', { name: /WebDriverAgentRunner/ })).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: /BrokenRunner/ })).toBeInTheDocument()
    expect(screen.getByText('Needs attention')).toBeInTheDocument()
  })

  it('shows malformed metadata and signing failures instead of claiming readiness', async () => {
    const user = userEvent.setup()
    render(<TestLab desktop udid="device-1" onToast={vi.fn()} />)
    await user.click(await screen.findByRole('button', { name: /BrokenRunner/ }))

    expect(screen.getByText(/Application container is missing · Executable must end with -Runner/)).toBeInTheDocument()
    expect(screen.getByText('Re-sign the runner with a development profile')).toBeInTheDocument()
    expect(screen.getByText('Needs attention')).toBeInTheDocument()
  })

  it('reports a desktop inspection failure without substituting demonstration data', async () => {
    const onToast = vi.fn()
    backend.api.xctestPreflight.mockRejectedValue(new Error('Installation Proxy unavailable'))
    render(<TestLab desktop udid="device-1" onToast={onToast} />)

    expect(await screen.findByRole('alert')).toHaveTextContent('Error: Installation Proxy unavailable')
    expect(onToast).toHaveBeenCalledWith('Error: Installation Proxy unavailable')
    expect(screen.queryByText('WebDriverAgentRunner')).not.toBeInTheDocument()
  })

  it('uses bounded demonstration candidates without calling desktop services', async () => {
    render(<TestLab desktop={false} udid="demo-device" onToast={vi.fn()} />)

    expect(await screen.findByRole('button', { name: /WebDriverAgentRunner/ })).toBeInTheDocument()
    expect(screen.getAllByText('CoreDeviceProxy/RSD TestManager/DVT · demonstration')).toHaveLength(2)
    expect(backend.api.xctestPreflight).not.toHaveBeenCalled()
    expect(backend.api.developerStatus).not.toHaveBeenCalled()
  })

  it('validates a standard plan with normalized filters against fresh device metadata', async () => {
    const user = userEvent.setup()
    render(<TestLab desktop udid="device-1" onToast={vi.fn()} />)
    await screen.findByRole('button', { name: /WebDriverAgentRunner/ })

    await user.selectOptions(screen.getByLabelText('Target application'), 'com.example.DeviceLab')
    await user.type(screen.getByLabelText('Tests to include'), ' Suite/TestA {enter}Suite/TestA')
    await user.type(screen.getByLabelText('Tests to skip'), 'Suite/TestB')
    await user.click(screen.getByRole('button', { name: 'Validate run plan' }))

    expect(backend.api.xctestPlanPrepare).toHaveBeenCalledWith({
      runnerBundleId: 'com.example.WebDriverAgentRunner.xctrunner',
      targetBundleId: 'com.example.DeviceLab',
      mode: 'test',
      testsToRun: ['Suite/TestA'],
      testsToSkip: ['Suite/TestB'],
      timeoutSeconds: 900,
    }, 'device-1')
    expect(await screen.findByText('Validated execution plan')).toBeInTheDocument()
    expect(screen.getByText('1 included · 1 skipped')).toBeInTheDocument()
  })

  it('converts WebDriverAgent mode into a bridge-only readiness plan', async () => {
    const user = userEvent.setup()
    backend.api.xctestPlanPrepare.mockResolvedValue(runPlan({
      targetBundleId: null,
      targetName: null,
      mode: 'wda',
      testsToRun: [],
      testsToSkip: [],
      timeoutSeconds: 30,
      wdaBridge: true,
    }))
    render(<TestLab desktop udid="device-1" onToast={vi.fn()} />)
    await screen.findByRole('button', { name: /WebDriverAgentRunner/ })

    await user.selectOptions(screen.getByLabelText('Target application'), 'com.example.DeviceLab')
    await user.type(screen.getByLabelText('Tests to include'), 'Suite/TestA')
    await user.selectOptions(screen.getByLabelText('Plan mode'), 'wda')

    expect(screen.getByLabelText('Target application')).toBeDisabled()
    expect(screen.getByLabelText('Tests to include')).toBeDisabled()
    expect(screen.getByLabelText('Tests to include')).toHaveValue('')
    expect(screen.getByLabelText('Plan timeout')).toHaveValue('30')
    await user.click(screen.getByRole('button', { name: 'Validate run plan' }))

    expect(backend.api.xctestPlanPrepare).toHaveBeenCalledWith({
      runnerBundleId: 'com.example.WebDriverAgentRunner.xctrunner',
      targetBundleId: null,
      mode: 'wda',
      testsToRun: [],
      testsToSkip: [],
      timeoutSeconds: 30,
    }, 'device-1')
    expect(await screen.findByText('Local WDA bridge requested; networking remains locked')).toBeInTheDocument()
  })

  it('invalidates a validated preview when the draft changes', async () => {
    const user = userEvent.setup()
    render(<TestLab desktop udid="device-1" onToast={vi.fn()} />)
    await screen.findByRole('button', { name: /WebDriverAgentRunner/ })
    await user.click(screen.getByRole('button', { name: 'Validate run plan' }))
    expect(await screen.findByText('Validated execution plan')).toBeInTheDocument()

    await user.selectOptions(screen.getByLabelText('Plan timeout'), '1800')

    expect(screen.queryByText('Validated execution plan')).not.toBeInTheDocument()
  })

  it('keeps plan validation disabled while the selected preflight is incomplete', async () => {
    backend.api.developerStatus.mockResolvedValue({ ...developerStatus, ddiMounted: false })
    render(<TestLab desktop udid="device-1" onToast={vi.fn()} />)
    await screen.findByRole('button', { name: /WebDriverAgentRunner/ })

    expect(screen.getByRole('button', { name: 'Validate run plan' })).toBeDisabled()
    expect(backend.api.xctestPlanPrepare).not.toHaveBeenCalled()
  })

  it('previews a demonstration plan without calling the desktop backend', async () => {
    const user = userEvent.setup()
    render(<TestLab desktop={false} udid="demo-device" onToast={vi.fn()} />)
    await screen.findByRole('button', { name: /WebDriverAgentRunner/ })

    await user.click(screen.getByRole('button', { name: 'Validate run plan' }))

    expect(await screen.findByText('Validated execution plan')).toBeInTheDocument()
    expect(backend.api.xctestPlanPrepare).not.toHaveBeenCalled()
  })
})
