import { dispatch, useProjectStore } from "@/lib/store/project"
import { useUiStore } from "@/lib/store/ui"
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

/** Opens the tapper; its reviewed estimate is applied as one undo step. */
export function tapTempo(): void {
  useUiStore.getState().openDialog("tempoTap")
}
