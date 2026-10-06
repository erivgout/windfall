import { useEffect, useSyncExternalStore } from "react"

import type { SampleAsset, SampleInfo } from "@/bindings"
import { backend, errorMessage } from "@/lib/ipc"

export type SampleInfoState =
  | { status: "loading" }
  | { status: "ready"; info: SampleInfo }
  | { status: "error"; message: string }

/*
 * Waveforms are fetched once per sample and kept. An id always means the
 * same file inside one project, but ids start over in the next project, so
 * the file is part of the key.
 */
const cache = new Map<string, SampleInfoState>()
const listeners = new Set<() => void>()

function keyOf(sample: SampleAsset): string {
  return `${sample.id}|${sample.path.kind}|${sample.path.path}`
}

function store(key: string, state: SampleInfoState) {
  cache.set(key, state)
  for (const listener of [...listeners]) listener()
}

function load(sample: SampleAsset) {
  const key = keyOf(sample)
  if (cache.has(key)) return
  store(key, { status: "loading" })
  backend.sampleInfoById(sample.id).then(
    (info) => store(key, { status: "ready", info }),
    (error: unknown) =>
      store(key, { status: "error", message: errorMessage(error) })
  )
}

function subscribe(listener: () => void) {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

/** Forgets every waveform. Tests call this between backends. */
export function clearSampleInfo() {
  cache.clear()
}

/** Facts and waveform of a sample in the project, loaded on first use. */
export function useSampleInfo(
  sample: SampleAsset | undefined
): SampleInfoState | null {
  const key = sample ? keyOf(sample) : null
  const state = useSyncExternalStore(subscribe, () =>
    key === null ? null : (cache.get(key) ?? null)
  )
  useEffect(() => {
    if (sample) load(sample)
  }, [sample])
  return state
}
