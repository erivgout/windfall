import { useSyncExternalStore } from "react"

/*
 * Ids start over in every project: track 3 of the song just opened is not
 * track 3 of the one before. Whatever a panel keeps by id outside the
 * project itself (held peaks, a scroll position per lane, which effects
 * are folded away) has to go when another project takes the place of the
 * open one. This is the one signal for that.
 */

type Listener = () => void

const listeners = new Set<Listener>()
let generation = 0

/**
 * Calls `listener` each time another project has taken the place of the
 * open one: after New and Open, when the stores already hold the new
 * project. Returns a function that stops.
 *
 *     onProjectReplaced(() => heldPeaks.clear())
 */
export function onProjectReplaced(listener: Listener): () => void {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

/** Tells everyone the project was replaced. The stores call this, on `project:loaded`. */
export function announceProjectReplaced() {
  generation += 1
  for (const listener of [...listeners]) listener()
}

/** Captures the current document identity for asynchronous UI work. */
export function getProjectGeneration(): number {
  return generation
}

/**
 * Counts the projects that have been loaded. Use it as a `key` to start a
 * panel over with nothing left from the project before.
 */
export function useProjectGeneration(): number {
  return useSyncExternalStore(onProjectReplaced, getProjectGeneration)
}
