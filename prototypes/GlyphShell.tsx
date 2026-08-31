import { PhoneFrame } from './PhoneFrame'
import { stations, type Lab } from './useLab'
import './glyph.css'

export function GlyphShell({ lab }: { lab: Lab }) {
  return (
    <div className="glyph-shell">
      <header className="glyph-bar">
        <span className="glyph-lights" aria-hidden="true"><i /><i /><i /></span>
        <span className="glyph-word">Glyph</span>
        <span className="glyph-pips" aria-hidden="true">{Array.from({ length: 12 }, (_, index) => <i key={index} />)}</span>
        <span className="glyph-lock">USB LOCK</span>
      </header>
      <div className="glyph-body">
        <div className="glyph-col">
          <button type="button" className="glyph-name" onClick={() => lab.setMenu((value) => !value)}>{lab.device.name}</button>
          {lab.menu && (
            <div className="glyph-menu">
              {lab.catalog.map((item) => (
                <button key={item.id} type="button" onClick={() => { lab.setDeviceId(item.id); lab.setMenu(false); lab.notify(`${item.name} lit`) }}>
                  {item.name}
                </button>
              ))}
            </div>
          )}
          <nav className="glyph-modes" aria-label="Stations">
            {stations.map((item, itemIndex) => (
              <button key={item.id} type="button" data-on={lab.station === item.id || undefined} onClick={() => lab.setStation(item.id)}>
                {item.label.toUpperCase()}<small>0{itemIndex + 1}</small>
              </button>
            ))}
          </nav>
          {lab.station !== 'inspect' && <div style={{ height: 18 }} />}
          {lab.station === 'files' && (
            <div className="glyph-list">
              {lab.path.length > 0 && <button type="button" onClick={lab.goUp}>UP<small>DIR</small></button>}
              {lab.files.map((entry) => (
                <button key={entry.name} type="button" onClick={() => entry.folder ? lab.openFolder(entry.name) : lab.notify(entry.name)}>
                  {entry.name}<small>{entry.folder ? 'DIR' : 'FILE'}</small>
                </button>
              ))}
            </div>
          )}
          {lab.station === 'apps' && (
            <div className="glyph-list">
              {lab.apps.map((app) => (
                <button key={app.id} type="button" data-on={lab.appId === app.id || undefined} onClick={() => lab.setAppId(app.id)}>
                  {app.name}<small>{app.version}</small>
                </button>
              ))}
              {lab.selectedApp && !lab.selectedApp.system && (
                <button type="button" onClick={lab.uninstall}>Remove {lab.selectedApp.name}<small>RM</small></button>
              )}
            </div>
          )}
          {lab.station === 'watch' && (
            <div className="glyph-list">
              <button type="button" onClick={() => lab.setPaused((value) => !value)}>{lab.paused ? 'Resume' : 'Pause'}<small>LOG</small></button>
              {lab.logs.slice(-8).map((line, index) => (
                <button key={`${line.time}-${index}`} type="button">{line.process}<small>{line.level}</small></button>
              ))}
            </div>
          )}
        </div>
        <div className="glyph-mid">
          <PhoneFrame caption={lab.device.udid.slice(0, 12)} />
        </div>
        <div className="glyph-digits">
          <div><span>OS</span><b>{lab.device.ios}</b></div>
          <div><span>PWR</span><b>{lab.device.battery}</b></div>
          <div><span>FREE</span><b>{lab.device.storageTotal - lab.device.storageUsed}</b></div>
        </div>
      </div>
      <footer className="glyph-status">
        <span>Pair · DDI · Dev · RSD</span>
        <span>{lab.device.model} · {lab.device.chip}</span>
      </footer>
      {lab.toast && <div className="glyph-toast">{lab.toast}</div>}
    </div>
  )
}
