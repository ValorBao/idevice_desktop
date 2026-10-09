import { useCallback, useEffect, useRef, useState } from 'react'
import { devices, type Device } from '../data'
import { api, errorMessage, events } from '../api'
import type { Connection } from '../types'
import { summaryToDevice } from './device'

export type DeviceSession = {
  /** Every device discovery currently reports, usable or not. */
  catalog: Device[]
  /** The selected device, falling back to the first known one. */
  device: Device
  connection: Connection
  connected: boolean
  /**
  * Re-reads the catalog and reconciles the selection with it. In browser demo
  * mode there is nothing to read, so it reveals the demo device instead.
  */
  refresh: () => Promise<void>
  select: (id: string) => Promise<void>
  disconnect: () => Promise<void>
  /**
   * Drops the session view without touching the backend selection, for a user
   * who dismisses onboarding rather than disconnecting a connected device.
   */
  dismiss: () => void
  /** Pairs the selected device. Resolves true once the session is usable. */
  pair: () => Promise<boolean>
}

/**
 * Owns the device session: discovery, selection, connection state, pairing, and
 * the background developer-disk-image mount.
 *
 * The shell used to hold this inline, where four refs coordinating a
 * serialized refresh sat next to the layout. Both the serialization and the
 * lifecycle guards below exist for reasons a reader cannot infer from the
 * markup, so they belong somewhere they can be read and tested on their own.
 *
 * `onSessionLost` fires when no usable device remains, so the shell can leave a
 * view that needs one.
 */
export function useDeviceSession({
  desktop,
  onError,
  onSessionLost,
}: {
  desktop: boolean
  onError: (message: string) => void
  onSessionLost: () => void
}): DeviceSession {
  const [catalog, setCatalog] = useState<Device[]>(desktop ? [] : devices)
  const [deviceId, setDeviceId] = useState(desktop ? '' : 'd1')
  const [connection, setConnection] = useState<Connection>(desktop ? 'none' : 'connected')

  const deviceIdRef = useRef(deviceId)
  const mountedRef = useRef(true)
  const lifecycleRef = useRef(0)
  const refreshRunningRef = useRef(false)
  const refreshPendingRef = useRef(false)
  // Held in refs so a changed callback never restarts device monitoring.
  const onErrorRef = useRef(onError)
  const onSessionLostRef = useRef(onSessionLost)

  useEffect(() => { onErrorRef.current = onError }, [onError])
  useEffect(() => { onSessionLostRef.current = onSessionLost }, [onSessionLost])
  useEffect(() => { deviceIdRef.current = deviceId }, [deviceId])
  useEffect(() => {
    mountedRef.current = true
    lifecycleRef.current += 1
    return () => {
      mountedRef.current = false
      lifecycleRef.current += 1
    }
  }, [])

  const device = catalog.find((item) => item.id === deviceId) ?? catalog[0] ?? devices[0]
  const connected = connection === 'connected'

  const refresh = useCallback(async () => {
    if (!desktop) {
      setConnection('detected')
      return
    }
    if (!mountedRef.current) return
    refreshPendingRef.current = true
    if (refreshRunningRef.current) return

    refreshRunningRef.current = true
    const lifecycle = lifecycleRef.current
    const lifecycleIsCurrent = () => mountedRef.current && lifecycleRef.current === lifecycle
    try {
      while (refreshPendingRef.current && lifecycleIsCurrent()) {
        refreshPendingRef.current = false
        try {
          const found = await api.deviceList()
          if (!lifecycleIsCurrent()) return
          // A device event arrived while this snapshot was loading. Do not let
          // the older catalog take over the session; fetch the latest one.
          if (refreshPendingRef.current) continue

          setCatalog(found.map(summaryToDevice))
          if (!found.length) {
            setDeviceId('')
            setConnection('none')
            onSessionLostRef.current()
            await api.deviceDisconnect().catch((error) => {
              if (lifecycleIsCurrent()) onErrorRef.current(errorMessage(error))
            })
            continue
          }

          const current = found.find((item) => item.id === deviceIdRef.current)
          const target = current ?? found.find((item) => item.paired && item.connectable) ?? found[0]
          if (target.paired && target.connectable) {
            if (target.id !== deviceIdRef.current) await api.deviceSelect(target.id)
            if (!lifecycleIsCurrent()) return
            if (refreshPendingRef.current) continue
            setDeviceId(target.id)
            setConnection('connected')
          } else {
            setDeviceId(target.id)
            setConnection('detected')
            onSessionLostRef.current()
            await api.deviceDisconnect().catch((error) => {
              if (lifecycleIsCurrent()) onErrorRef.current(errorMessage(error))
            })
          }
        } catch (error) {
          if (!lifecycleIsCurrent()) return
          if (refreshPendingRef.current) continue
          setConnection('none')
          onErrorRef.current(errorMessage(error))
        }
      }
    } finally {
      refreshRunningRef.current = false
      // StrictMode can remount while the first mount still has a listing in
      // flight. The remounted effect marks a refresh pending; start it after
      // the obsolete runner releases the serialization lock.
      if (refreshPendingRef.current && mountedRef.current) void refresh()
    }
  }, [desktop])

  useEffect(() => {
    if (!desktop) return
    let disposed = false
    let unlisten: (() => void) | undefined
    void events.deviceChanged(() => { if (!disposed) void refresh() })
      .then((stop) => {
        if (disposed) stop()
        else unlisten = stop
      })
      .catch((error) => { if (!disposed) onErrorRef.current(errorMessage(error)) })
    void api.deviceMonitorStart()
      .then(() => { if (!disposed) return refresh() })
      .catch((error) => { if (!disposed) onErrorRef.current(errorMessage(error)) })
    return () => {
      disposed = true
      unlisten?.()
      void api.deviceMonitorStop()
    }
  }, [desktop, refresh])

  // Mount the developer disk image as soon as a device is usable, so no page has
  // to offer a mount control. This runs in the background: pages stay
  // interactive while it works, and only a failure is reported. A stale
  // selection drops its result rather than reporting against the new device.
  useEffect(() => {
    if (!desktop || !connected || !device?.udid) return
    let disposed = false
    void api.ddiEnsure(device.udid).catch((error) => {
      if (!disposed) onErrorRef.current(errorMessage(error))
    })
    return () => { disposed = true }
  }, [desktop, connected, device?.udid])

  const select = useCallback(async (id: string) => {
    setDeviceId(id)
    if (!desktop) {
      setConnection('connected')
      return
    }
    try {
      await api.deviceSelect(id)
      setConnection('connected')
    } catch (error) {
      setConnection('detected')
      onErrorRef.current(errorMessage(error))
    }
  }, [desktop])

  const disconnect = useCallback(async () => {
    if (desktop) await api.deviceDisconnect().catch((error) => onErrorRef.current(errorMessage(error)))
    setConnection('none')
    onSessionLostRef.current()
  }, [desktop])

  const dismiss = useCallback(() => setConnection('none'), [])

  const pair = useCallback(async () => {
    try {
      if (desktop) {
        const paired = await api.devicePair(device.udid)
        await api.deviceSelect(paired.udid)
        await refresh()
      }
      setConnection('connected')
      return true
    } catch (error) {
      onErrorRef.current(errorMessage(error))
      return false
    }
  }, [desktop, device.udid, refresh])

  return { catalog, device, connection, connected, refresh, select, disconnect, dismiss, pair }
}
