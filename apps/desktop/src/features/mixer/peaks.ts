import type { TrackId } from "@/bindings"
import {
  onProjectReplaced,
  subscribeRealtime,
  useProjectStore,
} from "@/lib/store"

/*
 * The loudest level each track has reached since it was last reset, as
 * linear gain. One listener on the realtime feed keeps all of them, so a
 * track that clips while its strip is scrolled out of view still shows it
 * when the strip comes back. Readouts follow a track by subscribing here and
 * writing to the DOM; nothing goes through React state.
 */

type Listener = (peak: number) => void

/** A held peak above this is a clip: the track went over 0 dB. */
export const CLIP_GAIN = 1

const held = new Map<TrackId, number>()
const listeners = new Map<TrackId, Set<Listener>>()
let watchers = 0
let stopFeed: (() => void) | null = null

function tell(id: TrackId, peak: number) {
  for (const listener of listeners.get(id) ?? []) listener(peak)
}

function follow(meters: readonly number[]) {
  const tracks = useProjectStore.getState().project.mixer.tracks
  for (let index = 0; index < tracks.length; index += 1) {
    const left = meters[index * 2] ?? 0
    const right = meters[index * 2 + 1] ?? 0
    const level = left > right ? left : right
    const id = tracks[index].id
    if (level > (held.get(id) ?? 0)) {
      held.set(id, level)
      tell(id, level)
    }
  }
}

/**
 * Starts holding peaks. Returns a function that stops; the held values are
 * dropped when the last watcher stops.
 */
export function watchPeaks(): () => void {
  watchers += 1
  stopFeed ??= subscribeRealtime((frame) => follow(frame.meters))
  return () => {
    watchers -= 1
    if (watchers > 0) return
    stopFeed?.()
    stopFeed = null
    resetAllPeaks()
  }
}

export function peakOf(id: TrackId): number {
  return held.get(id) ?? 0
}

export function resetPeak(id: TrackId) {
  if (!held.delete(id)) return
  tell(id, 0)
}

export function resetAllPeaks() {
  const ids = [...held.keys()]
  held.clear()
  for (const id of ids) tell(id, 0)
}

// Track ids repeat from project to project, so a peak held for track 3 of
// the last song would show on track 3 of this one.
onProjectReplaced(resetAllPeaks)

/** Calls `listener` with the held peak now and whenever it changes. */
export function subscribePeak(id: TrackId, listener: Listener): () => void {
  let set = listeners.get(id)
  if (!set) {
    set = new Set()
    listeners.set(id, set)
  }
  set.add(listener)
  listener(peakOf(id))
  return () => {
    set.delete(listener)
    if (set.size === 0) listeners.delete(id)
  }
}
