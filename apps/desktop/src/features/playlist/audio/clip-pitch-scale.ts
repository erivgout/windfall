import type { AudioClipUpdate, ClipId } from "@/bindings"
import { MAX_TUNE_SEMITONES } from "@/lib/units"

type ClipPitchScaleFactor = "half" | "double"

type PitchClip = {
  id: ClipId
  pitch: number
}

export function scaledClipPitch(
  pitch: number,
  factor: ClipPitchScaleFactor
): number {
  return factor === "half"
    ? pitch / 2
    : Math.min(MAX_TUNE_SEMITONES, Math.max(-MAX_TUNE_SEMITONES, pitch * 2))
}

export function clipPitchScaleUpdates(
  clips: readonly PitchClip[],
  factor: ClipPitchScaleFactor
): AudioClipUpdate[] {
  return clips.flatMap(({ id, pitch }) => {
    const next = scaledClipPitch(pitch, factor)
    return Math.abs(next - pitch) < 0.001 ? [] : [{ id, patch: { pitch: next } }]
  })
}
