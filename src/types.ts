

export type WorkbenchMode = 'inspect' | 'files' | 'apps' | 'watch'
export type Page = WorkbenchMode | 'overview' | 'diagnostics' | 'files' | 'apps' | 'crashes' | 'logs' | 'developer' | 'location'
export type Connection = 'connected' | 'detected' | 'none'
export type ProbeStatus = 'active' | 'ready' | 'idle' | 'warning' | 'error' | 'disabled'
