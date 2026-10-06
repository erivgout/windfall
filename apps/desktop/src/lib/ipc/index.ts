import type { Backend } from "./backend"
import { createMockBackend } from "./mock"
import { createTauriBackend } from "./tauri"

export { errorMessage } from "./backend"
export type { Backend, Unsubscribe } from "./backend"

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window
}

let current: Backend | null = null

function resolve(): Backend {
  current ??= isTauri() ? createTauriBackend() : createMockBackend()
  return current
}

/** Swaps the backend. Tests use this to start from a fresh mock. */
export function setBackend(next: Backend) {
  current = next
}

/**
 * The one backend the UI talks to: Tauri inside the app, the mock in a plain
 * browser. It is created on first use, and every call is forwarded to
 * whichever backend is current.
 */
export const backend: Backend = new Proxy({} as Backend, {
  get: (_target, key) => Reflect.get(resolve(), key),
})
