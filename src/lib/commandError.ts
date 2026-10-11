import type { CommandError } from '../api'

/** Product kinds the UI is allowed to branch on. Other kinds toast their message. */
export const COMMAND_KIND = {
  cancelled: 'cancelled',
  passcode: 'passcode',
  ddiMissing: 'ddi-missing',
  developerMode: 'developer-mode',
  networkDiscovery: 'network_discovery',
} as const

/** Kinds that are not an error toast. Cancellation is how a task ends, not a failure. */
const SILENT_KINDS: ReadonlySet<string> = new Set([COMMAND_KIND.cancelled])

const isRecord = (error: unknown): error is Record<string, unknown> =>
  typeof error === 'object' && error !== null

/**
 * Turns a thrown invoke payload, `Error`, or string into the CommandError the
 * backend already serialized. This is the only parser; pages do not rebuild it.
 */
export function commandError(error: unknown): CommandError {
  if (isRecord(error)) {
    if (typeof error.kind !== 'string' && 'error' in error && error.error !== error) {
      return commandError(error.error)
    }
    const message = 'message' in error ? String(error.message) : JSON.stringify(error)
    return {
      kind: typeof error.kind === 'string' ? error.kind : 'unknown',
      message,
      retryable: error.retryable === true,
    }
  }
  return { kind: 'unknown', message: String(error), retryable: false }
}

export function isCommandKind(error: unknown, kind: string): boolean {
  return commandError(error).kind === kind
}

/**
 * Default UI action for a rejected command. Cancelled is silent; every other
 * kind toasts its message. Returns the parsed error so a catch that also resets
 * page state does not parse twice.
 */
export function reportCommand(error: unknown, onToast: (message: string) => void): CommandError {
  const parsed = commandError(error)
  if (!SILENT_KINDS.has(parsed.kind)) onToast(parsed.message)
  return parsed
}

export const errorMessage = (error: unknown) => commandError(error).message
