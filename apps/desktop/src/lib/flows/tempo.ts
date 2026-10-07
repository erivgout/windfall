import { dispatch, useProjectStore } from "@/lib/store/project"
import { normalizeTempo } from "@/lib/time"
import { DEFAULT_TEMPO_BPM, MAX_TEMPO_BPM, MIN_TEMPO_BPM } from "@/lib/units"

export function currentTempo(): number {
  return useProjectStore.getState().project.settings.tempoBpm
}

/** Sets the tempo as one undo step. A tempo out of range is brought into it. */
export async function setTempo(tempoBpm: number): Promise<void> {
  const next = normalizeTempo(tempoBpm)
  if (next === currentTempo()) return
  await dispatch({ type: "updateSettings", patch: { tempoBpm: next } })
}

/** Whether the tempo times `factor` is still a tempo there can be. */
export function canScaleTempo(tempoBpm: number, factor: number): boolean {
  const next = tempoBpm * factor
  return next >= MIN_TEMPO_BPM && next <= MAX_TEMPO_BPM
}

export function scaleTempo(factor: number): Promise<void> {
  return setTempo(currentTempo() * factor)
}

export function resetTempo(): Promise<void> {
  return setTempo(DEFAULT_TEMPO_BPM)
}

/** A pause this long starts the count over. */
const TAP_RESET_MS = 2000
/** The taps the tempo is averaged over, at most. */
const MAX_TAPS = 8

const taps: number[] = []

/**
 * Takes one tap of "tap tempo". From the second tap on it returns the
 * tempo the taps so far beat, averaged over the last few; the first tap,
 * or the first after a pause, returns null.
 */
export function tapInterval(now: number): number | null {
  if (taps.length > 0 && now - taps[taps.length - 1] > TAP_RESET_MS) {
    taps.length = 0
  }
  taps.push(now)
  if (taps.length > MAX_TAPS) taps.shift()
  if (taps.length < 2) return null
  const beat = (taps[taps.length - 1] - taps[0]) / (taps.length - 1)
  return beat > 0 ? normalizeTempo(60_000 / beat) : null
}

/** Forgets the taps so far. Tests call this between cases. */
export function forgetTaps() {
  taps.length = 0
}

/** One tap: sets the tempo to the beat of the taps, once there are two. */
export async function tapTempo(): Promise<void> {
  const tempo = tapInterval(performance.now())
  if (tempo !== null) await setTempo(tempo)
}
