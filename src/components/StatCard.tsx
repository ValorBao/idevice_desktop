import type { ReactNode } from 'react'

export function StatCard({ label, wide, children }: { label: string; wide?: boolean; children: ReactNode }) {
  return <div className={`stat-card card ${wide ? 'wide' : ''}`}><span className="stat-label">{label}</span>{children}</div>
}
