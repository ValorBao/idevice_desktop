import { FolderOpen, Layers, Sliders, TerminalSquare } from 'lucide-react'
import { PhoneFrame } from './PhoneFrame'
import { stations, type Lab } from './useLab'
import './void.css'

const railIcon = {
  inspect: Sliders,
  files: FolderOpen,
  apps: Layers,
  watch: TerminalSquare,
}

export function VoidShell({ lab }: { lab: Lab }) {
  const free = `${(lab.device.storageTotal - lab.device.storageUsed).toFixed(0)} GB`
  return (
    <div className="void-shell">
      <header className="void-bar">
        <span className="void-lights" aria-hidden="true"><i /><i /><i /></span>
        <b>Void</b>
        <button type="button" className="press" onClick={() => lab.setMenu((value) => !value)} style={{ color: 'inherit', background: 'none', border: 0, cursor: 'pointer' }}>
          {lab.device.name}
        </button>
        <em><i />Locked · {lab.device.conn}</em>
      </header>
      {lab.menu && (
        <div className="void-menu">
          {lab.catalog.map((item) => (
            <button key={item.id} type="button" onClick={() => { lab.setDeviceId(item.id); lab.setMenu(false); lab.notify(`${item.name} seated`) }}>
              {item.name}
              <small style={{ display: 'block', opacity: 0.45 }}>{item.model} · iOS {item.ios}</small>
            </button>
          ))}
        </div>
      )}
      <div className="void-body">
        <nav className="void-rail" aria-label="Stations">
          {stations.map((item) => {
            const Icon = railIcon[item.id]
            return (
              <button key={item.id} type="button" data-on={lab.station === item.id || undefined} aria-label={item.label} title={item.label} onClick={() => lab.setStation(item.id)}>
                <Icon size={18} />
              </button>
            )
          })}
        </nav>
        <main className="void-stage">
          {lab.station === 'inspect' && (
            <>
              <div className="void-copy">
                <small>Connected object</small>
                <h1>{lab.device.name}</h1>
                <p>{lab.device.model} · {lab.device.udid}</p>
              </div>
              <div className="void-readout">
                <div><span>OS</span><strong>{lab.device.ios.split('.')[0]}</strong><small>{lab.device.ios}</small></div>
                <div><span>Power</span><strong>{lab.device.battery}</strong><small>{lab.device.batteryHealth}% health</small></div>
                <div><span>Free</span><strong>{free.split(' ')[0]}</strong><small>of {lab.device.storageTotal} GB</small></div>
              </div>
              <div className="void-hero">
                <PhoneFrame caption="Drag the vehicle" />
              </div>
            </>
          )}
          {lab.station === 'files' && (
            <div className="void-panel">
              <h2>{lab.currentPath}</h2>
              {lab.path.length > 0 && <button type="button" className="void-row" onClick={lab.goUp}><b>Up</b><small>parent</small></button>}
              {lab.files.map((entry) => (
                <button key={entry.name} type="button" className="void-row" onClick={() => entry.folder ? lab.openFolder(entry.name) : lab.notify(entry.name)}>
                  <b>{entry.name}</b><small>{entry.folder ? 'folder' : entry.size}</small>
                </button>
              ))}
            </div>
          )}
          {lab.station === 'apps' && (
            <div className="void-panel">
              <h2>Applications</h2>
              {lab.apps.map((app) => (
                <button key={app.id} type="button" className="void-row" onClick={() => lab.setAppId(app.id)}>
                  <b>{app.name}</b><small>{app.bundle}</small>
                </button>
              ))}
              {lab.selectedApp && !lab.selectedApp.system && (
                <button type="button" className="void-row" onClick={lab.uninstall}><b>Remove {lab.selectedApp.name}</b><small>uninstall</small></button>
              )}
            </div>
          )}
          {lab.station === 'watch' && (
            <div className="void-panel">
              <h2>Blackbox</h2>
              <button type="button" className="void-row" onClick={() => lab.setPaused((value) => !value)}>
                <b>{lab.paused ? 'Resume stream' : 'Pause stream'}</b><small>{lab.logs.length} lines</small>
              </button>
              {lab.logs.slice(-12).map((line, index) => (
                <div key={`${line.time}-${index}`} className="void-log"><time>{line.time}</time><span>{line.process} · {line.message}</span></div>
              ))}
            </div>
          )}
          <div className="void-dock">
            {stations.map((item) => (
              <button key={item.id} type="button" data-on={lab.station === item.id || undefined} onClick={() => lab.setStation(item.id)}>{item.label}</button>
            ))}
          </div>
        </main>
      </div>
      {lab.toast && <div className="void-toast">{lab.toast}</div>}
    </div>
  )
}
