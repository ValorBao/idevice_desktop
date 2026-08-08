import { useState } from 'react'
import { Logs } from './Logs'
import { Processes } from './Processes'

export function Monitor({ connected, desktop, udid, onError }: { connected: boolean; desktop: boolean; udid: string; onError: (message: string) => void }) {
  const [tab, setTab] = useState<'logs' | 'processes'>('logs')

  return (
    <section className="monitor-page">
      <div className="monitor-tabs tabs" role="tablist" aria-label="Monitor tools">
        <button role="tab" aria-selected={tab === 'logs'} className={tab === 'logs' ? 'active' : ''} onClick={() => setTab('logs')}>Logs</button>
        <button role="tab" aria-selected={tab === 'processes'} className={tab === 'processes' ? 'active' : ''} onClick={() => setTab('processes')}>Processes</button>
      </div>
      <div className="monitor-content">
        {tab === 'logs'
          ? <Logs connected={connected} desktop={desktop} udid={udid} onError={onError} />
          : <Processes desktop={desktop} udid={udid} onToast={onError} />}
      </div>
    </section>
  )
}
