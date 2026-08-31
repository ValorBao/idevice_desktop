import { FolderOpen, Layers, Sliders, TerminalSquare } from 'lucide-react'
import { PhoneFrame } from './PhoneFrame'
import { type Lab } from './useLab'
import './ripper.css'

const ripperStations: { id: 'inspect' | 'files' | 'apps' | 'watch'; label: string; sub: string; icon: typeof Sliders }[] = [
  { id: 'inspect', label: 'Prosthetic Rig', sub: 'Chassis & Hardware', icon: Sliders },
  { id: 'files', label: 'Memory Shards', sub: 'AFC Sandboxes', icon: FolderOpen },
  { id: 'apps', label: 'Kernel Override', sub: 'JIT & Sideloading', icon: Layers },
  { id: 'watch', label: 'Synapse Trace', sub: 'Blackbox Logs', icon: TerminalSquare },
]

export function RipperShell({ lab }: { lab: Lab }) {
  const freeGb = (lab.device.storageTotal - lab.device.storageUsed).toFixed(0)
  return (
    <div className="ripper-shell">
      {/* Top Surgical Bar */}
      <header className="ripper-bar">
        <span className="ripper-lights" aria-hidden="true"><i /><i /><i /></span>
        <div className="ripper-title-wrap">
          <span className="ripper-brand-icon"><i /></span>
          <span className="ripper-title">RIPPER BAY</span>
          <span className="ripper-subtag">SURGICAL OVERCLOCK</span>
        </div>
        <div className="ripper-status-pill">
          <i />
          <span>NEURAL LINK · SEATED ({lab.device.conn})</span>
        </div>
      </header>

      <div className="ripper-body">
        {/* Left Docking Rail */}
        <aside className="ripper-dock">
          <button
            type="button"
            className="ripper-target-selector"
            onClick={() => lab.setMenu((v) => !v)}
            aria-label="Select Target Prosthetic"
          >
            <small>DOCKED CYBERWARE</small>
            <strong>{lab.device.name}</strong>
            <span>{lab.device.model} · iOS {lab.device.ios}</span>
          </button>

          {lab.menu && (
            <div className="ripper-device-menu">
              {lab.catalog.map((item) => (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => {
                    lab.setDeviceId(item.id)
                    lab.setMenu(false)
                    lab.notify(`${item.name} clamped & keyed`)
                  }}
                >
                  <b>{item.name}</b>
                  <small style={{ display: 'block', opacity: 0.5 }}>{item.model} · iOS {item.ios}</small>
                </button>
              ))}
            </div>
          )}

          {/* Governor & Bypass State Probes */}
          <div className="ripper-probes-panel">
            <div className="ripper-section-tag">
              <span>GOVERNOR BYPASS</span>
              <em>ACTIVE</em>
            </div>
            <div className="ripper-probe-grid">
              <div className="ripper-probe-cell" data-active="true">
                <span>PAIR KEY</span>
                <i />
              </div>
              <div className="ripper-probe-cell" data-active="true">
                <span>DDI DRIVER</span>
                <i />
              </div>
              <div className="ripper-probe-cell" data-active="true">
                <span>DEV LIMIT</span>
                <i />
              </div>
              <div className="ripper-probe-cell" data-active="true">
                <span>RSD BUS</span>
                <i />
              </div>
            </div>
          </div>

          {/* 4 Surgical Station Buttons */}
          <nav className="ripper-nav-list" aria-label="Surgical Stations">
            {ripperStations.map((item) => {
              const Icon = item.icon
              return (
                <button
                  key={item.id}
                  type="button"
                  className="ripper-nav-button"
                  data-on={lab.station === item.id || undefined}
                  onClick={() => lab.setStation(item.id)}
                >
                  <Icon size={16} />
                  <div className="ripper-nav-copy">
                    <b>{item.label}</b>
                    <small>{item.sub}</small>
                  </div>
                </button>
              )
            })}
          </nav>

          <footer className="ripper-dock-footer">
            <span>BAY 04 · STERILE LOCK</span>
            <code>{lab.device.udid.slice(0, 8)}</code>
          </footer>
        </aside>

        {/* Main Operational Stage */}
        <main className="ripper-stage">
          {lab.station === 'inspect' && (
            <div className="ripper-rig-view">
              <div className="ripper-clamp-arm top-left" />
              <div className="ripper-clamp-arm bottom-right" />

              {/* Holographic Callout Nodes */}
              <div className="ripper-hud-node left-top">
                <div className="hud-tag-box">
                  <span>FIRMWARE OS</span>
                  <strong>{lab.device.ios}</strong>
                </div>
                <div className="hud-line" />
              </div>

              <div className="ripper-hud-node right-mid">
                <div className="hud-tag-box">
                  <span>CORE TEMP / PWR</span>
                  <strong>{lab.device.battery}% CHARGE</strong>
                </div>
                <div className="hud-line" />
              </div>

              <div className="ripper-hud-node left-bot">
                <div className="hud-tag-box">
                  <span>UNALLOCATED STORAGE</span>
                  <strong>{freeGb} GB FREE</strong>
                </div>
                <div className="hud-line" />
              </div>

              <div className="ripper-rig-phone-wrap">
                <div className="ripper-laser-scan" />
                <PhoneFrame caption="Surgical Docking Clamp" />
              </div>
            </div>
          )}

          {lab.station === 'files' && (
            <div className="ripper-panel-content">
              <div className="ripper-panel-header">
                <h2>Memory Shards {lab.currentPath}</h2>
                <span>AFC SANDBOX CONTAINER</span>
              </div>
              {lab.path.length > 0 && (
                <button type="button" className="ripper-item-row" onClick={lab.goUp}>
                  <b>[PARENT]</b>
                  <strong>../ Up Directory</strong>
                  <small>ROOT</small>
                </button>
              )}
              {lab.files.map((entry) => (
                <button
                  key={entry.name}
                  type="button"
                  className="ripper-item-row"
                  onClick={() => entry.folder ? lab.openFolder(entry.name) : lab.notify(`Extracted ${entry.name}`)}
                >
                  <b>{entry.folder ? '[SECTOR]' : '[BLOCK]'}</b>
                  <strong>{entry.name}</strong>
                  <small>{entry.folder ? 'DIR' : entry.size ?? entry.date}</small>
                </button>
              ))}
            </div>
          )}

          {lab.station === 'apps' && (
            <div className="ripper-panel-content">
              <div className="ripper-panel-header">
                <h2>Kernel Override (JIT & Sideload)</h2>
                <span>GET-TASK-ALLOW ENABLED</span>
              </div>
              {lab.apps.map((app) => (
                <button
                  key={app.id}
                  type="button"
                  className="ripper-item-row"
                  onClick={() => {
                    lab.setAppId(app.id)
                    lab.notify(`Attached JIT tunnel to ${app.name}`)
                  }}
                >
                  <b>{app.system ? '[SYSTEM]' : '[INJECTED]'}</b>
                  <strong>{app.name}</strong>
                  <small>{app.bundle} · {app.version}</small>
                </button>
              ))}
              {lab.selectedApp && !lab.selectedApp.system && (
                <button
                  type="button"
                  className="ripper-item-row"
                  style={{ borderColor: 'rgba(255, 0, 85, 0.4)', marginTop: 12 }}
                  onClick={lab.uninstall}
                >
                  <b style={{ color: 'var(--neon-magenta)' }}>[PURGE]</b>
                  <strong style={{ color: 'var(--neon-magenta)' }}>Purge {lab.selectedApp.name} from Flash</strong>
                  <small>UNINSTALL</small>
                </button>
              )}
            </div>
          )}

          {lab.station === 'watch' && (
            <div className="ripper-panel-content">
              <div className="ripper-panel-header">
                <h2>Synapse Activity Stream</h2>
                <button
                  type="button"
                  style={{
                    background: 'transparent',
                    border: '1px solid var(--neon-cyan)',
                    color: 'var(--neon-cyan)',
                    padding: '4px 10px',
                    borderRadius: 4,
                    cursor: 'pointer',
                    font: 'inherit',
                  }}
                  onClick={() => lab.setPaused((v) => !v)}
                >
                  {lab.paused ? '▶ RESUME TAP' : '⏸ HOLD STREAM'}
                </button>
              </div>
              {lab.logs.slice(-16).map((line, index) => (
                <div key={`${line.time}-${index}`} className="ripper-item-row">
                  <b>{line.level}</b>
                  <strong>{line.process}</strong>
                  <small>{line.message}</small>
                </div>
              ))}
            </div>
          )}

          {lab.toast && (
            <div className="ripper-toast">
              <i />
              <span>{lab.toast}</span>
            </div>
          )}
        </main>
      </div>
    </div>
  )
}
