import type { ReactNode } from 'react'

/**
 * One selectable view inside a workbench.
 *
 * `label` is the visible text and, unless `ariaLabel` overrides it, the
 * accessible name. `title` and `detail` are the page header copy shown while
 * this view is active, which keeps the header wording next to the tab that
 * produces it.
 */
export type TabItem<T extends string> = {
  id: T
  label: string
  ariaLabel?: string
  icon?: ReactNode
  /** Rendered after the label, such as the Monitor live indicator. */
  badge?: ReactNode
  title?: string
  detail?: string
}

/**
 * The tab bar every workbench shares.
 *
 * The four workbenches and the Monitor instrument panel each carried their own
 * copy of this markup, so an accessibility or styling fix had to be repeated
 * five times to avoid leaving one panel behind.
 */
export function WorkbenchTabs<T extends string>({
  items,
  active,
  onSelect,
  ariaLabel,
  className = 'workbench-tabs',
  bare = false,
}: {
  items: readonly TabItem<T>[]
  active: T
  onSelect: (id: T) => void
  ariaLabel: string
  /** Defaults to the workbench tab styling; Monitor uses its own. */
  className?: string
  /** Omits the sub-bar wrapper, for panels that place the bar themselves. */
  bare?: boolean
}) {
  const bar = (
    <div className={className} role="tablist" aria-label={ariaLabel}>
      {items.map((item) => (
        <button
          key={item.id}
          role="tab"
          aria-label={item.ariaLabel}
          aria-selected={active === item.id}
          className={active === item.id ? 'active' : ''}
          onClick={() => onSelect(item.id)}
        >
          {item.icon}
          <span>{item.label}</span>
          {item.badge}
        </button>
      ))}
    </div>
  )

  return bare ? bar : <div className="workbench-subbar">{bar}</div>
}

/** The header copy for the active view, when that view declares any. */
export function tabCopy<T extends string>(
  items: readonly TabItem<T>[],
  active: T,
): [string, string] | null {
  const item = items.find((entry) => entry.id === active)
  return item?.title ? [item.title, item.detail ?? ''] : null
}
