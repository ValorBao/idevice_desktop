import type { AppInfo } from '../data'

export function AppIcon({ app, large }: { app: AppInfo; large?: boolean }) {
  return <span className={`app-icon ${large ? 'large' : ''}`} style={{ background: app.color }}>{app.icon ? <img src={app.icon} alt="" /> : app.name[0]}</span>
}
