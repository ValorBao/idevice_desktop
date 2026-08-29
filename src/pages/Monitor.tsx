import { useState } from 'react'
import { Logs } from './Logs'
import { NetworkCapture } from './NetworkCapture'
import { Notifications } from './Notifications'
import { Performance } from './Performance'
import { Processes } from './Processes'

export function Monitor({ connected, desktop, udid, onError }: { connected: boolean; desktop: boolean; udid: string; onError: (message: string) => void }) {
  const [tab, setTab] = useState<'logs' | 'processes' | 'performance' | 'network' | 'notifications'>('logs')

  return (
    <section className="monitor-page">
      <div className="monitor-tabs tabs" role="tablist" aria-label="Monitor tools">
        <button role="tab" aria-selected={tab === 'logs'} className={tab === 'logs' ? 'active' : ''} onClick={() => setTab('logs')}>Logs</button>
        <button role="tab" aria-selected={tab === 'processes'} className={tab === 'processes' ? 'active' : ''} onClick={() => setTab('processes')}>Processes</button>
        <button role="tab" aria-selected={tab === 'performance'} className={tab === 'performance' ? 'active' : ''} onClick={() => setTab('performance')}>Performance</button>
        <button role="tab" aria-selected={tab === 'network'} className={tab === 'network' ? 'active' : ''} onClick={() => setTab('network')}>Network</button>
        <button role="tab" aria-selected={tab === 'notifications'} className={tab === 'notifications' ? 'active' : ''} onClick={() => setTab('notifications')}>Notifications</button>
      </div>
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
