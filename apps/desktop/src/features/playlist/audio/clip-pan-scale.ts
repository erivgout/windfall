import type { AudioClipUpdate, ClipId } from "@/bindings"

type ClipPanScaleFactor = "half" | "double"

type PanClip = {
  id: ClipId
  pan: number
}

export function scaledClipPan(pan: number, factor: ClipPanScaleFactor): number {
  return factor === "half" ? pan / 2 : Math.min(1, Math.max(-1, pan * 2))
}

export function clipPanScaleUpdates(
  clips: readonly PanClip[],
  factor: ClipPanScaleFactor
): AudioClipUpdate[] {
  return clips.flatMap(({ id, pan }) => {
    const next = scaledClipPan(pan, factor)
    return Math.abs(next - pan) < 0.001 ? [] : [{ id, patch: { pan: next } }]
  })
}
