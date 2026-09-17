import { useState } from 'react'
import { WorkbenchTabs, type TabItem } from '../components/WorkbenchTabs'
import { Logs } from './Logs'
import { NetworkCapture } from './NetworkCapture'
import { Notifications } from './Notifications'
import { Performance } from './Performance'
import { Processes } from './Processes'

type MonitorTool = 'logs' | 'processes' | 'performance' | 'network' | 'notifications'

const monitorTabs: readonly TabItem<MonitorTool>[] = [
  { id: 'logs', label: 'Logs' },
  { id: 'processes', label: 'Processes' },
  { id: 'performance', label: 'Performance' },
  { id: 'network', label: 'Network' },
  { id: 'notifications', label: 'Notifications' },
]

export function Monitor({ connected, desktop, udid, onError }: { connected: boolean; desktop: boolean; udid: string; onError: (message: string) => void }) {
  const [tab, setTab] = useState<MonitorTool>('logs')

  return (
    <section className="monitor-page">
      <WorkbenchTabs
        items={monitorTabs}
        active={tab}
        onSelect={setTab}
        ariaLabel="Monitor tools"
        className="monitor-tabs tabs"
        bare
      />
      <div className="monitor-content">
        {tab === 'logs' && <Logs connected={connected} desktop={desktop} udid={udid} onError={onError} />}
        {tab === 'processes' && <Processes desktop={desktop} udid={udid} onToast={onError} />}
        {tab === 'performance' && <Performance desktop={desktop} udid={udid} onToast={onError} />}
        {tab === 'network' && <NetworkCapture desktop={desktop} udid={udid} onToast={onError} />}
        {tab === 'notifications' && <Notifications connected={connected} desktop={desktop} udid={udid} onToast={onError} />}
      </div>
    </section>
  )
}
