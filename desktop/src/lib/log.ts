import { api } from './api';
import { LogLevel } from '../types';

/**
 * The diagnostics log (FRONTEND.md §6.4). The core owns the file and writes the
 * git side itself; this is how the UI adds what only it knows: the errors it
 * showed the user, and the ones it recovered from without showing anything.
 *
 * Never throws: a failure to log must not turn into a second error.
 */
export function log(level: LogLevel, message: string): void {
  api.logMessage(level, message).catch(() => {});
}

/** The readable part of whatever a `catch` received. */
export function describeError(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

/** Log an error the UI recovered from; also printed to the devtools console. */
export function logError(context: string, err: unknown): void {
  const message = `${context}: ${describeError(err)}`;
  console.error(message);
  log('error', message);
}
