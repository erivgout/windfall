import { LOOP_CROSSFADE_PRESETS } from "./loop-crossfade-presets"

export function nextLoopCrossfadePreset(
  crossfade: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(crossfade)) return null

  const index = LOOP_CROSSFADE_PRESETS.findIndex(
    (item) => Math.abs(crossfade - item.crossfade) < 0.001
  )
  if (index !== -1) {
    const nextIndex = index + (direction === "previous" ? -1 : 1)
    return LOOP_CROSSFADE_PRESETS[nextIndex]?.crossfade ?? null
  }

  let previous: number | null = null
  for (const item of LOOP_CROSSFADE_PRESETS) {
    if (item.crossfade - crossfade >= 0.001) {
      return direction === "previous" ? previous : item.crossfade
    }
    if (crossfade - item.crossfade >= 0.001) previous = item.crossfade
  }
  return direction === "previous" ? previous : null
}
