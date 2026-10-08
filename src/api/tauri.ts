/** Thin wrapper over Tauri's `invoke`, with typed error helpers. */

import { invoke as tauriInvoke } from "@tauri-apps/api/core";

/** The structured error shape our Rust commands reject with. */
export interface AppError {
  kind: string;
  message: string;
  hint?: string | null;
}

export function isAppError(e: unknown): e is AppError {
  return (
    typeof e === "object" &&
    e !== null &&
    "kind" in e &&
    "message" in e &&
    typeof (e as AppError).message === "string"
  );
}

export function errorMessage(e: unknown): string {
  if (isAppError(e)) return e.message;
  if (e instanceof Error) return e.message;
  return String(e);
}

export function errorHint(e: unknown): string | undefined {
  if (isAppError(e) && e.hint) return e.hint;
  return undefined;
}

export function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return tauriInvoke<T>(cmd, args);
}
