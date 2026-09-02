import type { ReactNode } from 'react'
import type { LucideIcon } from 'lucide-react'

export type WorkbenchTab<Id extends string> = {
  id: Id
  label: string
  icon: LucideIcon
  ariaLabel?: string
  extra?: ReactNode
}

export function WorkbenchTabs<Id extends string>({
  label,
  value,
  tabs,
  onChange,
}: {
  label: string
  value: Id
  tabs: Array<WorkbenchTab<Id>>
  onChange: (id: Id) => void
}) {
  return (
    <div className="workbench-subbar">
      <div className="workbench-tabs" role="tablist" aria-label={label}>
        {tabs.map((tab) => {
          const Icon = tab.icon
          return (
            <button
              key={tab.id}
              role="tab"
              aria-label={tab.ariaLabel}
              aria-selected={value === tab.id}
              className={value === tab.id ? 'active' : ''}
              onClick={() => onChange(tab.id)}
            >
              <Icon size={14} />
              <span>{tab.label}</span>
              {tab.extra}
            </button>
          )
        })}
      </div>
    </div>
  )
}
