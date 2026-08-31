import { PhoneFrame } from './PhoneFrame'
import { stations, type Lab } from './useLab'
import './ion.css'

export function IonShell({ lab }: { lab: Lab }) {
  return (
    <div className="ion-shell">
      <header className="ion-bar">
        <span className="ion-lights" aria-hidden="true"><i /><i /><i /></span>
        <b>Ion</b>
        <em>Field stable · {lab.device.conn}</em>
      </header>
      <aside className="ion-side">
        <button type="button" className="ion-device" onClick={() => lab.setMenu((value) => !value)}>
          <strong>{lab.device.name}</strong>
          <small>{lab.device.model} · iOS {lab.device.ios}</small>
        </button>
        {lab.menu && (
          <div className="ion-menu">
            {lab.catalog.map((item) => (
              <button key={item.id} type="button" onClick={() => { lab.setDeviceId(item.id); lab.setMenu(false); lab.notify(`${item.name} locked`) }}>
                {item.name}
              </button>
            ))}
          </div>
        )}
        <div className="ion-ticks">
          <span data-on>PAIR</span>
          <span data-on>DDI</span>
          <span data-on>DEV</span>
          <span data-on>RSD</span>
        </div>
        <nav className="ion-modes" aria-label="Stations">
          {stations.map((item) => (
            <button key={item.id} type="button" data-on={lab.station === item.id || undefined} onClick={() => lab.setStation(item.id)}>
              <span>{item.label}</span>
            </button>
          ))}
        </nav>
      </aside>
      <main className="ion-main">
        {lab.station === 'inspect' && (
          <div className="ion-cage">
            <i className="ring r1" />
            <i className="ring r2" />
            <i className="ring r3" />
            <div className="ion-callout c1"><i /><div><span>OS</span><b>{lab.device.ios}</b></div></div>
            <div className="ion-callout c2"><i /><div><span>POWER</span><b>{lab.device.battery}%</b></div></div>
            <div className="ion-callout c3"><i /><div><span>FREE</span><b>{lab.device.storageTotal - lab.device.storageUsed} GB</b></div></div>
            <PhoneFrame caption="Containment lock" />
          </div>
        )}
        {lab.station === 'files' && (
          <div className="ion-board">
            <h2>Payload {lab.currentPath}</h2>
            {lab.path.length > 0 && <button type="button" className="ion-line" onClick={lab.goUp}><span>UP</span><b>Parent</b><small>dir</small></button>}
            {lab.files.map((entry) => (
              <button key={entry.name} type="button" className="ion-line" onClick={() => entry.folder ? lab.openFolder(entry.name) : lab.notify(entry.name)}>
                <span>{entry.folder ? 'DIR' : 'FILE'}</span><b>{entry.name}</b><small>{entry.size ?? entry.date}</small>
              </button>
            ))}
          </div>
        )}
        {lab.station === 'apps' && (
          <div className="ion-board">
            <h2>Override</h2>
            {lab.apps.map((app) => (
              <button key={app.id} type="button" className="ion-line" onClick={() => lab.setAppId(app.id)}>
                <span>{app.system ? 'SYS' : 'USR'}</span><b>{app.name}</b><small>{app.version}</small>
              </button>
            ))}
            {lab.selectedApp && !lab.selectedApp.system && (
              <button type="button" className="ion-line" onClick={lab.uninstall}><span>RM</span><b>Uninstall {lab.selectedApp.name}</b><small>confirm</small></button>
            )}
          </div>
        )}
        {lab.station === 'watch' && (
          <div className="ion-board">
            <h2>Wire</h2>
            <button type="button" className="ion-line" onClick={() => lab.setPaused((value) => !value)}>
              <span>{lab.paused ? 'RUN' : 'HOLD'}</span><b>{lab.paused ? 'Resume' : 'Pause'}</b><small>{lab.logs.length}</small>
            </button>
            {lab.logs.slice(-14).map((line, index) => (
              <div key={`${line.time}-${index}`} className="ion-line">
                <span>{line.level}</span><b>{line.process}</b><small>{line.message}</small>
              </div>
            ))}
          </div>
        )}
        {lab.toast && <div className="ion-toast">{lab.toast}</div>}
      </main>
    </div>
  )
}
