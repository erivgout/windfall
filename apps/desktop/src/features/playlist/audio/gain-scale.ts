import type { AudioClipUpdate, ClipId } from "@/bindings"
import { MAX_GAIN } from "@/lib/units"

type GainScaleFactor = "half" | "double"

type GainClip = {
  id: ClipId
  gain: number
}

export function scaledGain(gain: number, factor: GainScaleFactor): number {
  return factor === "half" ? gain / 2 : Math.min(gain * 2, MAX_GAIN)
}

export function gainScaleUpdates(
  clips: readonly GainClip[],
  factor: GainScaleFactor
): AudioClipUpdate[] {
  return clips.flatMap(({ id, gain }) => {
    const next = scaledGain(gain, factor)
    return Math.abs(next - gain) < 0.001 ? [] : [{ id, patch: { gain: next } }]
  })
}
