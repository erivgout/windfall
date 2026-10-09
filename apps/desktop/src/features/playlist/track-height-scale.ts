import { MAX_ROW_HEIGHT, MIN_ROW_HEIGHT } from "./layout"

export function scaledTrackHeight(
  height: number,
  factor: "half" | "double"
): number {
  if (height === 0) return 0
  return factor === "half"
    ? Math.max(MIN_ROW_HEIGHT, Math.floor(height / 2))
    : Math.min(MAX_ROW_HEIGHT, height * 2)
}

export function nextTrackHeightScale(
  height: number,
  factor: "half" | "double"
): number | null {
  const next = scaledTrackHeight(height, factor)
  return next === height ? null : next
}
