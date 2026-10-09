import { TRACK_HEIGHT_PRESETS } from "./track-height-presets"

export function nextTrackHeightPreset(
  height: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(height)) return null

  let previous: number | null = null
  for (const [index, item] of TRACK_HEIGHT_PRESETS.entries()) {
    if (height === item.height) {
      return direction === "previous"
        ? previous
        : (TRACK_HEIGHT_PRESETS[index + 1]?.height ?? null)
    }
    if (height < item.height) {
      return direction === "previous" ? previous : item.height
    }
    previous = item.height
  }
  return direction === "previous" ? previous : null
}
