import { screen } from "@testing-library/react"

import type { BrowserEntry } from "@/bindings"
import { setBackend, type Backend } from "@/lib/ipc"
import { startTestApp } from "@/test/harness"

import { registerBrowserActions } from "./actions"
import { resetPreview } from "./preview"
import { resetBrowserStore } from "./store"

/**
 * Starts the test app with the browser's actions in place.
 * `override` swaps backend calls, for folders that are huge, slow or broken.
 */
export async function startBrowserTest(
  override: (mock: Backend) => Partial<Backend> = () => ({})
) {
  const app = await startTestApp()
  const backend: Backend = { ...app.backend, ...override(app.backend) }
  setBackend(backend)
  resetBrowserStore()
  resetPreview()
  const unregister = registerBrowserActions()
  return {
    backend,
    stop() {
      unregister()
      app.stop()
    },
  }
}

/** A promise settled from outside, to answer backend calls in any order. */
export function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (error: Error) => void
  const promise = new Promise<T>((onResolve, onReject) => {
    resolve = onResolve
    reject = onReject
  })
  return { promise, resolve, reject }
}

export function audio(folder: string, name: string): BrowserEntry {
  return { name, path: `${folder}/${name}`, kind: "audio" }
}

export const tree = () => screen.getByRole("tree")
export const item = (name: string) => screen.getByRole("treeitem", { name })
export const findItem = (name: string) =>
  screen.findByRole("treeitem", { name })
export const queryItem = (name: string) =>
  screen.queryByRole("treeitem", { name })
export const itemNames = () =>
  screen.getAllByRole("treeitem").map((row) => row.textContent)
